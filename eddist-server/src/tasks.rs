use std::{
    any::Any,
    future::Future,
    panic::AssertUnwindSafe,
    time::{Duration, Instant},
};

use futures::FutureExt as _;
use metrics::counter;
use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;
use tracing::{error, info, warn};

/// Request-scoped tasks that shutdown waits for before the process exits.
#[derive(Clone, Default)]
pub struct BackgroundTasks(TaskTracker);

impl BackgroundTasks {
    pub(crate) fn spawn<F, T>(&self, name: &'static str, future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.0.spawn(run_guarded(name, future))
    }

    pub(crate) fn spawn_detached<F, T>(&self, name: &'static str, future: F)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        drop(self.spawn(name, future));
    }

    pub(crate) async fn drain(&self, deadline: Duration) {
        self.0.close();
        let started = Instant::now();
        match tokio::time::timeout(deadline, self.0.wait()).await {
            Ok(()) => info!(
                elapsed_ms = started.elapsed().as_millis(),
                "Posting-related tasks finished before shutdown"
            ),
            Err(_) => warn!(
                remaining = self.0.len(),
                ?deadline,
                "Posting-related tasks did not finish before the shutdown deadline"
            ),
        }
    }
}

/// Re-raises the panic after logging so the JoinHandle keeps the usual panic semantics.
async fn run_guarded<F, T>(name: &'static str, future: F) -> T
where
    F: Future<Output = T>,
{
    match AssertUnwindSafe(future).catch_unwind().await {
        Ok(output) => output,
        Err(payload) => {
            error!(
                task = name,
                panic = %panic_message(&*payload),
                "background task panicked"
            );
            counter!("background_task_panic", "task" => name).increment(1);
            std::panic::resume_unwind(payload);
        }
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.as_str()
    } else {
        "non-string panic payload"
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use metrics_util::debugging::{DebugValue, DebuggingRecorder};
    use tokio::sync::oneshot;

    use super::*;

    #[tokio::test]
    async fn drain_waits_for_detached_tasks() {
        let tasks = BackgroundTasks::default();
        let (release, wait) = oneshot::channel::<()>();
        tasks.spawn_detached("waiter", async move {
            let _ = wait.await;
        });
        assert_eq!(tasks.0.len(), 1);

        release.send(()).unwrap();
        tasks.drain(Duration::from_secs(1)).await;
        assert_eq!(tasks.0.len(), 0);
    }

    #[tokio::test]
    async fn drain_returns_at_the_deadline_without_aborting_tasks() {
        let tasks = BackgroundTasks::default();
        let (release, wait) = oneshot::channel::<()>();
        let (done_tx, done_rx) = oneshot::channel::<()>();
        tasks.spawn_detached("slow", async move {
            let _ = wait.await;
            let _ = done_tx.send(());
        });

        let started = Instant::now();
        tasks.drain(Duration::from_millis(50)).await;
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(tasks.0.len(), 1);

        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), done_rx)
            .await
            .expect("task should keep running after the deadline")
            .unwrap();
    }

    #[tokio::test]
    async fn tasks_spawned_after_the_drain_starts_are_waited_for() {
        let tasks = BackgroundTasks::default();
        let (release, wait) = oneshot::channel::<()>();
        tasks.spawn_detached("first", async move {
            let _ = wait.await;
        });

        let drain = {
            let tasks = tasks.clone();
            tokio::spawn(async move { tasks.drain(Duration::from_secs(1)).await })
        };
        tokio::task::yield_now().await;
        let finished = Arc::new(AtomicBool::new(false));
        let flag = finished.clone();
        tasks.spawn_detached("late", async move {
            flag.store(true, Ordering::SeqCst);
        });
        release.send(()).unwrap();
        drain.await.unwrap();
        assert!(finished.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn panicking_tasks_do_not_block_the_drain() {
        let tasks = BackgroundTasks::default();
        tasks.spawn_detached("panicker", async { panic!("boom") });
        tasks.drain(Duration::from_secs(1)).await;
        assert_eq!(tasks.0.len(), 0);
    }

    // Asserts the counter rather than the log: tracing caches callsite interest
    // process-wide, so a sibling test hitting this callsite without a subscriber races it.
    #[test]
    fn run_guarded_counts_panics_with_the_task_name() {
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        let result = metrics::with_local_recorder(&recorder, || {
            futures::executor::block_on(
                AssertUnwindSafe(run_guarded("panicker", async { panic!("boom") })).catch_unwind(),
            )
        });

        assert!(result.is_err(), "the panic must be re-raised");
        let counted = snapshotter
            .snapshot()
            .into_vec()
            .into_iter()
            .any(|(key, _, _, value)| {
                let key = key.key();
                key.name() == "background_task_panic"
                    && key
                        .labels()
                        .any(|label| label.key() == "task" && label.value() == "panicker")
                    && value == DebugValue::Counter(1)
            });
        assert!(counted);
    }

    #[test]
    fn panic_message_extracts_string_payloads() {
        let payload: Box<dyn Any + Send> = Box::new("static message");
        assert_eq!(panic_message(&*payload), "static message");
        let payload: Box<dyn Any + Send> = Box::new(String::from("owned message"));
        assert_eq!(panic_message(&*payload), "owned message");
    }
}
