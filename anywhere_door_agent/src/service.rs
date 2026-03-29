use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn file_watcher_enabled() -> bool {
    match env::var("ANYWHERE_DOOR_ENABLE_OS_WATCHER") {
        Ok(value) => matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => true,
    }
}

fn file_watcher_output_path() -> String {
    match env::var("ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT") {
        Ok(path) if !path.is_empty() => path,
        _ => "output/file_event_metadata.ndjson".to_string(),
    }
}

fn start_file_watcher_worker(stop_requested: Arc<AtomicBool>, output_path: String) {
    thread::spawn(move || {
        if let Err(err) = crate::filesystem::watcher::run_os_file_watcher(stop_requested, &output_path) {
            eprintln!("OS file watcher failed: {}", err);
        }
    });
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

    // let filesystem_enabled = filesystem_service_enabled();
    // let filesystem_interval = filesystem_scan_interval();
    // let filesystem_output = filesystem_output_path();
    let watcher_enabled = file_watcher_enabled();
    let watcher_output = file_watcher_output_path();

    // if filesystem_enabled {
    //     start_filesystem_worker(
    //         Arc::clone(&stop_requested),
    //         filesystem_interval,
    //         filesystem_output,
    //     );
    // }

    if watcher_enabled {
        start_file_watcher_worker(Arc::clone(&stop_requested), watcher_output);
    }

    // Main loop: keep the service alive while watcher thread runs in background
    while !stop_requested.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(1));
    }

    if announce_lifecycle {
        println!("Anywhere Door agent service stopped.");
    }

    Ok(())
}