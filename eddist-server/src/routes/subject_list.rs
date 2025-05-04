use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use eddist_core::domain::{board::validate_board_key, sjis_str::SJisStr};

use crate::{
    AppState,
    services::{AppService, thread_list_service::BoardKey},
    shiftjis::{SJisResponseBuilder, SjisContentType},
};

pub async fn get_subject_txt(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let svc = state.get_container().thread_list();
    let threads = match svc.execute(BoardKey(board_key)).await {
        Ok(threads) => threads,
        Err(e) => {
            return if e.to_string().contains("failed to find board info") {
                Response::builder().status(404).body(Body::empty()).unwrap()
            } else {
                log::error!("Failed to get thread list: {e:?}");
                Response::builder().status(500).body(Body::empty()).unwrap()
            };
        }
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(threads.get_sjis_thread_list()))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .build()
        .into_response()
}

pub async fn get_subject_txt_with_metadent(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let svc = state.get_container().metadent_thread_list();
    let threads = match svc
        .execute(crate::services::metadent_thread_list_service::BoardKey(
            board_key,
        ))
        .await
    {
        Ok(threads) => threads,
        Err(e) => {
            return if e.to_string().contains("failed to find board info") {
                Response::builder().status(404).body(Body::empty()).unwrap()
            } else {
                log::error!("Failed to get thread list: {e:?}");
                Response::builder().status(500).body(Body::empty()).unwrap()
            };
        }
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(threads.get_sjis_thread_list()))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .build()
        .into_response()
}
