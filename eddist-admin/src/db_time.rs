use chrono::{DateTime, Timelike, Utc};

/// The schema stores timestamps with millisecond precision (`DATETIME(3)`).
///
/// Truncating before binding keeps MySQL from rounding a value at the column
/// boundary and makes the same application value portable to PostgreSQL.
pub(crate) fn truncate_to_millis(value: DateTime<Utc>) -> DateTime<Utc> {
    value
        .with_nanosecond(value.nanosecond() / 1_000_000 * 1_000_000)
        .expect("millisecond precision is a valid nanosecond value")
}

pub(crate) fn now() -> DateTime<Utc> {
    truncate_to_millis(Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_to_schema_precision() {
        let value = DateTime::parse_from_rfc3339("2026-09-12T12:34:56.123987Z")
            .unwrap()
            .to_utc();

        assert_eq!(
            truncate_to_millis(value),
            DateTime::parse_from_rfc3339("2026-09-12T12:34:56.123Z")
                .unwrap()
                .to_utc()
        );
    }
}
