use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc as tokio_mpsc;

#[derive(Serialize)]
struct PathMetadata {
    path: String,
    exists: bool,
    is_dir: bool,
    size_bytes: Option<u64>,
    modified_epoch_ms: Option<u128>,
}

#[derive(Serialize)]
struct FileEventMetadata {
    timestamp_epoch_ms: u128,
    event_kind: String,
    paths: Vec<PathMetadata>,
}

fn watch_roots() -> Vec<PathBuf> {
    // Try to read configured watch roots from environment
    if let Ok(configured_roots) = env::var("ANYWHERE_DOOR_WATCH_ROOTS") {
        // Parse paths separated by either comma or semicolon
        // This allows flexibility: Windows (C:\path;D:\path) or Linux (/home/user,/var/log)
        let separator = if configured_roots.contains(';') { ';' } else { ',' };

        let roots = configured_roots
            .split(separator)
            .filter(|part| !part.trim().is_empty())
            .map(|part| PathBuf::from(part.trim()))
            .collect::<Vec<_>>();

        if !roots.is_empty() {
            return roots;
        }
    }

    // Default watch roots based on platform
    #[cfg(windows)]
    {
        // On Windows, auto-detect available drives (C:, D:, E:, etc.)
        let mut roots = Vec::new();
        for letter in 'A'..='Z' {
            let drive = format!("{}:\\", letter);
            let drive_path = PathBuf::from(&drive);
            if drive_path.exists() {
                roots.push(drive_path);
            }
        }
        roots
    }

    #[cfg(not(windows))]
    {
        // On Linux/Unix, watch from root unless configured otherwise
        vec![PathBuf::from("/")]
    }
}

fn ensure_parent_dir(path: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create watcher output directory: {err}"))?;
        }
    }

    Ok(())
}

fn to_epoch_ms(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn path_metadata(path: &Path) -> PathMetadata {
    match fs::metadata(path) {
        Ok(meta) => {
            let modified_epoch_ms = meta.modified().ok().map(to_epoch_ms);
            PathMetadata {
                path: path.display().to_string(),
                exists: true,
                is_dir: meta.is_dir(),
                size_bytes: if meta.is_file() { Some(meta.len()) } else { None },
                modified_epoch_ms,
            }
        }
        Err(_) => PathMetadata {
            path: path.display().to_string(),
            exists: false,
            is_dir: false,
            size_bytes: None,
            modified_epoch_ms: None,
        },
    }
}

fn event_kind_to_string(kind: &EventKind) -> String {
    match kind {
        EventKind::Create(_) => "create".to_string(),
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => "rename_from".to_string(),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => "rename_to".to_string(),
        EventKind::Modify(ModifyKind::Name(_)) => "rename".to_string(),
        EventKind::Modify(_) => "modify".to_string(),
        EventKind::Remove(_) => "remove".to_string(),
        EventKind::Access(_) => "access".to_string(),
        EventKind::Any => "any".to_string(),
        EventKind::Other => "other".to_string(),
    }
}

fn event_to_metadata(event: Event) -> FileEventMetadata {
    let paths = event
        .paths
        .iter()
        .map(|path| path_metadata(path))
        .collect::<Vec<_>>();

    FileEventMetadata {
        timestamp_epoch_ms: to_epoch_ms(SystemTime::now()),
        event_kind: event_kind_to_string(&event.kind),
        paths,
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left_real), Ok(right_real)) => left_real == right_real,
        _ => false,
    }
}

pub fn run_os_file_watcher(
    stop_requested: Arc<AtomicBool>,
    output_path: &str,
    sync_tx: Option<tokio_mpsc::UnboundedSender<Event>>,
) -> Result<(), String> {
    ensure_parent_dir(output_path)?;

    let mut output = OpenOptions::new()
        .append(true)
        .create(true)
        .open(output_path)
        .map_err(|err| format!("Failed to open watcher metadata output file: {err}"))?;
    let output_metadata_path = PathBuf::from(output_path);

    let (event_tx, event_rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |result| {
            let _ = event_tx.send(result);
        },
        Config::default(),
    )
    .map_err(|err| format!("Failed to initialize OS file watcher: {err}"))?;

    let roots = watch_roots();
    if roots.is_empty() {
        return Err("No watch roots found for this operating system.".to_string());
    }

    let mut watched_roots = 0usize;
    for root in roots {
        if watcher.watch(&root, RecursiveMode::Recursive).is_ok() {
            watched_roots += 1;
        }
    }

    if watched_roots == 0 {
        #[cfg(windows)]
        {
            return Err(
                "Failed to watch any filesystem root path. Ensure drives are accessible and ANYWHERE_DOOR_WATCH_ROOTS is set correctly (use semicolon separator: C:\\Users;D:\\Data). You may need to run with elevated privileges."
                    .to_string(),
            );
        }

        #[cfg(not(windows))]
        {
            return Err(
                "Failed to watch any filesystem root path. This usually means insufficient permissions. Set ANYWHERE_DOOR_WATCH_ROOTS to accessible directories (use comma separator: /home/user,/var/log), or run with higher privileges."
                    .to_string(),
            );
        }
    }

    eprintln!("OS file watcher initialized successfully, watching {} root path(s)", watched_roots);

    while !stop_requested.load(Ordering::SeqCst) {
        match event_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(mut event)) => {
                event.paths.retain(|path| !same_path(path, &output_metadata_path));
                if event.paths.is_empty() {
                    continue;
                }

                // Forward to sync pipeline (if connected)
                if let Some(ref tx) = sync_tx {
                    let _ = tx.send(event.clone());
                }

                // Write to local NDJSON log
                let metadata = event_to_metadata(event);
                let line = serde_json::to_string(&metadata)
                    .map_err(|err| format!("Failed to serialize watcher metadata: {err}"))?;
                writeln!(output, "{}", line)
                    .map_err(|err| format!("Failed to write watcher metadata: {err}"))?;
                output
                    .flush()
                    .map_err(|err| format!("Failed to flush watcher metadata: {err}"))?;
            }
            Ok(Err(err)) => {
                eprintln!("OS watcher backend error: {}", err);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err("OS file watcher event channel disconnected.".to_string());
            }
        }
    }

    Ok(())
}
