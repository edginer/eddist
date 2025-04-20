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
    services::{
        kako_thread_retrieval_service::KakoThreadRetrievalServiceInput,
        thread_retrieval_service::ThreadRetrievalServiceInput, AppService,
    },
    shiftjis::{SJisResponseBuilder, SjisContentType},
    AppState,
};

pub async fn get_dat_txt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((board_key, thread_id_with_dat)): Path<(String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    let thread_number = thread_id_with_dat.replace(".dat", "");
    let Ok(thread_number_num) = thread_number.parse::<i64>() else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let svc = state.get_container().thread_retrival();
    let result = match svc
        .execute(ThreadRetrievalServiceInput {
            board_key: board_key.clone(),
            thread_number: thread_number_num as u64,
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

    let range = headers.get("Range");
    let ua = headers.get("User-Agent").map(|x| x.to_str().unwrap());

    let (result, is_partial) = match (range, ua) {
        (Some(range), Some(ua)) if !ua.contains("Xeno") => {
            let range = range.to_str().unwrap();
            if let Some(range) = range.split('=').nth(1) {
                let range = range.split('-').collect::<Vec<_>>();
                let Some(start) = range.first().and_then(|x| x.parse::<usize>().ok()) else {
                    return Response::builder().status(400).body(Body::empty()).unwrap();
                };

                let raw = result.raw().into_iter().skip(start).collect::<Vec<_>>();
                (raw, true)
            } else {
                (result.raw(), false)
            }
        }
        _ => (result.raw(), false),
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(result))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
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
    Path((board_key, _, _, thread_id_with_dat)): Path<(String, String, String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    let thread_number = thread_id_with_dat.replace(".dat", "");

    let svc = state.get_container().kako_thread_retrieval();

    let result = match svc
        .execute(KakoThreadRetrievalServiceInput {
            board_key,
            thread_number,
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

    let sjis_str = if let Ok(result) = str::from_utf8(&result) {
        SJisStr::from(result)
    } else {
        SJisStr::from_unchecked_vec(result)
    };

    SJisResponseBuilder::new(sjis_str)
        .content_type(SjisContentType::TextPlain)
        .server_ttl(3600)
        .build()
        .into_response()
}
