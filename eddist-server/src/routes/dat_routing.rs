use core::str;

use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use eddist_core::domain::{board::validate_board_key, sjis_str::SJisStr};
use http::{HeaderMap, StatusCode};

use crate::{
    AppState,
    services::{
        AppService,
        kako_thread_retrieval_service::{
            KakoThreadRetrievalError, KakoThreadRetrievalServiceInput,
        },
        thread_retrieval_service::{DatReadResult, DatRetrievalError, ThreadRetrievalServiceInput},
    },
    shiftjis::{SJisResponseBuilder, SjisContentType},
    utils::get_ua,
};

fn parse_dat_file_name(thread_id_with_dat: &str) -> Option<&str> {
    if thread_id_with_dat.len() != 14 {
        return None;
    }
    thread_id_with_dat.strip_suffix(".dat")
}

struct DatPath {
    board_key: String,
    /// The thread number exactly as it appeared in the path; the archive redirect
    /// rebuilds the URL from these digits.
    thread_number_str: String,
    thread_number: u64,
}

impl DatPath {
    fn parse(board_key: &str, thread_id_with_dat: &str) -> Option<Self> {
        let thread_number_str = parse_dat_file_name(thread_id_with_dat)?;
        let thread_number = thread_number_str.parse::<i64>().ok()?;
        if validate_board_key(board_key).is_err() {
            return None;
        }
        Some(Self {
            board_key: board_key.to_string(),
            thread_number_str: thread_number_str.to_string(),
            thread_number: thread_number as u64,
        })
    }
}

/// A dat ETag names its resource: `W/"<boardKey>-<threadNumber>-<byte size>"`.
/// A tag from another resource must not be used for this resource's conditional
/// checks even when the byte size happens to match.
#[derive(Debug, PartialEq, Eq)]
struct DatEtag {
    board_key: String,
    thread_number: u64,
    byte_size: usize,
}

/// Quotes are optional: some clients send the unquoted `W/bbs-...` form.
fn parse_dat_etag(value: &str) -> Option<DatEtag> {
    let inner = value.trim();
    let inner = inner.strip_prefix("W/").unwrap_or(inner);
    let inner = match inner.strip_prefix('"') {
        Some(quoted) => quoted.strip_suffix('"')?,
        None => inner,
    };
    let mut parts = inner.rsplitn(3, '-');
    let byte_size = parts.next()?.parse().ok()?;
    let thread_number = parts.next()?.parse().ok()?;
    let board_key = parts.next()?.to_string();
    Some(DatEtag {
        board_key,
        thread_number,
        byte_size,
    })
}

impl DatEtag {
    fn matches(&self, path: &DatPath) -> bool {
        self.board_key == path.board_key && self.thread_number == path.thread_number
    }
}

/// Conditional request state for one dat response.
struct DatConditions {
    /// The raw `If-None-Match` value, echoed on a 304.
    if_none_match: Option<String>,
    /// Byte size from `If-None-Match` when the tag names this board and thread and
    /// no Range applies; lets the cache path skip materializing the payload.
    known_byte_size: Option<usize>,
    range: DatRange,
}

/// The Xeno client sends Range headers but cannot handle 206 responses, so
/// ranges are only honored for other clients.
#[derive(Debug, PartialEq, Eq)]
enum DatRange {
    Full,
    From(usize),
    /// Rejected only once the thread is found, so missing and archived threads
    /// still get their 404 or archive redirect.
    Unparsable,
}

impl DatConditions {
    fn parse(headers: &HeaderMap, path: &DatPath) -> Self {
        let if_none_match = headers
            .get("If-None-Match")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        // Range requests don't use ETags for conditional checks.
        let known_byte_size = if headers.contains_key("Range") {
            None
        } else {
            if_none_match.as_deref().and_then(|value| {
                value
                    .rsplit(',')
                    .filter_map(parse_dat_etag)
                    .find(|etag| etag.matches(path))
                    .map(|etag| etag.byte_size)
            })
        };

        let range = match (headers.get("Range"), get_ua(headers)) {
            (Some(range), Some(ua)) if !ua.contains("Xeno") => {
                match range.to_str().ok().and_then(parse_range_start) {
                    Some(Ok(start)) => DatRange::From(start),
                    Some(Err(())) => DatRange::Unparsable,
                    None => DatRange::Full,
                }
            }
            _ => DatRange::Full,
        };

        Self {
            if_none_match,
            known_byte_size,
            range,
        }
    }
}

/// Extracts the start byte of a `bytes=N-` Range header. `None` when the header
/// carries no range spec, `Some(Err)` when the spec's start byte is malformed.
fn parse_range_start(range: &str) -> Option<Result<usize, ()>> {
    let start = range.split('=').nth(1)?;
    Some(start.split('-').next()?.parse::<usize>().map_err(|_| ()))
}

fn dat_retrieval_error_response(
    error: &DatRetrievalError,
    board_key: &str,
    thread_number: &str,
    now: i64,
) -> Response {
    match error {
        DatRetrievalError::ThreadNotFound
            if thread_number.parse::<i64>().unwrap_or(i64::MAX) <= now =>
        {
            Response::builder()
                .header("Cache-Control", "s-maxage=86400, max-age=86400")
                .status(StatusCode::FOUND)
                .header(
                    "Location",
                    format!(
                        "/{board_key}/kako/{}/{}/{thread_number}.dat",
                        &thread_number[0..4],
                        &thread_number[0..5],
                    ),
                )
                .body(Body::empty())
                .unwrap()
        }
        DatRetrievalError::ThreadNotFound | DatRetrievalError::BoardNotFound => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
        DatRetrievalError::Other(error) => {
            tracing::error!(
                error = %error,
                board_key,
                thread_number,
                "failed to retrieve dat"
            );
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        }
    }
}

fn not_modified_response(if_none_match: Option<String>) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header("Cache-Control", "max-age=5,s-maxage=1");
    if let Some(etag) = if_none_match {
        builder = builder.header("ETag", etag);
    }
    builder.body(Body::empty()).unwrap()
}

/// Builds the full or partial dat response. The ETag is only attached to (and only
/// matches) full responses for this board and thread.
fn dat_found_response(path: &DatPath, conditions: DatConditions, raw: Vec<u8>) -> Response {
    let (raw, is_partial) = match conditions.range {
        DatRange::From(start) => (raw.get(start..).unwrap_or_default().to_vec(), true),
        DatRange::Full => (raw, false),
        DatRange::Unparsable => {
            return Response::builder().status(400).body(Body::empty()).unwrap();
        }
    };

    let (if_none_match, etag) = if is_partial {
        (None, None)
    } else {
        let etag = Some(format!(
            "W/\"{}-{}-{}\"",
            path.board_key,
            path.thread_number,
            raw.len()
        ));
        (conditions.if_none_match, etag)
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(raw))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .if_none_match(if_none_match)
        .with_etag(etag)
        .status_code(if is_partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .build()
        .into_response()
}

pub async fn get_dat_txt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((board_key, thread_id_with_dat)): Path<(String, String)>,
) -> Response {
    let Some(path) = DatPath::parse(&board_key, &thread_id_with_dat) else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };
    let conditions = DatConditions::parse(&headers, &path);

    let svc = state.get_container().thread_retrieval();
    let result = match svc
        .execute(ThreadRetrievalServiceInput {
            board_key: path.board_key.clone(),
            thread_number: path.thread_number,
            expected_byte_size: conditions.known_byte_size,
        })
        .await
    {
        Ok(result) => result,
        Err(e) => {
            return dat_retrieval_error_response(
                &e,
                &path.board_key,
                &path.thread_number_str,
                Utc::now().timestamp(),
            );
        }
    };

    match result {
        DatReadResult::Unchanged => not_modified_response(conditions.if_none_match),
        DatReadResult::Found(raw) => dat_found_response(&path, conditions, raw),
    }
}

/// Archived dat files may be stored as UTF-8 (converted) or as raw Shift-JIS.
/// Both are served as Shift-JIS, re-encoding only bytes that are valid UTF-8.
fn kako_dat_to_sjis(bytes: Vec<u8>) -> SJisStr {
    if let Ok(text) = str::from_utf8(&bytes) {
        SJisStr::from(text)
    } else {
        SJisStr::from_unchecked_vec(bytes)
    }
}

pub async fn get_kako_dat_txt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((board_key, _, _, thread_id_with_dat)): Path<(String, String, String, String)>,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    let Some(thread_number) = parse_dat_file_name(&thread_id_with_dat) else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };

    let svc = state.get_container().kako_thread_retrieval();

    let result = match svc
        .execute(KakoThreadRetrievalServiceInput {
            board_key: board_key.clone(),
            thread_number: thread_number.to_owned(),
        })
        .await
    {
        Ok(result) => result,
        Err(KakoThreadRetrievalError::NotFound) => {
            return Response::builder().status(404).body(Body::empty()).unwrap();
        }
        Err(KakoThreadRetrievalError::Other(_)) => {
            return Response::builder().status(500).body(Body::empty()).unwrap();
        }
    };

    let etag = Some(format!(
        "W/\"{}-{}-{}\"",
        board_key,
        thread_number,
        result.len()
    ));
    let sjis_str = kako_dat_to_sjis(result);

    let if_none_match = headers
        .get("If-None-Match")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    SJisResponseBuilder::new(sjis_str)
        .content_type(SjisContentType::TextPlain)
        .server_ttl(3600)
        .if_none_match(if_none_match)
        .with_etag(etag)
        .build()
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dat_path() -> DatPath {
        DatPath {
            board_key: "bbs".to_string(),
            thread_number_str: "1234567890".to_string(),
            thread_number: 1234567890,
        }
    }

    #[test]
    fn dependency_failures_are_internal_errors() {
        for error in [
            anyhow::anyhow!("database unavailable"),
            anyhow::anyhow!("query failed: cannot find such thread"),
        ] {
            let response = dat_retrieval_error_response(
                &DatRetrievalError::Other(error),
                "bbs",
                "1234567890",
                1234567891,
            );
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert!(!response.headers().contains_key("Location"));
        }
    }

    #[test]
    fn missing_threads_keep_archive_redirect_and_future_not_found() {
        let error = DatRetrievalError::ThreadNotFound;
        let response = dat_retrieval_error_response(&error, "bbs", "1234567890", 1234567891);
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers()["Location"],
            "/bbs/kako/1234/12345/1234567890.dat"
        );
        let response = dat_retrieval_error_response(&error, "bbs", "1234567890", 1234567889);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn missing_board_remains_not_found() {
        let response = dat_retrieval_error_response(
            &DatRetrievalError::BoardNotFound,
            "bbs",
            "1234567890",
            1234567891,
        );
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn dat_etags_parse_only_complete_tags() {
        assert_eq!(
            parse_dat_etag(r#"W/"bbs-1234567890-4096""#),
            Some(DatEtag {
                board_key: "bbs".to_string(),
                thread_number: 1234567890,
                byte_size: 4096,
            })
        );
        assert_eq!(
            parse_dat_etag(r#""bbs-1234567890-4096""#),
            Some(DatEtag {
                board_key: "bbs".to_string(),
                thread_number: 1234567890,
                byte_size: 4096,
            })
        );
        assert_eq!(
            parse_dat_etag("W/bbs-1234567890-4096"),
            Some(DatEtag {
                board_key: "bbs".to_string(),
                thread_number: 1234567890,
                byte_size: 4096,
            })
        );
        for invalid in [
            "*",
            r#"W/"bbs-4096""#,
            r#"W/"bbs-thread-4096""#,
            r#"W/"bbs-1234567890-x""#,
            r#"W/"bbs-1234567890-4096"#,
        ] {
            assert_eq!(parse_dat_etag(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn known_size_ignores_tags_of_other_resources_and_range_requests() {
        let path = dat_path();

        let mut headers = HeaderMap::new();
        headers.insert(
            "If-None-Match",
            r#"W/"bbs-1234567890-4096""#.parse().unwrap(),
        );
        let conditions = DatConditions::parse(&headers, &path);
        assert_eq!(conditions.known_byte_size, Some(4096));

        headers.insert(
            "If-None-Match",
            r#"W/"other-1234567890-4096""#.parse().unwrap(),
        );
        let conditions = DatConditions::parse(&headers, &path);
        assert_eq!(conditions.known_byte_size, None);

        headers.insert(
            "If-None-Match",
            r#"W/"bbs-9999999999-4096""#.parse().unwrap(),
        );
        let conditions = DatConditions::parse(&headers, &path);
        assert_eq!(conditions.known_byte_size, None);

        headers.insert(
            "If-None-Match",
            r#"W/"bbs-1234567890-10", W/"other-1234567890-30", W/"bbs-1234567890-20""#
                .parse()
                .unwrap(),
        );
        let conditions = DatConditions::parse(&headers, &path);
        assert_eq!(conditions.known_byte_size, Some(20));

        headers.insert(
            "If-None-Match",
            r#"W/"bbs-1234567890-4096""#.parse().unwrap(),
        );
        headers.insert("Range", "bytes=0-".parse().unwrap());
        let conditions = DatConditions::parse(&headers, &path);
        assert_eq!(conditions.known_byte_size, None);
    }

    #[test]
    fn range_start_parsing_matches_legacy_behavior() {
        assert_eq!(parse_range_start("bytes=100-"), Some(Ok(100)));
        assert_eq!(parse_range_start("bytes=10-x"), Some(Ok(10)));
        assert_eq!(parse_range_start("bytes=-100"), Some(Err(())));
        assert_eq!(parse_range_start("bytes=abc"), Some(Err(())));
        assert_eq!(parse_range_start("100-"), None);
    }

    #[test]
    fn unparsable_ranges_are_rejected_only_for_clients_that_honor_them() {
        let path = dat_path();

        let mut headers = HeaderMap::new();
        headers.insert("Range", "bytes=abc".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0".parse().unwrap());
        assert_eq!(
            DatConditions::parse(&headers, &path).range,
            DatRange::Unparsable
        );

        // Xeno cannot handle partial responses, so the Range header is ignored.
        headers.insert("User-Agent", "Xeno/1.0".parse().unwrap());
        let conditions = DatConditions::parse(&headers, &path);
        assert_eq!(conditions.range, DatRange::Full);
    }
}
