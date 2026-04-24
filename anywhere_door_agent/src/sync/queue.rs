//! In-memory priority queue deduped by file path.
//!
//! When a new [`SyncEvent`] arrives for a path that is already queued, the
//! existing entry is replaced with the latest event (last-writer-wins).

use super::debounce::SyncEvent;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;

/// Thread-safe, path-deduped event queue.
pub struct SyncQueue {
    inner: Mutex<HashMap<PathBuf, SyncEvent>>,
}

impl SyncQueue {
    pub fn new() -> Self {
        SyncQueue {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Push an event into the queue. If the path is already present, the entry
    /// is replaced with the newer event.
    pub async fn push(&self, event: SyncEvent) {
        let mut map = self.inner.lock().await;
        map.insert(event.path.clone(), event);
    }

    /// Pop up to `n` events from the queue for processing.
    pub async fn pop_batch(&self, n: usize) -> Vec<SyncEvent> {
        let mut map = self.inner.lock().await;
        let keys: Vec<PathBuf> = map.keys().take(n).cloned().collect();
        let mut batch = Vec::with_capacity(keys.len());

        for key in keys {
            if let Some(event) = map.remove(&key) {
                batch.push(event);
            }
        }

        batch
    }

    /// Current number of events waiting in the queue.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Whether the queue is empty.
    #[allow(dead_code)]
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }
}
