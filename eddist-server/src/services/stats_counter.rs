use std::{
    collections::HashMap,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};

use crate::repositories::stats_repository::StatsRepository;

type BoardCounters = Arc<(AtomicI64, AtomicI64)>;

// Outer RwLock is write-locked only when a new board_key is first seen.
// Increment hot-path takes only a read lock and touches per-board atomics.
static BOARD_STATS: OnceLock<Arc<RwLock<HashMap<String, BoardCounters>>>> = OnceLock::new();

fn get_board_stats() -> &'static Arc<RwLock<HashMap<String, BoardCounters>>> {
    BOARD_STATS.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

fn get_or_insert_board(board_key: &str) -> BoardCounters {
    {
        let map = get_board_stats().read().unwrap();
        if let Some(entry) = map.get(board_key) {
            return Arc::clone(entry);
        }
    }
    let mut map = get_board_stats().write().unwrap();
    Arc::clone(
        map.entry(board_key.to_string())
            .or_insert_with(|| Arc::new((AtomicI64::new(0), AtomicI64::new(0)))),
    )
}

pub fn increment_board_response_delta(board_key: &str) {
    get_or_insert_board(board_key)
        .0
        .fetch_add(1, Ordering::Relaxed);
}

pub fn increment_board_thread_delta(board_key: &str) {
    get_or_insert_board(board_key)
        .1
        .fetch_add(1, Ordering::Relaxed);
}

pub async fn flush_stats_now(repo: &dyn StatsRepository) -> anyhow::Result<()> {
    let snapshot: Vec<(String, i64, i64)> = {
        let map = get_board_stats().read().unwrap();
        map.iter()
            .filter_map(|(key, counters)| {
                let r = counters.0.swap(0, Ordering::Relaxed);
                let t = counters.1.swap(0, Ordering::Relaxed);
                (r != 0 || t != 0).then(|| (key.clone(), r, t))
            })
            .collect()
    };

    if snapshot.is_empty() {
        return Ok(());
    }

    let result = repo.flush_board_stats(&snapshot).await;

    if result.is_err() {
        // Restore swapped-out deltas so they survive to the next flush cycle.
        let map = get_board_stats().read().unwrap();
        for (board_key, response_delta, thread_delta) in snapshot {
            if let Some(counters) = map.get(&board_key) {
                counters.0.fetch_add(response_delta, Ordering::Relaxed);
                counters.1.fetch_add(thread_delta, Ordering::Relaxed);
            }
        }
    }

    result
}

pub fn start_stats_flush_task<T: StatsRepository + Clone>(repo: T, interval: Duration) {
    tokio::spawn(async move {
        tracing::info!("Started stats flush task with interval: {interval:?}");
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let _ = flush_stats_now(&repo).await;
        }
    });
}
