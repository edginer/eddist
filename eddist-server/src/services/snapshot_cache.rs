use std::{
    future::Future,
    sync::{Arc, RwLock},
    time::Duration,
};

use tokio::task::JoinHandle;

/// Readers get an `Arc` of the whole snapshot so related values come from one load.
#[derive(Debug)]
pub struct SnapshotCache<T>(Arc<RwLock<Arc<T>>>);

impl<T> Clone for SnapshotCache<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> SnapshotCache<T> {
    pub fn new(initial: T) -> Self {
        Self(Arc::new(RwLock::new(Arc::new(initial))))
    }

    pub fn load(&self) -> Arc<T> {
        self.0
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn replace(&self, snapshot: T) {
        *self
            .0
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(snapshot);
    }
}

/// The first refresh runs one `period` after spawning: callers load the initial snapshot
/// themselves so that startup can fail on it.
pub(crate) fn spawn_refresh_task<F, Fut>(
    name: &'static str,
    period: Duration,
    mut refresh: F,
) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    tracing::info!(cache = name, ?period, "Started cache refresh task");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
        loop {
            interval.tick().await;
            if let Err(e) = refresh().await {
                tracing::error!(
                    cache = name,
                    error = %format_args!("{e:#}"),
                    "Failed to refresh cache; keeping the previous snapshot"
                );
            }
        }
    })
}
