use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};

use sqlx::MySqlPool;

pub struct StatsCounter {
    pub response_delta: AtomicI64,
    pub thread_delta: AtomicI64,
}

static GLOBAL_STATS_COUNTER: OnceLock<Arc<StatsCounter>> = OnceLock::new();

fn get_global_counter() -> &'static Arc<StatsCounter> {
    GLOBAL_STATS_COUNTER.get_or_init(|| {
        Arc::new(StatsCounter {
            response_delta: AtomicI64::new(0),
            thread_delta: AtomicI64::new(0),
        })
    })
}

pub fn increment_response_delta() {
    get_global_counter()
        .response_delta
        .fetch_add(1, Ordering::Relaxed);
}

pub fn increment_thread_delta() {
    get_global_counter()
        .thread_delta
        .fetch_add(1, Ordering::Relaxed);
}

pub async fn flush_stats_now(pool: &MySqlPool) -> anyhow::Result<()> {
    let counter = get_global_counter();
    let delta_r = counter.response_delta.load(Ordering::Relaxed);
    let delta_t = counter.thread_delta.load(Ordering::Relaxed);

    if delta_r == 0 && delta_t == 0 {
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO daily_stats (date, total_responses, new_threads) \
         VALUES (DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00')), ?, ?) \
         ON DUPLICATE KEY UPDATE \
         total_responses = total_responses + VALUES(total_responses), \
         new_threads = new_threads + VALUES(new_threads)",
        delta_r,
        delta_t,
    )
    .execute(pool)
    .await?;

    counter.response_delta.fetch_sub(delta_r, Ordering::Relaxed);
    counter.thread_delta.fetch_sub(delta_t, Ordering::Relaxed);

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
