mod github;
mod render;
mod theme;

use tracing_subscriber::{
    fmt::{format::Pretty, time::UtcTime},
    prelude::*,
};
use tracing_web::{MakeConsoleWriter, performance_layer};
use worker::{Cache, Context, Cors, Env, Method, Request, Response, RouteContext, Router, event};

const CACHE_CONTROL: &str = "public, s-maxage=86400, stale-while-revalidate=3600";
const ERROR_CACHE_CONTROL: &str = "no-store";
const KV_NAMESPACE: &str = "STATWORKS_KV";
const KV_TTL_SECS: u64 = 6 * 60 * 60;

#[event(start)]
fn start() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        // 日本時間に設定
        .with_timer(UtcTime::rfc_3339())
        .with_writer(MakeConsoleWriter);
    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> worker::Result<Response> {
    console_error_panic_hook::set_once();

    let cors = Cors::default()
        .with_origins(["*"])
        .with_methods([Method::Get, Method::Post, Method::Options])
        .with_allowed_headers(["Content-Type", "Authorization"]);

    let resp = Router::new()
        .get("/health", |_, _| Response::ok("statworks"))
        .get_async("/summary", |req, ctx| async move {
            handle_summary(req, ctx).await
        })
        .get_async("/api/summary", |req, ctx| async move {
            handle_summary(req, ctx).await
        })
        .run(req, env)
        .await?
        .with_cors(&cors)?;

    Ok(resp)
}

async fn handle_summary(req: Request, ctx: RouteContext<()>) -> worker::Result<Response> {
    let url = req.url()?;
    let theme = theme::theme_from_query(
        query_param(&url, "background-color"),
        query_param(&url, "text-color"),
    );

    if let Some(cached) = cache_get(&req).await? {
        return Ok(cached);
    }

    let user = match query_param(&url, "user") {
        Some(value) => value,
        None => {
            let svg = render_or_error(render::render_error_card(
                theme,
                "user is required".to_string(),
            ))?;
            return svg_response_no_cache(svg);
        }
    };

    let cache_key = format!(
        "summary:{user}:{bg}:{text}",
        user = user,
        bg = theme.background_color,
        text = theme.text_color
    );

    if let Some(svg) = kv_get(&ctx, &cache_key).await? {
        return respond_with_cache(req, svg).await;
    }

    let summary = match github::fetch_github_summary(&user).await {
        Ok(value) => value,
        Err(err) => {
            let svg = render_or_error(render::render_error_card(theme, err.to_string()))?;
            return svg_response_no_cache(svg);
        }
    };

    let stats_rows = vec![
        render::StatRow {
            label: "Stars".to_string(),
            value: summary.stars_total.to_string(),
            dy: 0,
        },
        render::StatRow {
            label: "Commits (year)".to_string(),
            value: summary.commits.to_string(),
            dy: 20,
        },
        render::StatRow {
            label: "Pull Requests".to_string(),
            value: summary.pull_requests.to_string(),
            dy: 40,
        },
        render::StatRow {
            label: "Issues".to_string(),
            value: summary.issues.to_string(),
            dy: 60,
        },
        render::StatRow {
            label: "Languages".to_string(),
            value: summary.languages.len().to_string(),
            dy: 80,
        },
    ];

    let segments = build_segments_from_summary(&summary, 5, 40.0, 20);
    let title = format!("{user} GitHub Stats");
    let aria_label = format!("GitHub stats for {user}");
    let svg = render_or_error(render::render_summary_card(
        theme, title, stats_rows, segments, aria_label,
    ))?;

    kv_put(&ctx, &cache_key, &svg).await?;
    respond_with_cache(req, svg).await
}

fn build_segments_from_summary(
    summary: &github::GitHubSummary,
    top_n: usize,
    radius: f64,
    legend_line_height: i32,
) -> Vec<render::LangSegment> {
    let total: u64 = summary.languages.iter().map(|(_, size)| *size).sum();
    if total == 0 {
        return Vec::new();
    }

    let palette = [
        "#DEA584", "#E34C26", "#3572A5", "#F1E05A", "#00ADD8", "#9B59B6", "#16A085",
    ];

    let langs: Vec<(String, String, f64)> = summary
        .languages
        .iter()
        .take(top_n)
        .enumerate()
        .map(|(idx, (name, size))| {
            let ratio = *size as f64 / total as f64;
            let color = palette.get(idx).copied().unwrap_or("#95A5A6").to_string();
            (name.clone(), color, ratio)
        })
        .collect();

    render::build_lang_segments(&langs, radius, legend_line_height)
}

fn query_param(url: &worker::Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.to_string())
}

async fn cache_get(req: &Request) -> worker::Result<Option<Response>> {
    let cache = Cache::default();
    cache.get(req, true).await
}

async fn respond_with_cache(req: Request, svg: String) -> worker::Result<Response> {
    let response = svg_response(svg.clone())?;
    let cache_response = svg_response(svg)?;
    let cache = Cache::default();
    cache.put(&req, cache_response).await?;
    Ok(response)
}

fn svg_response(svg: String) -> worker::Result<Response> {
    let mut resp = Response::ok(svg)?;
    let headers = resp.headers_mut();
    headers.set("Content-Type", "image/svg+xml")?;
    headers.set("Cache-Control", CACHE_CONTROL)?;
    Ok(resp)
}

fn svg_response_no_cache(svg: String) -> worker::Result<Response> {
    let mut resp = Response::ok(svg)?;
    let headers = resp.headers_mut();
    headers.set("Content-Type", "image/svg+xml")?;
    headers.set("Cache-Control", ERROR_CACHE_CONTROL)?;
    Ok(resp)
}

fn render_or_error(result: Result<String, askama::Error>) -> worker::Result<String> {
    result.map_err(|err| worker::Error::RustError(err.to_string()))
}

async fn kv_get(ctx: &RouteContext<()>, key: &str) -> worker::Result<Option<String>> {
    let kv = match ctx.env.kv(KV_NAMESPACE) {
        Ok(kv) => kv,
        Err(_) => return Ok(None),
    };

    kv.get(key)
        .text()
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))
}

async fn kv_put(ctx: &RouteContext<()>, key: &str, svg: &str) -> worker::Result<()> {
    let kv = match ctx.env.kv(KV_NAMESPACE) {
        Ok(kv) => kv,
        Err(_) => return Ok(()),
    };

    kv.put(key, svg)?
        .expiration_ttl(KV_TTL_SECS)
        .execute()
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))
}
