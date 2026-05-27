use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use sqlx::MySqlPool;

static BOARD_STATS: OnceLock<Arc<Mutex<HashMap<String, (i64, i64)>>>> = OnceLock::new();

fn get_board_stats() -> &'static Arc<Mutex<HashMap<String, (i64, i64)>>> {
    BOARD_STATS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

pub fn increment_board_response_delta(board_key: &str) {
    let mut map = get_board_stats().lock().unwrap();
    let entry = map.entry(board_key.to_string()).or_insert((0, 0));
    entry.0 += 1;
}

pub fn increment_board_thread_delta(board_key: &str) {
    let mut map = get_board_stats().lock().unwrap();
    let entry = map.entry(board_key.to_string()).or_insert((0, 0));
    entry.1 += 1;
}

pub async fn flush_stats_now(pool: &MySqlPool) -> anyhow::Result<()> {
    let deltas: HashMap<String, (i64, i64)> = {
        let mut map = get_board_stats().lock().unwrap();
        std::mem::take(&mut *map)
    };

    for (board_key, (response_delta, thread_delta)) in deltas {
        if response_delta == 0 && thread_delta == 0 {
            continue;
        }
        sqlx::query!(
            "INSERT INTO daily_stats (date, board_key, total_responses, new_threads) \
             VALUES (DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00')), ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             total_responses = total_responses + VALUES(total_responses), \
             new_threads = new_threads + VALUES(new_threads)",
            board_key,
            response_delta,
            thread_delta,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub fn start_stats_flush_task(pool: MySqlPool, interval: Duration) {
    tokio::spawn(async move {
        tracing::info!("Started stats flush task with interval: {interval:?}");
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let _ = flush_stats_now(&pool).await;
        }
    });
}
