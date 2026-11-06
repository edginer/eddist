use core::{fmt::Display, str};

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
        thread_retrieval_service::{DatContent, DatRetrievalError, ThreadRetrievalServiceInput},
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

/// Accept unquoted tags as the previous size-only parser did.
fn etag_value(value: &str) -> Option<&str> {
    let inner = value.trim();
    let inner = inner.strip_prefix("W/").unwrap_or(inner);
    match inner.strip_prefix('"') {
        Some(quoted) => quoted.strip_suffix('"'),
        None => Some(inner),
    }
}

/// Conditional request state for one dat response.
struct DatConditions {
    /// The request's `If-None-Match` value, used for conditional response checks.
    if_none_match: Option<String>,
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
    fn parse(headers: &HeaderMap) -> Self {
        let if_none_match = headers
            .get("If-None-Match")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

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
            range,
        }
    }

    fn is_not_modified(&self, etag: &str) -> bool {
        self.range == DatRange::Full && matches_current_etag(self.if_none_match.as_deref(), etag)
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

fn dat_etag(board_key: &str, thread_number: impl Display, byte_size: usize) -> String {
    format!("W/\"{board_key}-{thread_number}-{byte_size}\"")
}

fn matches_current_etag(if_none_match: Option<&str>, etag: &str) -> bool {
    if_none_match.is_some_and(|value| {
        let value = value.trim();
        if value == "*" {
            return true;
        }
        let Some(current) = etag_value(etag) else {
            return false;
        };

        // Commas inside a quoted opaque tag are part of that tag.
        let mut quoted = false;
        value
            .split(move |ch| match ch {
                '"' => {
                    quoted = !quoted;
                    false
                }
                ',' => !quoted,
                _ => false,
            })
            .filter_map(etag_value)
            .any(|candidate| candidate == current)
    })
}

fn not_modified_response(etag: String, cache_control: &str) -> Response {
    Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header("Cache-Control", cache_control)
        .header("ETag", etag)
        .body(Body::empty())
        .unwrap()
}

fn dat_response(path: &DatPath, conditions: DatConditions, dat: DatContent) -> Response {
    let etag = dat_etag(&path.board_key, path.thread_number, dat.byte_size());
    if conditions.is_not_modified(&etag) {
        return not_modified_response(etag, "max-age=5,s-maxage=1");
    }

    dat_body_response(conditions.range, dat, etag)
}

/// Builds a body response without inspecting conditional request headers.
fn dat_body_response(range: DatRange, dat: DatContent, etag: String) -> Response {
    let (raw, etag, status) = match range {
        DatRange::From(start) => (
            dat.into_bytes().get(start..).unwrap_or_default().to_vec(),
            None,
            StatusCode::PARTIAL_CONTENT,
        ),
        DatRange::Full => (dat.into_bytes(), Some(etag), StatusCode::OK),
        DatRange::Unparsable => {
            return Response::builder().status(400).body(Body::empty()).unwrap();
        }
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(raw))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .with_etag(etag)
        .status_code(status)
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
    let conditions = DatConditions::parse(&headers);

    let svc = state.get_container().thread_retrieval();
    let result = match svc
        .execute(ThreadRetrievalServiceInput {
            board_key: path.board_key.clone(),
            thread_number: path.thread_number,
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

    dat_response(&path, conditions, result)
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

fn kako_dat_response(
    board_key: &str,
    thread_number: &str,
    if_none_match: Option<&str>,
    raw: Vec<u8>,
) -> Response {
    let etag = dat_etag(board_key, thread_number, raw.len());
    if matches_current_etag(if_none_match, &etag) {
        return not_modified_response(etag, "s-maxage=3600");
    }

    SJisResponseBuilder::new(kako_dat_to_sjis(raw))
        .content_type(SjisContentType::TextPlain)
        .server_ttl(3600)
        .with_etag(Some(etag))
        .build()
        .into_response()
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

    let if_none_match = headers.get("If-None-Match").and_then(|v| v.to_str().ok());

    kako_dat_response(&board_key, thread_number, if_none_match, result)
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

    fn dat_contents(raw: &[u8]) -> [DatContent; 2] {
        [
            DatContent::new(vec![raw.to_vec()]),
            DatContent::new(raw.chunks(7).map(<[u8]>::to_vec).collect()),
        ]
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
    fn conditional_checks_compare_the_whole_current_tag() {
        let current = r#"W/"bbs-1234567890-4096""#;
        for matching in [
            current,
            r#""bbs-1234567890-4096""#,
            "W/bbs-1234567890-4096",
            "*",
            r#"W/"bbs-1234567890-10", W/"bbs-1234567890-4096", W/"other-1234567890-20""#,
        ] {
            assert!(matches_current_etag(Some(matching), current), "{matching}");
        }
        for different in [
            r#"W/"other-1234567890-4096""#,
            r#"W/"bbs-9999999999-4096""#,
            r#"W/"bbs-1234567890-4097""#,
            r#"W/"bbs-4096""#,
            r#"W/"bbs-thread-4096""#,
            r#"W/"bbs-1234567890-x""#,
            r#"W/"bbs-1234567890-4096"#,
            r#"W/"other,W/bbs-1234567890-4096,tail""#,
        ] {
            assert!(
                !matches_current_etag(Some(different), current),
                "{different}"
            );
        }
        assert!(!matches_current_etag(None, current));
    }

    #[test]
    fn conditional_checks_are_skipped_for_partial_responses() {
        let current = r#"W/"bbs-1234567890-4096""#;
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", current.parse().unwrap());
        assert!(DatConditions::parse(&headers).is_not_modified(current));

        headers.insert("Range", "bytes=0-".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0".parse().unwrap());
        assert!(!DatConditions::parse(&headers).is_not_modified(current));
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
        let mut headers = HeaderMap::new();
        headers.insert("Range", "bytes=abc".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0".parse().unwrap());
        assert_eq!(DatConditions::parse(&headers).range, DatRange::Unparsable);

        // Xeno cannot handle partial responses, so the Range header is ignored.
        headers.insert("User-Agent", "Xeno/1.0".parse().unwrap());
        let conditions = DatConditions::parse(&headers);
        assert_eq!(conditions.range, DatRange::Full);
    }

    #[tokio::test]
    async fn not_modified_responses_do_not_depend_on_chunk_boundaries() {
        let path = dat_path();
        let raw = SJisStr::from("名前<>メール<>日時<>本文<>題名\n").get_inner();
        let byte_size = raw.len();
        let etag = dat_etag(&path.board_key, path.thread_number, byte_size);
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", etag.parse().unwrap());

        for dat in dat_contents(&raw) {
            let response = dat_response(&path, DatConditions::parse(&headers), dat);
            assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
            assert_eq!(response.headers()["ETag"], etag);
            assert_eq!(response.headers()["Cache-Control"], "max-age=5,s-maxage=1");
            assert!(!response.headers().contains_key("If-None-Match"));
            assert!(
                axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[tokio::test]
    async fn full_reads_return_shift_jis_when_the_request_tag_does_not_match() {
        let path = dat_path();
        let raw = SJisStr::from("名前<>メール<>日時<>本文<>題名\n").get_inner();
        let etag = dat_etag(&path.board_key, path.thread_number, raw.len());

        for request_tag in [
            None,
            Some(dat_etag(&path.board_key, path.thread_number, raw.len() + 1)),
            Some(dat_etag("other", path.thread_number, raw.len())),
        ] {
            let mut headers = HeaderMap::new();
            if let Some(request_tag) = request_tag {
                headers.insert("If-None-Match", request_tag.parse().unwrap());
            }
            for dat in dat_contents(&raw) {
                let response = dat_response(&path, DatConditions::parse(&headers), dat);
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(response.headers()["ETag"], etag);
                assert_eq!(
                    response.headers()["Content-Type"],
                    "text/plain; charset=Shift_JIS;"
                );
                assert!(!response.headers().contains_key("If-None-Match"));
                assert_eq!(
                    axum::body::to_bytes(response.into_body(), usize::MAX)
                        .await
                        .unwrap()
                        .as_ref(),
                    raw
                );
            }
        }
    }

    #[tokio::test]
    async fn full_reads_still_match_the_wildcard() {
        let path = dat_path();
        let raw = b"dat body\n".to_vec();
        let etag = dat_etag(&path.board_key, path.thread_number, raw.len());
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "*".parse().unwrap());

        let response = dat_response(
            &path,
            DatConditions::parse(&headers),
            DatContent::new(vec![raw]),
        );
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()["ETag"], etag);
        assert!(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn not_modified_responses_generate_the_current_etag_instead_of_echoing_the_request() {
        let path = dat_path();
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "W/bbs-1234567890-4096".parse().unwrap());
        let conditions = DatConditions::parse(&headers);

        let response = dat_response(&path, conditions, DatContent::new(vec![vec![b'a'; 4096]]));
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()["ETag"], r#"W/"bbs-1234567890-4096""#);
    }

    #[tokio::test]
    async fn malformed_ranges_are_not_overridden_by_a_matching_etag() {
        let path = dat_path();
        let raw = b"dat body\n".to_vec();
        let etag = dat_etag(&path.board_key, path.thread_number, raw.len());
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", etag.parse().unwrap());
        headers.insert("Range", "bytes=abc".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0".parse().unwrap());

        let response = dat_response(
            &path,
            DatConditions::parse(&headers),
            DatContent::new(vec![raw]),
        );
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn partial_reads_skip_conditional_checks_but_xeno_gets_a_full_response() {
        let path = dat_path();
        let raw = b"dat body\n".to_vec();
        let etag = dat_etag(&path.board_key, path.thread_number, raw.len());
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", etag.parse().unwrap());
        headers.insert("Range", "bytes=3-".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0".parse().unwrap());

        let response = dat_response(
            &path,
            DatConditions::parse(&headers),
            DatContent::new(vec![raw.clone()]),
        );
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert!(!response.headers().contains_key("ETag"));
        assert_eq!(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .as_ref(),
            &raw[3..]
        );

        headers.insert("User-Agent", "Xeno/1.0".parse().unwrap());
        let response = dat_response(
            &path,
            DatConditions::parse(&headers),
            DatContent::new(vec![raw]),
        );
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()["ETag"], etag);
    }

    #[tokio::test]
    async fn archived_responses_keep_their_etag_cache_policy_and_shift_jis_body() {
        let text = "名前<>メール<>日時<>本文<>題名\n";
        let sjis_bytes = SJisStr::from(text).get_inner();

        for raw in [text.as_bytes().to_vec(), sjis_bytes.clone()] {
            // Archived ETags continue to describe the stored bytes before conversion.
            let etag = dat_etag("bbs", "1234567890", raw.len());
            for (request_tag, expected_status) in [
                (None, StatusCode::OK),
                (Some(etag.as_str()), StatusCode::NOT_MODIFIED),
                (Some("*"), StatusCode::NOT_MODIFIED),
                (Some(r#"W/"other-1234567890-1""#), StatusCode::OK),
            ] {
                let response = kako_dat_response("bbs", "1234567890", request_tag, raw.clone());
                assert_eq!(response.status(), expected_status);
                assert_eq!(response.headers()["ETag"], etag);
                assert_eq!(response.headers()["Cache-Control"], "s-maxage=3600");
                assert!(!response.headers().contains_key("If-None-Match"));
                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                if expected_status == StatusCode::OK {
                    assert_eq!(body.as_ref(), sjis_bytes);
                } else {
                    assert!(body.is_empty());
                }
            }
        }
    }
}
