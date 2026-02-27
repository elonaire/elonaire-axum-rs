use scraper::Html;

/// Extract visible text from sanitized HTML
pub fn extract_text_from_html(html: &str) -> String {
    let document = Html::parse_fragment(html);
    document.root_element().text().collect::<Vec<_>>().join(" ")
}

pub fn calculate_read_time_minutes(html: &str) -> u32 {
    let text = extract_text_from_html(html);

    let word_count = text
        .split_whitespace()
        .filter(|w| !w.trim().is_empty())
        .count();

    let words_per_minute = 200;

    let minutes = (word_count as f32 / words_per_minute as f32).ceil();

    minutes.max(1.0) as u32
}
