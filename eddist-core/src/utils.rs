use chrono::{DateTime, Datelike, TimeDelta, Weekday};

pub fn is_prod() -> bool {
    matches!(
        std::env::var("RUST_ENV").as_deref(),
        Ok("prod" | "production")
    )
}

pub fn is_user_registration_enabled() -> bool {
    matches!(
        std::env::var("ENABLE_USER_REGISTRATION").as_deref(),
        Ok("true")
    )
}

pub fn is_res_pub_enabled() -> bool {
    !matches!(std::env::var("ENABLE_RES_PUB").as_deref(), Ok("false"))
}

pub fn is_thread_pub_enabled() -> bool {
    !matches!(std::env::var("ENABLE_THREAD_PUB").as_deref(), Ok("false"))
}

pub fn to_ja_datetime(datetime: DateTime<chrono::Utc>) -> String {
    let datetime = datetime.checked_add_signed(TimeDelta::hours(9)).unwrap();
    let weekday = datetime.weekday();
    datetime
        .format("%Y/%m/%d({weekday}) %H:%M:%S.%3f")
        .to_string()
        .replace("{weekday}", convert_weekday_to_ja(weekday))
}

pub fn convert_weekday_to_ja(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "月",
        Weekday::Tue => "火",
        Weekday::Wed => "水",
        Weekday::Thu => "木",
        Weekday::Fri => "金",
        Weekday::Sat => "土",
        Weekday::Sun => "日",
    }
}

/// Slugify a string for use in HTML attributes and form field names.
/// Converts to lowercase, replaces non-alphanumeric chars with hyphens,
/// collapses consecutive hyphens, and trims leading/trailing hyphens.
pub fn slugify(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
        } else if !result.ends_with('-') {
            result.push('-');
        }
    }
    result.trim_matches('-').to_string()
}
