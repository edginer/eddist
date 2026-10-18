use eddist_core::domain::{board::BoardInfo, client_info::ClientInfo, tinker::Tinker};

use crate::{
    domain::{
        res::{AuthorIdInitialized, Res},
        res_core::ResCore,
        utils::count_anchors,
    },
    error::{BbsCgiError, ContentEmptyParamType, ContentLengthExceededParamType},
};

const MAX_CHARS_PER_LINE: usize = 250;
/// Replies below a tinker internal_level of 2 (and without a tinker) may post at most
/// this many anchors, and no image URLs at all.
const MAX_ANCHORS_BELOW_LV2: usize = 3;
/// The reply interval below public level 2 (and without a tinker).
const RES_SPAN_BELOW_LV2_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostingKind {
    Res,
    Thread,
}

/// Checks body, name and mail sizes and line limits; for a new thread the raw
/// title (`Some`) is checked as well.
pub fn validate_content_length(
    res: &ResCore<'_>,
    title: Option<&str>,
    board_info: &BoardInfo,
) -> Result<(), BbsCgiError> {
    if res.body.len() > board_info.max_response_body_byte_length as usize {
        return Err(BbsCgiError::ContentLengthExceeded(
            ContentLengthExceededParamType::Body,
        ));
    }
    if res.from.len() > board_info.max_author_name_byte_length as usize {
        return Err(BbsCgiError::ContentLengthExceeded(
            ContentLengthExceededParamType::Name,
        ));
    }
    if res.mail.len() > board_info.max_email_byte_length as usize {
        return Err(BbsCgiError::ContentLengthExceeded(
            ContentLengthExceededParamType::Mail,
        ));
    }

    let lines = res.body.lines().collect::<Vec<_>>();
    let l_count = lines.len();
    if l_count > board_info.max_response_body_lines as usize {
        return Err(BbsCgiError::ContentLengthExceeded(
            ContentLengthExceededParamType::BodyLines,
        ));
    }

    if lines
        .iter()
        .any(|line| line.chars().count() > MAX_CHARS_PER_LINE)
    {
        return Err(BbsCgiError::ContentLengthExceeded(
            ContentLengthExceededParamType::BodyLineLength,
        ));
    }

    if let Some(title) = title {
        if title.is_empty() {
            return Err(BbsCgiError::ContentEmpty(ContentEmptyParamType::ThreadName));
        }

        if title.len() > board_info.max_thread_name_byte_length as usize {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::ThreadName,
            ));
        }

        // Restrict length of character (not byte) only for thread name, using max_thread_name_byte_length / 2
        if title.chars().count() > board_info.max_thread_name_byte_length as usize / 2 {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::ThreadName,
            ));
        }
    }

    Ok(())
}

/// Checks the tinker-based posting intervals; thread creation without a tinker is
/// rejected when the board restricts it.
pub fn validate_client_info(
    client_info: &ClientInfo,
    board_info: &BoardInfo,
    kind: PostingKind,
    restrict_thread_creation_on_no_tinker: bool,
) -> Result<(), BbsCgiError> {
    let is_thread = kind == PostingKind::Thread;

    if let Some(tinker) = &client_info.tinker {
        // Response creation span multiplier based on tinker internal_level
        let span_multiplier = match tinker.internal_level() {
            0 | 1 => 3,
            2 => 2,
            _ => 1, // internal_level >= 3
        };
        let res_span_sec = (board_info.base_response_creation_span_sec * span_multiplier) as u64;

        let elapsed =
            (chrono::Utc::now().timestamp() as u64).saturating_sub(tinker.last_wrote_at());
        if elapsed < res_span_sec {
            return Err(BbsCgiError::TooManyCreatingRes {
                tinker_level: tinker.internal_level(),
                span_sec: res_span_sec as i32,
            });
        }

        if is_thread && let Some(last_created_thread_at) = tinker.last_created_thread_at() {
            let elapsed =
                (chrono::Utc::now().timestamp() as u64).saturating_sub(last_created_thread_at);
            // Thread creation span limit (in seconds) based on tinker internal_level
            // Base values are doubled for the final limit
            let base_span = match tinker.internal_level() {
                0 => 60 * 60, // 1 hour
                1 => 60 * 30, // 30 min
                2 => 60 * 15, // 15 min
                3 => 60 * 8,  // 8 min
                4 => 60 * 4,  // 4 min
                _ => 60 * 2,  // 2 min (internal_level >= 5)
            };
            let span_limit = base_span * 2;

            if elapsed < span_limit {
                return Err(BbsCgiError::TooManyCreatingThread {
                    tinker_level: tinker.internal_level(),
                    span_sec: span_limit as i32,
                });
            }
        }

        Ok(())
    } else {
        if is_thread && restrict_thread_creation_on_no_tinker {
            return Err(BbsCgiError::TmpCanNotCreateThread);
        }
        Ok(())
    }
}

/// Reply only: image URLs are rejected below a tinker internal_level of 2
/// (and without a tinker).
pub fn validate_res_images(
    res: &mut Res<AuthorIdInitialized>,
    tinker: Option<&Tinker>,
) -> Result<(), BbsCgiError> {
    if media_restricted_below_lv2(tinker) && !res.get_all_images().is_empty() {
        return Err(BbsCgiError::ImageUrlBelowLv2);
    }
    Ok(())
}

/// Reply only: more than [`MAX_ANCHORS_BELOW_LV2`] anchors are rejected below a
/// tinker internal_level of 2 (and without a tinker).
pub fn validate_res_anchors(body: &str, tinker: Option<&Tinker>) -> Result<(), BbsCgiError> {
    if media_restricted_below_lv2(tinker) && count_anchors(body) > MAX_ANCHORS_BELOW_LV2 {
        return Err(BbsCgiError::NgWordDetected);
    }
    Ok(())
}

/// Whether image and anchor rules apply: below a tinker internal_level of 2, or
/// without a tinker.
fn media_restricted_below_lv2(tinker: Option<&Tinker>) -> bool {
    tinker.is_none_or(|t| t.internal_level() < 2)
}

/// Reply only: the creation interval is [`RES_SPAN_BELOW_LV2_SECS`] below public
/// level 2 (and without a tinker), and the board value from level 2 on.
pub fn res_creation_span_sec(tinker: Option<&Tinker>, board_info: &BoardInfo) -> u64 {
    match tinker {
        Some(tinker) if tinker.level() >= 2 => board_info.base_response_creation_span_sec as u64,
        _ => RES_SPAN_BELOW_LV2_SECS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    use chrono::Utc;
    use eddist_core::domain::tinker::Tinker;
    use uuid::Uuid;

    use crate::domain::{authed_token::AuthedToken, metadent::MetadentType};

    fn board_info() -> BoardInfo {
        let now = Utc::now().naive_utc();
        BoardInfo {
            id: Uuid::now_v7(),
            local_rules: String::new(),
            base_thread_creation_span_sec: 120,
            base_response_creation_span_sec: 5,
            max_thread_name_byte_length: 50,
            max_author_name_byte_length: 50,
            max_email_byte_length: 50,
            max_response_body_byte_length: 2000,
            max_response_body_lines: 30,
            threads_archive_cron: None,
            threads_archive_trigger_thread_count: None,
            created_at: now,
            updated_at: now,
            read_only: false,
            force_metadent_type: None,
            enable_1001_message: false,
            custom_1001_message: None,
        }
    }

    fn configured_tinker(level: u32, internal_level: u32) -> Tinker {
        Tinker::from_parts("token".to_string(), 0, 0, level, internal_level, 0, 0, None)
    }

    fn initialized_res(body: &str) -> Res<AuthorIdInitialized> {
        let token = AuthedToken::new("192.0.2.1".to_string(), "test-ua".to_string(), 0);
        Res::new_from_res(
            ResCore {
                from: "名無しさん",
                mail: "",
                body: Cow::Borrowed(body),
            },
            "test-board",
            Utc::now(),
            MetadentType::None,
            ClientInfo {
                user_agent: "test-ua".to_string(),
                asn_num: 0,
                ip_addr: "192.0.2.1".to_string(),
                tinker: None,
            },
            None,
            false,
        )
        .set_author_id(&token, None)
    }

    #[test]
    fn thread_title_is_validated_only_for_the_thread_kind() {
        let board_info = board_info();
        let res = ResCore {
            from: "name",
            mail: "",
            body: Cow::Borrowed("body"),
        };

        assert!(matches!(
            validate_content_length(&res, Some(""), &board_info),
            Err(BbsCgiError::ContentEmpty(ContentEmptyParamType::ThreadName))
        ));

        let long_title = "a".repeat(30);
        assert!(matches!(
            validate_content_length(&res, Some(&long_title), &board_info),
            Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::ThreadName
            ))
        ));

        assert!(validate_content_length(&res, Some("short title"), &board_info).is_ok());
        assert!(validate_content_length(&res, None, &board_info).is_ok());
    }

    #[test]
    fn res_images_are_rejected_below_internal_level_2() {
        let body = "image https://example.com/pic.png";

        assert!(matches!(
            validate_res_images(&mut initialized_res(body), None),
            Err(BbsCgiError::ImageUrlBelowLv2)
        ));

        let low = configured_tinker(5, 1);
        assert!(matches!(
            validate_res_images(&mut initialized_res(body), Some(&low)),
            Err(BbsCgiError::ImageUrlBelowLv2)
        ));

        let high = configured_tinker(5, 2);
        assert!(validate_res_images(&mut initialized_res(body), Some(&high)).is_ok());
    }

    #[test]
    fn res_anchors_over_three_are_rejected_below_internal_level_2() {
        let body = ">>1 >>2 >>3 >>4";

        assert!(matches!(
            validate_res_anchors(body, None),
            Err(BbsCgiError::NgWordDetected)
        ));

        let low = configured_tinker(5, 1);
        assert!(matches!(
            validate_res_anchors(body, Some(&low)),
            Err(BbsCgiError::NgWordDetected)
        ));

        let high = configured_tinker(5, 2);
        assert!(validate_res_anchors(body, Some(&high)).is_ok());
        assert!(validate_res_anchors(">>1 >>2 >>3", None).is_ok());
    }

    #[test]
    fn res_span_is_thirty_seconds_below_public_level_2() {
        let board_info = board_info();

        assert_eq!(res_creation_span_sec(None, &board_info), 30);
        let level1 = configured_tinker(1, 1);
        assert_eq!(res_creation_span_sec(Some(&level1), &board_info), 30);
        let level2 = configured_tinker(2, 2);
        assert_eq!(res_creation_span_sec(Some(&level2), &board_info), 5);
    }

    #[test]
    fn thread_creation_without_tinker_respects_the_board_restriction() {
        let board_info = board_info();
        let client_info = ClientInfo {
            user_agent: "test-ua".to_string(),
            asn_num: 0,
            ip_addr: "192.0.2.1".to_string(),
            tinker: None,
        };

        assert!(matches!(
            validate_client_info(&client_info, &board_info, PostingKind::Thread, true),
            Err(BbsCgiError::TmpCanNotCreateThread)
        ));
        assert!(
            validate_client_info(&client_info, &board_info, PostingKind::Thread, false).is_ok()
        );
        assert!(validate_client_info(&client_info, &board_info, PostingKind::Res, true).is_ok());
    }
}
