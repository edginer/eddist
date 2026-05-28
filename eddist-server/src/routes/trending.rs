use std::collections::{HashMap, HashSet};

use axum::{Json, extract::State, http::HeaderValue, response::IntoResponse};
use eddist_core::redis_keys::{TRENDING_THREADS_CACHE_KEY, unsafe_threads_key};
use http::StatusCode;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    repositories::trending_repository::TrendingRepository,
    services::server_settings_cache::{ServerSettingKey, get_server_setting_bool},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct TrendingThreadResponse {
    pub board_key: String,
    pub board_name: String,
    pub thread_number: i64,
    pub title: String,
    pub response_count: i32,
    pub recent_response_count: i64,
}

#[derive(Serialize, Deserialize)]
pub struct TrendingResponse {
    pub threads: Vec<TrendingThreadResponse>,
}

pub async fn get_trending(State(state): State<AppState>) -> impl IntoResponse {
    let mut redis_conn = state.redis_conn.clone();

    if let Ok(cached) = redis_conn
        .get::<_, Option<String>>(TRENDING_THREADS_CACHE_KEY)
        .await
    {
        if let Some(json) = cached {
            if let Ok(threads) = serde_json::from_str::<Vec<TrendingThreadResponse>>(&json) {
                let mut resp = Json(TrendingResponse { threads }).into_response();
                resp.headers_mut()
                    .insert("Cache-Control", HeaderValue::from_static("s-maxage=60"));
                return resp;
            }
        }
    }

    let exclude_flagged = get_server_setting_bool(ServerSettingKey::AiModerationOnRes).await
        || get_server_setting_bool(ServerSettingKey::AiModerationOnThread).await;

    let candidates = match state.trending_repo.get_trending_threads(6, 20).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to fetch trending threads: {e:?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }
    };

    let threads = if exclude_flagged {
        let board_ids: Vec<String> = candidates
            .iter()
            .map(|t| t.board_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut unsafe_by_board: HashMap<String, HashSet<i64>> = HashMap::new();
        for board_id in board_ids {
            let key = unsafe_threads_key(&board_id);
            let unsafe_ids: Vec<i64> = redis_conn.smembers(&key).await.unwrap_or_default();
            unsafe_by_board.insert(board_id, unsafe_ids.into_iter().collect());
        }

        candidates
            .into_iter()
            .filter(|t| {
                unsafe_by_board
                    .get(&t.board_id)
                    .map(|set| !set.contains(&t.thread_number))
                    .unwrap_or(true)
            })
            .take(10)
            .map(|t| TrendingThreadResponse {
                board_key: t.board_key,
                board_name: t.board_name,
                thread_number: t.thread_number,
                title: t.title,
                response_count: t.response_count,
                recent_response_count: t.recent_response_count,
            })
            .collect::<Vec<_>>()
    } else {
        candidates
            .into_iter()
            .take(10)
            .map(|t| TrendingThreadResponse {
                board_key: t.board_key,
                board_name: t.board_name,
                thread_number: t.thread_number,
                title: t.title,
                response_count: t.response_count,
                recent_response_count: t.recent_response_count,
            })
            .collect::<Vec<_>>()
    };

    if let Ok(serialized) = serde_json::to_string(&threads) {
        let _ = redis_conn
            .set_ex::<_, _, ()>(TRENDING_THREADS_CACHE_KEY, serialized, 300)
            .await;
    }

    let mut resp = Json(TrendingResponse { threads }).into_response();
    resp.headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("s-maxage=60"));
    resp
}
