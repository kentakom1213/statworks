use std::collections::HashMap;

use serde::Deserialize;
use tracing::{debug, info, warn};
use worker::{Error, Fetch, Method, Request, RequestInit};

const GITHUB_API_BASE: &str = "https://api.github.com";
const MAX_REPOS: usize = 400;
const MAX_EVENT_PAGES: usize = 3;

#[derive(Debug, Clone)]
pub struct GitHubSummary {
    pub languages: Vec<(String, u64)>,
    pub stars_total: u64,
    pub commits: u64,
    pub pull_requests: u64,
    pub issues: u64,
}

#[derive(Debug, Deserialize)]
struct RepoItem {
    fork: bool,
    archived: bool,
    stargazers_count: u64,
    owner: RepoOwner,
    name: String,
}

#[derive(Debug, Deserialize)]
struct RepoOwner {
    login: String,
}

#[derive(Debug, Deserialize)]
struct EventItem {
    #[serde(rename = "type")]
    event_type: String,
    payload: Option<EventPayload>,
}

#[derive(Debug, Deserialize)]
struct EventPayload {
    size: Option<u64>,
}

pub async fn fetch_github_summary(login: &str) -> worker::Result<GitHubSummary> {
    info!("fetch_github_summary start: login={}", login);
    let mut languages: HashMap<String, u64> = HashMap::new();
    let mut stars_total = 0u64;
    let mut repo_count = 0usize;

    let mut page = 1usize;
    loop {
        let url = format!(
            "{base}/users/{login}/repos?per_page=100&page={page}&sort=updated",
            base = GITHUB_API_BASE,
            login = login,
            page = page
        );
        let repos: Vec<RepoItem> = github_get_json(&url).await?;
        debug!("repos fetched: login={} page={} count={}", login, page, repos.len());
        if repos.is_empty() {
            break;
        }

        for repo in repos {
            if repo.fork || repo.archived {
                continue;
            }
            repo_count += 1;
            if repo_count > MAX_REPOS {
                break;
            }

            stars_total += repo.stargazers_count;

            let lang_url = format!(
                "{base}/repos/{owner}/{repo}/languages",
                base = GITHUB_API_BASE,
                owner = repo.owner.login,
                repo = repo.name
            );
            let lang_map: HashMap<String, u64> = github_get_json(&lang_url).await?;
            for (name, size) in lang_map {
                if size == 0 {
                    continue;
                }
                *languages.entry(name).or_insert(0) += size;
            }
        }

        if repo_count >= MAX_REPOS {
            break;
        }

        page += 1;
    }

    let (commits, pull_requests, issues) = fetch_activity_counts(login).await?;

    let mut langs: Vec<(String, u64)> = languages.into_iter().collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1));

    info!(
        "fetch_github_summary done: login={} repos={} languages={}",
        login,
        repo_count,
        langs.len()
    );
    Ok(GitHubSummary {
        languages: langs,
        stars_total,
        commits,
        pull_requests,
        issues,
    })
}

async fn fetch_activity_counts(login: &str) -> worker::Result<(u64, u64, u64)> {
    info!("fetch_activity_counts start: login={}", login);
    let mut commits = 0u64;
    let mut pull_requests = 0u64;
    let mut issues = 0u64;
    let mut page = 1usize;

    while page <= MAX_EVENT_PAGES {
        let url = format!(
            "{base}/users/{login}/events/public?per_page=100&page={page}",
            base = GITHUB_API_BASE,
            login = login,
            page = page
        );
        let events: Vec<EventItem> = github_get_json(&url).await?;
        debug!(
            "events fetched: login={} page={} count={}",
            login,
            page,
            events.len()
        );
        if events.is_empty() {
            break;
        }

        for event in events {
            match event.event_type.as_str() {
                "PushEvent" => {
                    if let Some(payload) = event.payload {
                        if let Some(size) = payload.size {
                            commits += size;
                        }
                    }
                }
                "PullRequestEvent" => {
                    pull_requests += 1;
                }
                "IssuesEvent" => {
                    issues += 1;
                }
                _ => {}
            }
        }

        page += 1;
    }

    info!(
        "fetch_activity_counts done: login={} commits={} prs={} issues={}",
        login,
        commits,
        pull_requests,
        issues
    );
    Ok((commits, pull_requests, issues))
}

async fn github_get_json<T: for<'de> Deserialize<'de>>(url: &str) -> worker::Result<T> {
    debug!("github_get_json: url={}", url);
    let mut init = RequestInit::new();
    init.with_method(Method::Get);

    let request = Request::new_with_init(url, &init)?;
    let headers = request.headers();
    headers.set("User-Agent", "statworks")?;
    headers.set("Accept", "application/vnd.github+json")?;

    let mut response = Fetch::Request(request).send().await?;
    let status = response.status_code();
    if !(200..300).contains(&status) {
        let body = response.text().await.unwrap_or_default();
        warn!("GitHub API error: url={} status={} body={}", url, status, body);
        return Err(Error::RustError(format!(
            "GitHub API error {status}: {body}"
        )));
    }

    response.json::<T>().await
}
