use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn filesystem_service_enabled() -> bool {
    match env::var("ANYWHERE_DOOR_ENABLE_FILESYSTEM_SERVICE") {
        Ok(value) => matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => true,
    }
}

fn filesystem_scan_interval() -> Duration {
    match env::var("ANYWHERE_DOOR_FILESYSTEM_SCAN_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        Some(seconds) if seconds > 0 => Duration::from_secs(seconds),
        _ => Duration::from_secs(600),
    }
}

fn filesystem_output_path() -> String {
    match env::var("ANYWHERE_DOOR_FILESYSTEM_OUTPUT") {
        Ok(path) if !path.is_empty() => path,
        _ => "output/filesystem_scan.txt".to_string(),
    }
}

fn file_watcher_enabled() -> bool {
    match env::var("ANYWHERE_DOOR_ENABLE_OS_WATCHER") {
        Ok(value) => matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => true,
    }
}

fn file_watcher_output_path() -> String {
    match env::var("ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT") {
        Ok(path) if !path.is_empty() => path,
        Err(_) | Ok(_) => "output/file_event_metadata.ndjson".to_string(),
    }
}

fn start_filesystem_worker(
    stop_requested: Arc<AtomicBool>,
    scan_interval: Duration,
    output_path: String,
) {
    thread::spawn(move || {
        while !stop_requested.load(Ordering::SeqCst) {
            match crate::filesystem::filesystem::run_filesystem_service(&output_path) {
                Ok(files_scanned) => {
                    println!(
                        "Filesystem service completed scan with {} files.",
                        files_scanned
                    );
                }
                Err(err) => {
                    eprintln!("Filesystem service scan failed: {}", err);
                }
            }

            let mut elapsed = Duration::from_secs(0);
            while elapsed < scan_interval && !stop_requested.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));
                elapsed += Duration::from_secs(1);
            }
        }
    });
}

fn start_file_watcher_worker(stop_requested: Arc<AtomicBool>, output_path: String) {
    thread::spawn(move || {
        if let Err(err) = crate::filesystem::watcher::run_os_file_watcher(stop_requested, &output_path) {
            eprintln!("OS file watcher failed: {}", err);
        }
    });
}

fn get_probe_file_path() -> PathBuf {
    match env::var_os("ANYWHERE_DOOR_PROBE_FILE") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => PathBuf::from("output/sample.txt"),
    }
}

fn get_heartbeat_message() -> String {
    let epoch_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    format!("Orchestrator heartbeat epoch={epoch_seconds}")
}

pub fn start_orchestrator() {
    let probe_file_path = get_probe_file_path();

    if let Some(parent) = probe_file_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!(
                    "Unable to create probe directory {}: {}",
                    parent.display(),
                    err
                );
                return;
            }
        }
    }

    let mut output_file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&probe_file_path)
    {
        Ok(file) => file,
        Err(err) => {
            eprintln!(
                "Unable to open probe file at {}: {}",
                probe_file_path.display(),
                err
            );
            return;
        }
    };

    if let Err(err) = writeln!(output_file, "{}", get_heartbeat_message()) {
        eprintln!(
            "Unable to write heartbeat to {}: {}",
            probe_file_path.display(),
            err
        );
        return;
    }

    if let Err(err) = writeln!(output_file, "Probe file path: {}", probe_file_path.display()) {
        eprintln!(
            "Unable to write probe path to {}: {}",
            probe_file_path.display(),
            err
        );
    }
}

pub fn run_service() -> Result<(), String> {
    let stop_requested = Arc::new(AtomicBool::new(false));
    run_loop(stop_requested, true, true)
}

pub fn run_loop(
    stop_requested: Arc<AtomicBool>,
    install_signal_handler: bool,
    announce_lifecycle: bool,
) -> Result<(), String> {
    if install_signal_handler {
        let stop_requested_for_handler = Arc::clone(&stop_requested);
        ctrlc::set_handler(move || {
            stop_requested_for_handler.store(true, Ordering::SeqCst);
        })
        .map_err(|err| format!("Failed to register signal handler: {err}"))?;
    }

    if announce_lifecycle {
        println!("Anywhere Door agent service started. Press Ctrl+C to stop.");
    }

    let filesystem_enabled = filesystem_service_enabled();
    let filesystem_interval = filesystem_scan_interval();
    let filesystem_output = filesystem_output_path();
    let watcher_enabled = file_watcher_enabled();
    let watcher_output = file_watcher_output_path();

    if filesystem_enabled {
        start_filesystem_worker(
            Arc::clone(&stop_requested),
            filesystem_interval,
            filesystem_output,
        );
    }

    if watcher_enabled {
        start_file_watcher_worker(Arc::clone(&stop_requested), watcher_output);
    }

    while !stop_requested.load(Ordering::SeqCst) {
        start_orchestrator();
        thread::sleep(Duration::from_secs(2));
    }

    if announce_lifecycle {
        println!("Anywhere Door agent service stopped.");
    }

    Ok(())
}