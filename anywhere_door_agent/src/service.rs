use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Configuration
// ============================================================================

/// Get the path to the device credentials file
fn get_credentials_path() -> PathBuf {
    match env::var("ANYWHERE_DOOR_CREDENTIALS_PATH") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".anywheredoor")
        }
    }
}

/// Get the path to the watch config file
fn get_watch_config_path() -> PathBuf {
    match env::var("ANYWHERE_DOOR_CONFIG_PATH") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".anywheredoor_watch_roots")
        }
    }
}

/// Get the server URL from environment or use default
fn get_server_url() -> String {
    match env::var("ANYWHERE_DOOR_SERVER_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => "http://127.0.0.1:8000".to_string(),
    }
}

// ============================================================================
// Interactive Setup Functions
// ============================================================================

/// Prompt user for input
fn prompt(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

/// Prompt user for password (hidden input)
fn prompt_password(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap();
    rpassword::read_password().unwrap_or_default()
}

/// Interactive login - prompts for username/password and gets JWT from backend
async fn interactive_login(server_url: &str) -> Result<String, String> {
    println!("\n========================================");
    println!("User Login");
    println!("========================================");
    
    let username = prompt("Enter username: ");
    let password = prompt_password("Enter password: ");
    
    // Call backend login endpoint
    let client = reqwest::Client::new();
    let login_payload = serde_json::json!({
        "username": username,
        "password": password
    });
    
    let response = client
        .post(&format!("{}/auth/login", server_url))
        .json(&login_payload)
        .send()
        .await
        .map_err(|e| format!("Failed to login: {}", e))?;
    
    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse login response: {}", e))?;
    
    // Extract JWT from response
    let jwt = body
        .get("jwt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            let detail = body
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            format!("Login failed: {}", detail)
        })?;
    
    println!("✓ Login successful!");
    Ok(jwt.to_string())
}

/// Show menu to select directories to watch
fn select_watch_directories() -> Result<String, String> {
    println!("\n========================================");
    println!("Select Directories to Watch");
    println!("========================================");
    
    let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    
    println!("\nOptions:");
    println!("[1] Home directory only ({0})", home);
    println!("[2] Multiple custom directories");
    println!("[3] Specific application data directory");
    println!("[4] Skip for now (you can configure later)");
    
    let choice = prompt("\nEnter your choice (1-4): ");
    
    let watch_roots = match choice.as_str() {
        "1" => {
            println!("✓ Selected: Home directory ({0})", home);
            home
        },
        "2" => {
            println!("\nEnter directories separated by commas (e.g., /home/user,/var/data,/opt/myapp)");
            let dirs = prompt("Enter directories: ");
            if dirs.is_empty() {
                return Err("No directories entered".to_string());
            }
            println!("✓ Selected directories: {0}", dirs);
            dirs
        },
        "3" => {
            println!("\nCommon locations to watch:");
            println!("  - ~/Documents");
            println!("  - ~/Downloads");
            println!("  - ~/Desktop");
            println!("  - /opt (application data)");
            let dirs = prompt("Enter directories: ");
            if dirs.is_empty() {
                return Err("No directories entered".to_string());
            }
            println!("✓ Selected directories: {0}", dirs);
            dirs
        },
        "4" => {
            println!("✓ Skipped. You can configure later by setting ANYWHERE_DOOR_WATCH_ROOTS");
            return Ok("".to_string());
        },
        _ => return Err("Invalid choice".to_string()),
    };
    
    Ok(watch_roots)
}

/// Save watch configuration to file
fn save_watch_config(watch_roots: &str) -> Result<(), String> {
    let config_path = get_watch_config_path();
    
    // Create config file with watch roots
    let config = serde_json::json!({
        "watch_roots": watch_roots,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    
    let config_str = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    std::fs::write(&config_path, config_str)
        .map_err(|e| format!("Failed to save config: {}", e))?;
    
    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&config_path, perms)
            .map_err(|e| format!("Failed to set config permissions: {}", e))?;
    }
    
    println!("✓ Watch configuration saved to: {}", config_path.display());
    Ok(())
}

/// Load watch roots from config file
fn load_watch_roots() -> Option<String> {
    let config_path = get_watch_config_path();
    
    if !config_path.exists() {
        return None;
    }
    
    let config_str = std::fs::read_to_string(&config_path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&config_str).ok()?;
    config
        .get("watch_roots")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ============================================================================
// Device Initialization
// ============================================================================

/// Initialize device credentials and watch directory configuration
/// If credentials don't exist, perform full interactive setup
async fn initialize_device() -> Result<(), String> {
    let credentials_path = get_credentials_path();
    let server_url = get_server_url();

    // Check if credentials already exist
    if crate::net::net::NetworkService::has_device_credentials(&credentials_path) {
        println!("✓ Device credentials found at: {}", credentials_path.display());
        
        // Check if watch roots are configured
        if let Some(roots) = load_watch_roots() {
            if !roots.is_empty() {
                unsafe {
                    env::set_var("ANYWHERE_DOOR_WATCH_ROOTS", &roots);
                }
                println!("✓ Watch directories loaded from config");
            }
        }
        
        return Ok(());
    }

    // No credentials found - perform full interactive setup
    println!("\n========================================");
    println!("Anywhere Door - First Time Setup");
    println!("========================================");
    println!("\nThis setup will:");
    println!("  1. Authenticate you with your credentials");
    println!("  2. Register this device for secure sync");
    println!("  3. Configure directories to watch");
    
    // Step 1: Interactive login
    let jwt = interactive_login(&server_url).await?;
    
    // Step 2: Device registration
    println!("\nRegistering device...");
    let auth_service = crate::auth::AuthService::new("jwt-secret".to_string());
    let mut network_service = crate::net::net::NetworkService::new(
        server_url,
        auth_service,
        jwt,
        "temp-device".to_string(),
        "temp-secret".to_string(),
    );

    network_service.register_and_save_device(&credentials_path).await?;
    
    // Step 3: Select watch directories
    let watch_roots = select_watch_directories()?;
    if !watch_roots.is_empty() {
        save_watch_config(&watch_roots)?;
        unsafe {
            env::set_var("ANYWHERE_DOOR_WATCH_ROOTS", &watch_roots);
        }
    }
    
    println!("\n✓ Setup complete!");
    println!("Device initialization complete!");
    Ok(())
}

// ============================================================================
// Asset & Configuration Checks
// ============================================================================

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
    // Use tokio runtime for async initialization
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

    // Initialize device credentials
    rt.block_on(initialize_device())?;

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
    
    // Check if watch roots are configured
    let watch_roots_configured = env::var("ANYWHERE_DOOR_WATCH_ROOTS")
        .map(|r| !r.trim().is_empty())
        .unwrap_or(false);
    
    if !watch_roots_configured && watcher_enabled {
        println!("⚠️  No watch directories configured. File watcher is disabled.");
        println!("Run the agent again with interactive setup to configure directories.");
    }

    // if filesystem_enabled {
    //     start_filesystem_worker(
    //         Arc::clone(&stop_requested),
    //         filesystem_interval,
    //         filesystem_output,
    //     );
    // }

    if watcher_enabled && watch_roots_configured {
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