use askama::Template;

use crate::theme::Theme;

pub const DEFAULT_FONT_FAMILY: &str =
    "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Helvetica, Arial, sans-serif";

#[derive(Debug, Clone)]
pub struct StatRow {
    pub label: String,
    pub value: String,
    pub dy: i32,
}

#[derive(Debug, Clone)]
pub struct LangSegment {
    pub name: String,
    pub color: String,
    pub percent_text: String,
    pub dasharray: String,
    pub dashoffset: String,
    pub legend_dy: i32,
}

#[derive(Template)]
#[template(path = "svg/stats_card.svg")]
pub struct StatsCardTemplate {
    pub theme: Theme,

    pub width: i32,
    pub height: i32,
    pub radius: i32,
    pub pad_x: i32,

    pub title: String,
    pub aria_label: String,
    pub font_family: String,
    pub title_y: i32,
    pub title_size: i32,

    pub stats_rows: Vec<StatRow>,
    pub left_x: i32,
    pub left_y: i32,
    pub stat_value_x: i32,
    pub stat_label_size: i32,
    pub stat_value_size: i32,
    pub stat_label_opacity: f32,

    pub top_languages_title: String,
    pub section_title_size: i32,
    pub pie_group_x: i32,
    pub pie_group_y: i32,
    pub pie_title_y: i32,

    pub pie_cx: i32,
    pub pie_cy: i32,
    pub pie_r: f64,
    pub pie_stroke: f64,
    pub pie_base_stroke: String,

    pub lang_segments: Vec<LangSegment>,
    pub show_pie: bool,
}

pub fn render_summary_card(
    theme: Theme,
    title: String,
    stats_rows: Vec<StatRow>,
    segments: Vec<LangSegment>,
    aria_label: String,
) -> Result<String, askama::Error> {
    let template = StatsCardTemplate {
        theme,
        width: 400,
        height: 160,
        radius: 8,
        pad_x: 18,
        title,
        aria_label,
        font_family: DEFAULT_FONT_FAMILY.to_string(),
        title_y: 28,
        title_size: 16,
        stats_rows,
        left_x: 18,
        left_y: 60,
        stat_value_x: 140,
        stat_label_size: 12,
        stat_value_size: 12,
        stat_label_opacity: 0.7,
        top_languages_title: "Top Languages".to_string(),
        section_title_size: 12,
        pie_group_x: 230,
        pie_group_y: 40,
        pie_title_y: 12,
        pie_cx: 70,
        pie_cy: 70,
        pie_r: 40.0,
        pie_stroke: 12.0,
        pie_base_stroke: "#e1e4e8".to_string(),
        lang_segments: segments.clone(),
        show_pie: !segments.is_empty(),
    };

    template.render()
}

pub fn render_error_card(theme: Theme, message: String) -> Result<String, askama::Error> {
    render_summary_card(
        theme,
        "Error".to_string(),
        vec![StatRow {
            label: "Message".to_string(),
            value: message,
            dy: 0,
        }],
        Vec::new(),
        "Error".to_string(),
    )
}

pub fn build_lang_segments(
    langs: &[(String, String, f64)],
    r: f64,
    legend_line_height: i32,
) -> Vec<LangSegment> {
    let c = 2.0 * std::f64::consts::PI * r;

    let mut acc = 0.0;
    let mut out = Vec::new();

    for (i, (name, color, p)) in langs.iter().enumerate() {
        let p = p.clamp(0.0, 1.0);

        let dasharray = format!("{:.4} {:.4}", c * p, c);
        let dashoffset = format!("{:.4}", -(c * acc));

        out.push(LangSegment {
            name: name.clone(),
            color: color.clone(),
            percent_text: format!("{:.1}%", p * 100.0),
            dasharray,
            dashoffset,
            legend_dy: (i as i32) * legend_line_height,
        });

        acc += p;
    }

    out
}
