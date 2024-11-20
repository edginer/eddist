use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
}

#[derive(Debug, Clone)]
pub struct BoardInfo {
    pub id: Uuid,
    pub local_rules: String,
    pub base_thread_creation_span_sec: i32,
    pub base_response_creation_span_sec: i32,
    pub max_thread_name_byte_length: i32,
    pub max_author_name_byte_length: i32,
    pub max_email_byte_length: i32,
    pub max_response_body_byte_length: i32,
    pub max_response_body_lines: i32,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub read_only: bool,
}

pub fn validate_board_key(board_key: &str) -> anyhow::Result<()> {
    (board_key
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && board_key.len() < 64)
        .then_some(())
        .ok_or(anyhow::anyhow!("invalid board key"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_board_key() {
        let cases = [
            ("a", Ok(())),
            ("1", Ok(())),
            ("a1", Ok(())),
            ("a1b2c3d4e5f6g7h8i9j0", Ok(())),
            ("A", Err(())),
            ("あ", Err(())),
            ("A/A", Err(())),
            (
                "123456789012345678901234567890123456789012345678901234567890123",
                Ok(()),
            ),
            (
                "1234567890123456789012345678901234567890123456789012345678901234",
                Err(()),
            ),
        ];

        for (input, expected) in cases.iter() {
            let result = validate_board_key(input);
            assert_eq!(expected.is_ok(), result.is_ok());
        }
    }
}
