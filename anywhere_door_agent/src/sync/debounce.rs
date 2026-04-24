//! Debounce buffer for filesystem events.
//!
//! Collects raw `notify::Event` items and collapses rapid bursts on the same
//! path into a single stable [`SyncEvent`] after a configurable quiet period
//! (default 2 seconds).

use notify::event::{EventKind, ModifyKind, RenameMode};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

// ============================================================================
// Public types
// ============================================================================

/// The kind of sync-relevant filesystem change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SyncEventKind {
    Create,
    Modify,
    Remove,
    Rename { from: PathBuf },
}

/// A debounced, stable filesystem event ready for the upload queue.
#[derive(Debug, Clone, Serialize)]
pub struct SyncEvent {
    pub path: PathBuf,
    pub event_kind: SyncEventKind,
    pub timestamp_ms: u128,
    pub size_bytes: Option<u64>,
}

// ============================================================================
// Internal bookkeeping
// ============================================================================

/// Tracks the latest state of a path while it is inside the debounce window.
struct PendingEntry {
    kind: SyncEventKind,
    /// When we *first* saw activity on this path in the current window.
    first_seen: Instant,
    /// System-clock timestamp of the most recent event (for the output).
    latest_timestamp_ms: u128,
    size_bytes: Option<u64>,
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn file_size(path: &PathBuf) -> Option<u64> {
    std::fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| m.len())
}

// ============================================================================
// Collapse logic
// ============================================================================

/// Given an existing pending entry and a new incoming event kind, produce the
/// collapsed kind (or `None` to discard the entry entirely).
fn collapse(existing: &SyncEventKind, incoming: &SyncEventKind) -> Option<SyncEventKind> {
    match (existing, incoming) {
        // create + modify → still a create
        (SyncEventKind::Create, SyncEventKind::Modify) => Some(SyncEventKind::Create),

        // create + remove → ephemeral, discard
        (SyncEventKind::Create, SyncEventKind::Remove) => None,

        // modify + modify → single modify
        (SyncEventKind::Modify, SyncEventKind::Modify) => Some(SyncEventKind::Modify),

        // anything + remove → remove wins
        (_, SyncEventKind::Remove) => Some(SyncEventKind::Remove),

        // remove + create → treat as modify (file replaced)
        (SyncEventKind::Remove, SyncEventKind::Create) => Some(SyncEventKind::Modify),

        // otherwise keep the incoming kind
        (_, new) => Some(new.clone()),
    }
}

// ============================================================================
// Debounce task
// ============================================================================

/// Quiet period: an entry is flushed when it hasn't received new events for
/// this duration.
const DEBOUNCE_QUIET_MS: u64 = 2_000;

/// Tick interval for the flush check.
const TICK_INTERVAL_MS: u64 = 500;

/// Runs the debounce loop.
///
/// * `raw_rx`   – receives raw `notify::Event` from the watcher thread.
/// * `stable_tx` – sends stable [`SyncEvent`] downstream (to the queue).
/// * `stop`     – checked each tick; when `true` the task exits.
pub async fn run_debounce(
    mut raw_rx: mpsc::UnboundedReceiver<notify::Event>,
    stable_tx: mpsc::UnboundedSender<SyncEvent>,
    stop: tokio::sync::watch::Receiver<bool>,
) {
    let quiet_period = Duration::from_millis(DEBOUNCE_QUIET_MS);
    let tick = Duration::from_millis(TICK_INTERVAL_MS);

    let mut pending: HashMap<PathBuf, PendingEntry> = HashMap::new();
    let mut interval = tokio::time::interval(tick);

    loop {
        tokio::select! {
            // ---- Incoming raw event -------------------------------------------
            maybe_event = raw_rx.recv() => {
                let event = match maybe_event {
                    Some(e) => e,
                    None => break, // channel closed
                };

                let kind = match classify(&event.kind) {
                    Some(k) => k,
                    None => continue, // access / other → skip
                };

                for path in &event.paths {
                    // Skip directories — we only sync files
                    if path.is_dir() {
                        continue;
                    }

                    let now = Instant::now();
                    let ts = now_epoch_ms();
                    let sz = file_size(path);

                    if let Some(entry) = pending.get_mut(path) {
                        // Collapse with existing entry
                        match collapse(&entry.kind, &kind) {
                            Some(collapsed) => {
                                entry.kind = collapsed;
                                entry.latest_timestamp_ms = ts;
                                entry.size_bytes = sz;
                                // Do NOT reset first_seen — we want the
                                // quiet-period to be measured from the *last*
                                // update, so overwrite first_seen here to
                                // extend the window.
                                entry.first_seen = now;
                            }
                            None => {
                                // Discard (e.g. create+remove = ephemeral)
                                pending.remove(path);
                            }
                        }
                    } else {
                        pending.insert(path.clone(), PendingEntry {
                            kind,
                            first_seen: now,
                            latest_timestamp_ms: ts,
                            size_bytes: sz,
                        });
                    }
                    // Only process the first matching kind per event
                    break;
                }
            }

            // ---- Periodic flush -----------------------------------------------
            _ = interval.tick() => {
                if *stop.borrow() {
                    // Flush everything remaining before exit
                    flush_all(&mut pending, &stable_tx);
                    break;
                }

                let now = Instant::now();
                let mut to_flush: Vec<PathBuf> = Vec::new();

                for (path, entry) in pending.iter() {
                    if now.duration_since(entry.first_seen) >= quiet_period {
                        to_flush.push(path.clone());
                    }
                }

                for path in to_flush {
                    if let Some(entry) = pending.remove(&path) {
                        let _ = stable_tx.send(SyncEvent {
                            path,
                            event_kind: entry.kind,
                            timestamp_ms: entry.latest_timestamp_ms,
                            size_bytes: entry.size_bytes,
                        });
                    }
                }
            }
        }
    }
}

/// Flush all remaining entries (used on shutdown).
fn flush_all(
    pending: &mut HashMap<PathBuf, PendingEntry>,
    tx: &mpsc::UnboundedSender<SyncEvent>,
) {
    for (path, entry) in pending.drain() {
        let _ = tx.send(SyncEvent {
            path,
            event_kind: entry.kind,
            timestamp_ms: entry.latest_timestamp_ms,
            size_bytes: entry.size_bytes,
        });
    }
}

/// Map a `notify::EventKind` to our simplified `SyncEventKind`, or `None` to
/// skip events we don't care about (access, other, any).
fn classify(kind: &EventKind) -> Option<SyncEventKind> {
    match kind {
        EventKind::Create(_) => Some(SyncEventKind::Create),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Some(SyncEventKind::Create),
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => Some(SyncEventKind::Remove),
        EventKind::Modify(ModifyKind::Name(_)) => Some(SyncEventKind::Modify),
        EventKind::Modify(_) => Some(SyncEventKind::Modify),
        EventKind::Remove(_) => Some(SyncEventKind::Remove),
        // Access, Any, Other → not sync-relevant
        _ => None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collapse_create_modify() {
        let result = collapse(&SyncEventKind::Create, &SyncEventKind::Modify);
        assert_eq!(result, Some(SyncEventKind::Create));
    }

    #[test]
    fn test_collapse_create_remove_ephemeral() {
        let result = collapse(&SyncEventKind::Create, &SyncEventKind::Remove);
        assert_eq!(result, None);
    }

    #[test]
    fn test_collapse_modify_remove() {
        let result = collapse(&SyncEventKind::Modify, &SyncEventKind::Remove);
        assert_eq!(result, Some(SyncEventKind::Remove));
    }

    #[test]
    fn test_collapse_remove_create() {
        let result = collapse(&SyncEventKind::Remove, &SyncEventKind::Create);
        assert_eq!(result, Some(SyncEventKind::Modify));
    }

    #[test]
    fn test_collapse_modify_modify() {
        let result = collapse(&SyncEventKind::Modify, &SyncEventKind::Modify);
        assert_eq!(result, Some(SyncEventKind::Modify));
    }
}
