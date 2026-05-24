use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize)]
pub struct TodayStatsResponse {
    pub total_responses: i64,
    pub new_threads: i64,
}

#[derive(Serialize)]
pub struct DailyStatResponse {
    pub date: String,
    pub total_responses: i64,
    pub new_threads: i64,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub today: TodayStatsResponse,
    pub history: Vec<DailyStatResponse>,
}

pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    let (today_stat, history) = match tokio::try_join!(
        state.stats_repo.get_today_stat(),
        state.stats_repo.get_daily_stats(30),
    ) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to fetch stats: {e:?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
        }
    };

    let today = TodayStatsResponse {
        total_responses: today_stat.as_ref().map(|s| s.total_responses).unwrap_or(0),
        new_threads: today_stat.as_ref().map(|s| s.new_threads).unwrap_or(0),
    };

    let history = history
        .into_iter()
        .map(|s| DailyStatResponse {
            date: s.date.to_string(),
            total_responses: s.total_responses,
            new_threads: s.new_threads,
        })
        .collect::<Vec<_>>();

    let mut resp = Json(StatsResponse { today, history }).into_response();
    resp.headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("s-maxage=30"));
    resp
}
