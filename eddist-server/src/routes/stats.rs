use std::collections::HashMap;

use axum::{Json, extract::State, http::HeaderValue, response::IntoResponse};
use http::StatusCode;
use serde::Serialize;

use crate::{app::AppState, repositories::stats_repository::StatsRepository};

#[derive(Serialize)]
pub struct TodayBoardStats {
    pub total_responses: i64,
    pub new_threads: i64,
}

#[derive(Serialize)]
pub struct DailyBoardStat {
    pub date: String,
    pub total_responses: i64,
    pub new_threads: i64,
}

#[derive(Serialize)]
pub struct BoardStats {
    pub today: TodayBoardStats,
    pub history: Vec<DailyBoardStat>,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub boards: HashMap<String, BoardStats>,
}

pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    let (today_rows, history_rows) = match tokio::try_join!(
        state.stats_repo.get_today_stats_per_board(),
        state.stats_repo.get_daily_stats_per_board(30),
    ) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to fetch stats: {e:?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }
    };

    let mut boards: HashMap<String, BoardStats> = HashMap::new();

    for row in today_rows {
        let entry = boards.entry(row.board_key).or_insert_with(|| BoardStats {
            today: TodayBoardStats {
                total_responses: 0,
                new_threads: 0,
            },
            history: vec![],
        });
        entry.today.total_responses = row.total_responses + row.new_threads;
        entry.today.new_threads = row.new_threads;
    }

    for row in history_rows {
        let entry = boards.entry(row.board_key).or_insert_with(|| BoardStats {
            today: TodayBoardStats {
                total_responses: 0,
                new_threads: 0,
            },
            history: vec![],
        });
        entry.history.push(DailyBoardStat {
            date: row.date.to_string(),
            total_responses: row.total_responses + row.new_threads,
            new_threads: row.new_threads,
        });
    }

    let mut resp = Json(StatsResponse { boards }).into_response();
    resp.headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("s-maxage=30"));
    resp
}
