//! Pipeline orchestrator — wires together debounce → queue → upload workers.

use super::debounce;
use super::queue::SyncQueue;
use super::uploader;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

/// Handle returned by [`start_pipeline`].  Provides the channel sender for the
/// watcher to push raw events into, and a method to shut the pipeline down
/// gracefully.
pub struct PipelineHandle {
    /// Send raw `notify::Event` items here (from the file watcher thread).
    pub event_tx: mpsc::UnboundedSender<notify::Event>,
    /// Signal shutdown.
    stop_tx: watch::Sender<bool>,
    /// Worker join handles.
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl PipelineHandle {
    /// Signal all pipeline tasks to stop and wait for them to finish.
    pub async fn shutdown(self) {
        let _ = self.stop_tx.send(true);
        for handle in self.handles {
            let _ = handle.await;
        }
        eprintln!("Sync: pipeline shut down");
    }
}

/// Start the full sync pipeline.
///
/// * `credentials_path` – path to the `.anywheredoor` credentials file.
///
/// Returns a [`PipelineHandle`] whose `event_tx` should be passed to the file
/// watcher so it can feed raw events into the pipeline.
pub fn start_pipeline(credentials_path: PathBuf) -> PipelineHandle {
    // Channels
    let (raw_tx, raw_rx) = mpsc::unbounded_channel::<notify::Event>();
    let (stable_tx, mut stable_rx) = mpsc::unbounded_channel();
    let (stop_tx, stop_rx) = watch::channel(false);

    let queue = Arc::new(SyncQueue::new());

    // --- Task 1: debounce -------------------------------------------------
    let debounce_stop = stop_rx.clone();
    let debounce_handle = tokio::spawn(async move {
        debounce::run_debounce(raw_rx, stable_tx, debounce_stop).await;
    });

    // --- Task 2: queue feeder (stable events → queue) ---------------------
    let queue_for_feeder = Arc::clone(&queue);
    let mut feeder_stop = stop_rx.clone();
    let feeder_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe_event = stable_rx.recv() => {
                    match maybe_event {
                        Some(event) => {
                            queue_for_feeder.push(event).await;
                        }
                        None => break,
                    }
                }
                _ = feeder_stop.changed() => { break; }
            }
        }
    });

    // --- Task 3: upload workers -------------------------------------------
    let worker_handles = uploader::spawn_workers(
        Arc::clone(&queue),
        stop_rx.clone(),
        credentials_path.clone(),
    );

    // --- Task 4: queue depth logger (every 30s) ---------------------------
    let queue_for_logger = Arc::clone(&queue);
    let mut logger_stop = stop_rx.clone();
    let logger_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let depth = queue_for_logger.len().await;
                    if depth > 0 {
                        eprintln!("Sync: queue depth = {}", depth);
                    }
                }
                _ = logger_stop.changed() => { break; }
            }
        }
    });

    // --- Task 5: WebSocket listener (Bidirectional Sync) ------------------
    let ws_handle = crate::net::spawn_websocket_listener(
        stop_rx.clone(),
        credentials_path.clone(),
    );

    // Collect all handles
    let mut handles = vec![debounce_handle, feeder_handle, logger_handle, ws_handle];
    handles.extend(worker_handles);

    eprintln!("Sync: pipeline started");

    PipelineHandle {
        event_tx: raw_tx,
        stop_tx,
        handles,
    }
}
