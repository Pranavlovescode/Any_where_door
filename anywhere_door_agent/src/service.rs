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

/// Get the user's home directory, with Windows compatibility
fn get_home_dir() -> String {
    // Try HOME first (Linux/Mac), then USERPROFILE (Windows)
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string())
}

/// Get the path to the device credentials file
pub fn get_credentials_path() -> PathBuf {
    match env::var("ANYWHERE_DOOR_CREDENTIALS_PATH") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => PathBuf::from(get_home_dir()).join(".anywheredoor"),
    }
}

/// Get the path to the watch config file
fn get_watch_config_path() -> PathBuf {
    match env::var("ANYWHERE_DOOR_CONFIG_PATH") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => PathBuf::from(get_home_dir()).join(".anywheredoor_watch_roots"),
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

struct LoginCredentials {
    username: String,
    password: String,
    jwt: String,
}

/// Interactive login - prompts for username/password and gets JWT from backend
async fn interactive_login(server_url: &str) -> Result<LoginCredentials, String> {
    println!("\n========================================");
    println!("User Login");
    println!("========================================");
    
    let username = prompt("Enter username: ");
    let password = prompt_password("Enter password: ");
    
    // Call backend login endpoint
    let client = reqwest::Client::new();
    let login_payload = serde_json::json!({
        "username": username.clone(),
        "password": password.clone()
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
    Ok(LoginCredentials {
        username,
        password,
        jwt: jwt.to_string(),
    })
}

/// Show menu to select directories to watch
fn select_watch_directories() -> Result<String, String> {
    println!("\n========================================");
    println!("Select Directories to Watch");
    println!("========================================");
    
    let home = get_home_dir();
    
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
    let config: serde_json::Value = serde_json::from_str(config_str.trim_start_matches('\u{feff}')).ok()?;
    config
        .get("watch_roots")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ============================================================================
// Device Initialization
// ============================================================================

/// Non-interactive device config loader for service mode.
/// Loads existing credentials and watch roots without prompting.
/// Used by the Windows service which cannot do interactive setup.
pub async fn try_load_device_config() -> Result<(), String> {
    let credentials_path = get_credentials_path();

    if crate::net::net::NetworkService::has_device_credentials(&credentials_path) {
        eprintln!("✓ Device credentials found at: {}", credentials_path.display());
    } else {
        eprintln!("⚠ No device credentials found at: {}", credentials_path.display());
        eprintln!("  Run the installer script to set up authentication.");
    }

    // Load watch roots from config file into env var
    if let Some(roots) = load_watch_roots() {
        if !roots.is_empty() {
            unsafe {
                env::set_var("ANYWHERE_DOOR_WATCH_ROOTS", &roots);
            }
            eprintln!("✓ Watch directories loaded from config: {}", roots);
        }
    }

    Ok(())
}

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
    let login = interactive_login(&server_url).await?;
    
    // Step 2: Device registration
    println!("\nRegistering device...");
    let auth_service = crate::auth::AuthService::new("jwt-secret".to_string());
    let mut network_service = crate::net::net::NetworkService::new(
        server_url,
        auth_service,
        login.jwt.clone(),
        "temp-device".to_string(),
        "temp-secret".to_string(),
    );

    network_service.register_and_save_device(&credentials_path).await?;
    persist_auth_credentials(&credentials_path, &login)?;
    
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

fn persist_auth_credentials(
    credentials_path: &PathBuf,
    login: &LoginCredentials,
) -> Result<(), String> {
    let credentials_str = std::fs::read_to_string(credentials_path)
        .map_err(|e| format!("Failed to read credentials file: {}", e))?;
    let mut credentials_json: serde_json::Value = serde_json::from_str(&credentials_str)
        .map_err(|e| format!("Failed to parse credentials file: {}", e))?;

    let obj = credentials_json
        .as_object_mut()
        .ok_or_else(|| "Credentials file is not a JSON object".to_string())?;
    obj.insert(
        "username".to_string(),
        serde_json::Value::String(login.username.clone()),
    );
    obj.insert(
        "password".to_string(),
        serde_json::Value::String(login.password.clone()),
    );
    obj.insert("jwt".to_string(), serde_json::Value::String(login.jwt.clone()));

    let serialized = serde_json::to_string_pretty(&credentials_json)
        .map_err(|e| format!("Failed to serialize credentials update: {}", e))?;
    std::fs::write(credentials_path, serialized)
        .map_err(|e| format!("Failed to write updated credentials: {}", e))?;

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

pub fn sync_enabled() -> bool {
    match env::var("ANYWHERE_DOOR_ENABLE_SYNC") {
        Ok(value) => matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => true, // enabled by default
    }
}

fn start_file_watcher_worker(
    stop_requested: Arc<AtomicBool>,
    output_path: String,
    sync_tx: Option<tokio::sync::mpsc::UnboundedSender<notify::Event>>,
) {
    thread::spawn(move || {
        if let Err(err) = crate::filesystem::watcher::run_os_file_watcher(stop_requested, &output_path, sync_tx) {
            eprintln!("OS file watcher failed: {}", err);
        }
    });
}

pub fn run_service() -> Result<(), String> {
    // Create a tokio runtime that persists for the entire service lifetime.
    // This runtime handles: device init, sync pipeline, and graceful shutdown.
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

    // Initialize device credentials (interactive if first run)
    rt.block_on(initialize_device())?;

    let stop_requested = Arc::new(AtomicBool::new(false));
    let credentials_path = get_credentials_path();

    // Start the sync pipeline if enabled and credentials exist
    let pipeline_handle = if sync_enabled()
        && crate::net::net::NetworkService::has_device_credentials(&credentials_path)
    {
        let handle = rt.block_on(async {
            crate::sync::start_pipeline(credentials_path)
        });
        Some(handle)
    } else {
        if sync_enabled() {
            eprintln!("Sync: disabled (no device credentials found)");
        } else {
            eprintln!("Sync: disabled via ANYWHERE_DOOR_ENABLE_SYNC");
        }
        None
    };

    // Extract the event sender for the watcher (if pipeline is running)
    let sync_tx = pipeline_handle.as_ref().map(|h| h.event_tx.clone());

    // Run the main service loop (blocking)
    let result = run_loop(Arc::clone(&stop_requested), true, true, sync_tx);

    // Graceful shutdown of the sync pipeline
    if let Some(handle) = pipeline_handle {
        eprintln!("Sync: shutting down pipeline...");
        rt.block_on(handle.shutdown());
    }

    result
}

pub fn run_loop(
    stop_requested: Arc<AtomicBool>,
    install_signal_handler: bool,
    announce_lifecycle: bool,
    sync_tx: Option<tokio::sync::mpsc::UnboundedSender<notify::Event>>,
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
        start_file_watcher_worker(Arc::clone(&stop_requested), watcher_output, sync_tx);
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
