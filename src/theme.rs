#[derive(Debug, Clone)]
pub struct Theme {
    pub background_color: String,
    pub text_color: String,
}

const DEFAULT_BACKGROUND: &str = "#F6F1D1";
const DEFAULT_TEXT: &str = "#0B2027";

pub fn theme_from_query(background: Option<String>, text: Option<String>) -> Theme {
    let background_color = background
        .and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .unwrap_or_else(|| DEFAULT_BACKGROUND.to_string());

    let text_color = text
        .and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .unwrap_or_else(|| DEFAULT_TEXT.to_string());

    Theme {
        background_color,
        text_color,
    }
}
