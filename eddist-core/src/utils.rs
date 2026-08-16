use std::fmt::Write;
use std::sync::OnceLock;

use chrono::{DateTime, Datelike, TimeDelta, Weekday};

static IS_PROD: OnceLock<bool> = OnceLock::new();
static IS_RES_PUB_ENABLED: OnceLock<bool> = OnceLock::new();
static IS_THREAD_PUB_ENABLED: OnceLock<bool> = OnceLock::new();
static IS_AUTH_TOKEN_PUB_ENABLED: OnceLock<bool> = OnceLock::new();

pub fn is_prod() -> bool {
    *IS_PROD.get_or_init(|| {
        matches!(
            std::env::var("RUST_ENV").as_deref(),
            Ok("prod" | "production")
        )
    })
}

pub fn is_user_registration_enabled() -> bool {
    matches!(
        std::env::var("ENABLE_USER_REGISTRATION").as_deref(),
        Ok("true")
    )
}

pub fn is_res_pub_enabled() -> bool {
    *IS_RES_PUB_ENABLED
        .get_or_init(|| !matches!(std::env::var("ENABLE_RES_PUB").as_deref(), Ok("false")))
}

pub fn is_thread_pub_enabled() -> bool {
    *IS_THREAD_PUB_ENABLED
        .get_or_init(|| !matches!(std::env::var("ENABLE_THREAD_PUB").as_deref(), Ok("false")))
}

pub fn is_auth_token_pub_enabled() -> bool {
    *IS_AUTH_TOKEN_PUB_ENABLED.get_or_init(|| {
        !matches!(
            std::env::var("ENABLE_AUTH_TOKEN_PUB").as_deref(),
            Ok("false")
        )
    })
}

pub fn is_authed_token_backup_enabled() -> bool {
    matches!(
        std::env::var("ENABLE_AUTHED_TOKEN_BACKUP").as_deref(),
        Ok("true")
    )
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

pub fn to_hex(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_hex_pads_and_lowercases_each_byte() {
        assert_eq!(to_hex([]), "");
        assert_eq!(to_hex([0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
        assert_eq!(to_hex(b"eddist"), "656464697374");
        assert_eq!(to_hex(vec![0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }
}
