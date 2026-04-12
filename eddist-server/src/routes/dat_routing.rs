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
        AppService, kako_thread_retrieval_service::KakoThreadRetrievalServiceInput,
        thread_retrieval_service::ThreadRetrievalServiceInput,
    },
    shiftjis::{SJisResponseBuilder, SjisContentType},
};

/// Extracts the byte-size suffix from a dat ETag of the form `W/"board-thread-SIZE"`.
/// Board keys are `[a-z0-9]+` and thread numbers are digits, so the last `-`-delimited
/// segment is always the size.
fn parse_etag_byte_size(inm: &str) -> Option<usize> {
    let inner = inm.trim().trim_start_matches("W/\"").trim_end_matches('"');
    inner.rsplit('-').next()?.parse().ok()
}

pub async fn get_dat_txt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((board_key, thread_id_with_dat)): Path<(String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    let Some(thread_number) = thread_id_with_dat.strip_suffix(".dat") else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };
    let Ok(thread_number_num) = thread_number.parse::<i64>() else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    // Parse the expected byte size from If-None-Match before the service call so
    // the service can skip flatten+collect when the cache size hasn't changed.
    // Range requests don't use ETags for conditional checks, so skip them.
    let range = headers.get("Range");
    let if_none_match_hdr = headers.get("If-None-Match");
    let expected_byte_size = if range.is_none() {
        if_none_match_hdr
            .and_then(|v| v.to_str().ok())
            .and_then(parse_etag_byte_size)
    } else {
        None
    };

    let svc = state.get_container().thread_retrival();
    let result = match svc
        .execute(ThreadRetrievalServiceInput {
            board_key: board_key.clone(),
            thread_number: thread_number_num as u64,
            expected_byte_size,
        })
        .await
    {
        Ok(raw) => raw,
        Err(e) => {
            if e.root_cause()
                .to_string()
                .contains("cannot find such thread")
            {
                let current_unix_epoch = Utc::now().timestamp();
                return if thread_number_num > current_unix_epoch {
                    // Not found response
                    Response::builder().status(404).body(Body::empty()).unwrap()
                } else {
                    // Redirect to kako thread
                    Response::builder()
                        .header("Cache-Control", "s-maxage=86400, max-age=86400")
                        .status(302)
                        .header(
                            "Location",
                            format!(
                                "/{}/kako/{}/{}/{}.dat",
                                board_key,
                                &thread_number[0..4],
                                &thread_number[0..5],
                                thread_number
                            ),
                        )
                        .body(Body::empty())
                        .unwrap()
                };
            } else {
                // Not found response
                return Response::builder().status(404).body(Body::empty()).unwrap();
            }
        }
    };

    let Some(result) = result.raw() else {
        let etag_val = if_none_match_hdr.unwrap();
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header("ETag", etag_val)
            .header("Cache-Control", "max-age=5,s-maxage=1")
            .body(Body::empty())
            .unwrap();
    };

    let ua = headers.get("User-Agent").map(|x| x.to_str().unwrap());

    let (result, is_partial) = match (range, ua) {
        (Some(range), Some(ua)) if !ua.contains("Xeno") => {
            let range = range.to_str().unwrap();
            if let Some(range) = range.split('=').nth(1) {
                let range = range.split('-').collect::<Vec<_>>();
                let Some(start) = range.first().and_then(|x| x.parse::<usize>().ok()) else {
                    return Response::builder().status(400).body(Body::empty()).unwrap();
                };
                (result.get(start..).unwrap_or_default().to_vec(), true)
            } else {
                (result, false)
            }
        }
        _ => (result, false),
    };

    let (if_none_match, etag) = if !is_partial {
        let inm = if_none_match_hdr
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let etag = Some(format!(
            "W/\"{}-{}-{}\"",
            board_key,
            thread_number_num,
            result.len()
        ));
        (inm, etag)
    } else {
        (None, None)
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(result))
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

pub async fn get_kako_dat_txt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((board_key, _, _, thread_id_with_dat)): Path<(String, String, String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    let Some(thread_number) = thread_id_with_dat.strip_suffix(".dat") else {
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
        Err(err) => {
            return if err.to_string().contains("Thread not found") {
                Response::builder().status(404).body(Body::empty()).unwrap()
            } else {
                Response::builder().status(500).body(Body::empty()).unwrap()
            };
        }
    };

    let etag = Some(format!(
        "W/\"{}-{}-{}\"",
        board_key,
        thread_number,
        result.len()
    ));
    let sjis_str = if let Ok(result) = str::from_utf8(&result) {
        SJisStr::from(result)
    } else {
        SJisStr::from_unchecked_vec(result)
    };

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
