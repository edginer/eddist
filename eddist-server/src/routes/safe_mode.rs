use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use eddist_core::{domain::board::validate_board_key, redis_keys::unsafe_threads_key};
use redis::AsyncCommands;
use serde::Serialize;

use crate::{
    AppState,
    services::{AppService, board_info_service::BoardInfoServiceInput},
};

#[derive(Serialize)]
pub struct UnsafeThreadIdsResponse {
    pub thread_ids: Vec<u64>,
}

pub async fn get_unsafe_thread_ids(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let board_info = state
        .get_container()
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await;

    let board_id = match board_info {
        Ok(info) => info.board_id,
        Err(e) => {
            if e.to_string().contains("board not found") {
                return Response::builder().status(404).body(Body::empty()).unwrap();
            }
            log::error!("Failed to get board info for safe mode: {e:?}");
            return Response::builder().status(500).body(Body::empty()).unwrap();
        }
    };

    let key = unsafe_threads_key(board_id);
    let mut redis_conn = state.redis_conn.clone();
    let thread_ids: Vec<u64> = match redis_conn.smembers(&key).await {
        Ok(ids) => ids,
        Err(e) => {
            log::error!("Failed to query unsafe threads from Redis: {e:?}");
            vec![]
        }
    };

    let mut resp = Json(UnsafeThreadIdsResponse { thread_ids }).into_response();
    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=5, max-age=5".parse().unwrap());
    resp
}
