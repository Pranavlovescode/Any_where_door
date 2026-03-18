use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

    while !stop_requested.load(Ordering::SeqCst) {
        start_orchestrator();
        thread::sleep(Duration::from_secs(2));
    }

    if announce_lifecycle {
        println!("Anywhere Door agent service stopped.");
    }

    Ok(())
}