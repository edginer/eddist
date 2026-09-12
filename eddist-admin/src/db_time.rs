use chrono::{NaiveDateTime, Timelike, Utc};

/// The schema stores timestamps with millisecond precision (`DATETIME(3)`).
///
/// Truncating before binding keeps MySQL from rounding a value at the column
/// boundary and makes the same application value portable to PostgreSQL.
pub(crate) fn truncate_to_millis(value: NaiveDateTime) -> NaiveDateTime {
    value
        .with_nanosecond(value.nanosecond() / 1_000_000 * 1_000_000)
        .expect("millisecond precision is a valid nanosecond value")
}

pub(crate) fn now() -> NaiveDateTime {
    truncate_to_millis(Utc::now().naive_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_to_schema_precision() {
        let value =
            NaiveDateTime::parse_from_str("2026-09-12 12:34:56.123987", "%Y-%m-%d %H:%M:%S%.f")
                .unwrap();

        assert_eq!(
            truncate_to_millis(value),
            NaiveDateTime::parse_from_str("2026-09-12 12:34:56.123", "%Y-%m-%d %H:%M:%S%.f",)
                .unwrap()
        );
    }
}
