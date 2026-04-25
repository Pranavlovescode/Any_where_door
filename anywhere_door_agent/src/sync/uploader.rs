//! Upload worker pool with hash-check deduplication and exponential-backoff
//! retry logic.
//!
//! Workers pull [`SyncEvent`] items from the [`SyncQueue`], compute the file
//! hash, ask the server whether it already has the file, and upload only when
//! necessary. Failed uploads are re-enqueued with exponential backoff.

use super::debounce::{SyncEvent, SyncEventKind};
use super::queue::SyncQueue;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

// ============================================================================
// Configuration helpers
// ============================================================================

fn upload_worker_count() -> usize {
    std::env::var("ANYWHERE_DOOR_UPLOAD_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3)
}

fn max_retry_attempts() -> u32 {
    std::env::var("ANYWHERE_DOOR_UPLOAD_RETRY_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)
}

fn server_url() -> String {
    std::env::var("ANYWHERE_DOOR_SERVER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8000".to_string())
}

/// Get the user's home directory, with Windows compatibility
fn get_home_dir() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string())
}

// ============================================================================
// Retry entry
// ============================================================================

#[derive(Debug, Clone)]
struct RetryEntry {
    event: SyncEvent,
    attempt: u32,
    retry_after: tokio::time::Instant,
}

// ============================================================================
// JWT management
// ============================================================================

/// Obtain a valid JWT by logging in with device credentials.
/// The device_secret is used as the password (backend should support this),
/// or we re-read any stored JWT from the credentials file.
async fn obtain_jwt(
    client: &reqwest::Client,
    base_url: &str,
    credentials_path: &Path,
) -> Result<String, String> {
    // First, try to read JWT from credentials file (might have been refreshed)
    let json = std::fs::read_to_string(credentials_path)
        .map_err(|e| format!("Failed to read credentials: {}", e))?;
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse credentials: {}", e))?;

    // If there's a JWT stored, try to use it first
    if let Some(jwt) = value.get("jwt").and_then(|v| v.as_str()) {
        if !jwt.is_empty() {
            // Quick validation: try a lightweight request
            let test_url = format!("{}/api/files/check-hashes", base_url);
            let test_payload = serde_json::json!({
                "jwt": jwt,
                "hashes": []
            });
            let resp = client.post(&test_url).json(&test_payload).send().await;
            if let Ok(r) = resp {
                if r.status().is_success() {
                    return Ok(jwt.to_string());
                }
            }
            eprintln!("Sync: stored JWT is expired or invalid, re-authenticating...");
        }
    }

    // No valid JWT — need to login.
    // Read username from credentials file (if stored by installer)
    let username = value
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let password = value
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if username.is_empty() || password.is_empty() {
        return Err(
            "No valid JWT and no username/password in credentials file. \
             Re-run the installer to set up authentication."
                .to_string(),
        );
    }

    // Login to get a fresh JWT
    let login_url = format!("{}/auth/login", base_url);
    let login_payload = serde_json::json!({
        "username": username,
        "password": password
    });

    let response = client
        .post(&login_url)
        .json(&login_payload)
        .send()
        .await
        .map_err(|e| format!("Login request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Login failed ({}): {}", status, body));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse login response: {}", e))?;

    let jwt = body
        .get("jwt")
        .and_then(|v| v.as_str())
        .ok_or("No jwt in login response")?
        .to_string();

    // Save the fresh JWT back to credentials file
    let mut creds = value.clone();
    creds["jwt"] = serde_json::Value::String(jwt.clone());
    if let Ok(updated) = serde_json::to_string_pretty(&creds) {
        let _ = std::fs::write(credentials_path, updated);
    }

    eprintln!("Sync: re-authenticated successfully, JWT refreshed");
    Ok(jwt)
}

// ============================================================================
// Upload workers
// ============================================================================

/// Spawns `N` upload worker tasks that drain the queue and upload files.
///
/// Returns a join handle vec so the caller can await graceful shutdown.
pub fn spawn_workers(
    queue: Arc<SyncQueue>,
    stop: tokio::sync::watch::Receiver<bool>,
    credentials_path: std::path::PathBuf,
) -> Vec<tokio::task::JoinHandle<()>> {
    let n = upload_worker_count();
    let retry_queue: Arc<Mutex<Vec<RetryEntry>>> = Arc::new(Mutex::new(Vec::new()));

    // Share the JWT across all workers (refreshable)
    let jwt_holder: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

    let mut handles = Vec::with_capacity(n + 1);

    // Spawn upload workers
    for worker_id in 0..n {
        let q = Arc::clone(&queue);
        let rq = Arc::clone(&retry_queue);
        let stop_rx = stop.clone();
        let creds = credentials_path.clone();
        let jwt = Arc::clone(&jwt_holder);

        handles.push(tokio::spawn(async move {
            worker_loop(worker_id, q, rq, stop_rx, creds, jwt).await;
        }));
    }

    // Spawn retry ticker — re-enqueues eligible retry entries every 5 seconds
    {
        let q = Arc::clone(&queue);
        let rq = Arc::clone(&retry_queue);
        let stop_rx = stop.clone();

        handles.push(tokio::spawn(async move {
            retry_ticker(q, rq, stop_rx).await;
        }));
    }

    eprintln!("Sync: started {} upload workers", n);
    handles
}

/// Main loop for a single upload worker.
async fn worker_loop(
    worker_id: usize,
    queue: Arc<SyncQueue>,
    retry_queue: Arc<Mutex<Vec<RetryEntry>>>,
    mut stop: tokio::sync::watch::Receiver<bool>,
    credentials_path: std::path::PathBuf,
    jwt_holder: Arc<Mutex<String>>,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap_or_default();
    let base_url = server_url();

    // Load device credentials
    let (device_id, device_secret) = match load_credentials(&credentials_path) {
        Ok(creds) => creds,
        Err(e) => {
            eprintln!("Sync worker {}: failed to load credentials: {}", worker_id, e);
            return;
        }
    };

    eprintln!(
        "Sync worker {}: ready (device: {}..., server: {})",
        worker_id,
        &device_id[..8.min(device_id.len())],
        base_url
    );

    // Ensure JWT is loaded (only first worker does the actual login)
    {
        let mut jwt = jwt_holder.lock().await;
        if jwt.is_empty() {
            match obtain_jwt(&client, &base_url, &credentials_path).await {
                Ok(token) => {
                    *jwt = token;
                    eprintln!("Sync worker {}: JWT obtained", worker_id);
                }
                Err(e) => {
                    eprintln!("Sync worker {}: JWT auth failed: {}", worker_id, e);
                    return;
                }
            }
        }
    }

    loop {
        if *stop.borrow() {
            break;
        }

        // Grab a batch of events
        let batch = queue.pop_batch(1).await;
        if batch.is_empty() {
            // Nothing to do — sleep briefly then check again
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                _ = stop.changed() => { break; }
            }
            continue;
        }

        for event in batch {
            let jwt = jwt_holder.lock().await.clone();

            let result = process_event(
                &client,
                &base_url,
                &jwt,
                &device_id,
                &device_secret,
                &event,
            )
            .await;

            match &result {
                Ok(()) => {}
                Err(e) if e.contains("401") || e.contains("Invalid token") => {
                    // JWT expired — try to refresh
                    eprintln!("Sync worker {}: JWT expired, refreshing...", worker_id);
                    match obtain_jwt(&client, &base_url, &credentials_path).await {
                        Ok(new_jwt) => {
                            *jwt_holder.lock().await = new_jwt;
                            // Retry this event immediately
                            queue.push(event).await;
                        }
                        Err(auth_err) => {
                            eprintln!(
                                "Sync worker {}: re-auth failed: {}",
                                worker_id, auth_err
                            );
                        }
                    }
                    continue;
                }
                Err(e) => {
                    eprintln!(
                        "Sync worker {}: upload failed for {}: {}",
                        worker_id,
                        event.path.display(),
                        e
                    );

                    // Push to retry queue
                    let max_retries = max_retry_attempts();
                    let mut rq = retry_queue.lock().await;

                    // Find existing retry entry for this path, or create new
                    let existing_attempt = rq
                        .iter()
                        .position(|r| r.event.path == event.path)
                        .map(|i| rq.remove(i).attempt)
                        .unwrap_or(0);

                    let next_attempt = existing_attempt + 1;

                    if next_attempt <= max_retries {
                        let backoff_ms =
                            std::cmp::min(2u64.pow(next_attempt) * 1_000, 60_000);
                        let retry_after =
                            tokio::time::Instant::now() + Duration::from_millis(backoff_ms);

                        eprintln!(
                            "Sync worker {}: retry {}/{} for {} in {}ms",
                            worker_id,
                            next_attempt,
                            max_retries,
                            event.path.display(),
                            backoff_ms
                        );

                        rq.push(RetryEntry {
                            event,
                            attempt: next_attempt,
                            retry_after,
                        });
                    } else {
                        eprintln!(
                            "Sync worker {}: giving up on {} after {} attempts",
                            worker_id,
                            event.path.display(),
                            max_retries
                        );
                    }
                }
            }
        }
    }
}

/// Process a single sync event: hash-check → upload (if needed).
async fn process_event(
    client: &reqwest::Client,
    base_url: &str,
    jwt: &str,
    _device_id: &str,
    _device_secret: &str,
    event: &SyncEvent,
) -> Result<(), String> {
    match &event.event_kind {
        SyncEventKind::Create | SyncEventKind::Modify => {
            // Skip if file no longer exists (deleted between event and processing)
            if !event.path.exists() {
                return Ok(());
            }

            // Compute hash
            let hash = compute_sha256(&event.path)?;

            // Check if server already has this hash
            if check_hash_exists(client, base_url, jwt, &hash).await? {
                eprintln!("Sync: skipping {} (hash already known)", event.path.display());
                return Ok(());
            }

            // Upload the file
            upload_file(client, base_url, jwt, &event.path, &hash).await?;
            eprintln!("Sync: uploaded {}", event.path.display());
            Ok(())
        }
        SyncEventKind::Remove => {
            // Log removal; we don't propagate deletes to the server for now
            eprintln!("Sync: file removed locally: {}", event.path.display());
            Ok(())
        }
        SyncEventKind::Rename { from } => {
            // Treat rename-to as a new create
            eprintln!(
                "Sync: file renamed from {} to {}",
                from.display(),
                event.path.display()
            );
            if event.path.exists() {
                let hash = compute_sha256(&event.path)?;
                if !check_hash_exists(client, base_url, jwt, &hash).await? {
                    upload_file(client, base_url, jwt, &event.path, &hash).await?;
                    eprintln!("Sync: uploaded renamed file {}", event.path.display());
                }
            }
            Ok(())
        }
    }
}

// ============================================================================
// Retry ticker
// ============================================================================

/// Periodically checks the retry queue and re-enqueues items whose backoff has
/// elapsed.
async fn retry_ticker(
    queue: Arc<SyncQueue>,
    retry_queue: Arc<Mutex<Vec<RetryEntry>>>,
    mut stop: tokio::sync::watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = stop.changed() => { break; }
        }

        if *stop.borrow() {
            break;
        }

        let now = tokio::time::Instant::now();
        let mut rq = retry_queue.lock().await;

        let mut ready_indices: Vec<usize> = Vec::new();
        for (i, entry) in rq.iter().enumerate() {
            if now >= entry.retry_after {
                ready_indices.push(i);
            }
        }

        // Remove from back to front to keep indices valid
        ready_indices.sort_unstable_by(|a, b| b.cmp(a));
        for i in ready_indices {
            let entry = rq.remove(i);
            queue.push(entry.event).await;
        }
    }
}

// ============================================================================
// HTTP helpers
// ============================================================================

/// Ask the server if it already has a file with the given SHA256 hash.
async fn check_hash_exists(
    client: &reqwest::Client,
    base_url: &str,
    jwt: &str,
    hash: &str,
) -> Result<bool, String> {
    let url = format!("{}/api/files/check-hashes", base_url);

    let payload = serde_json::json!({
        "jwt": jwt,
        "hashes": [hash]
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Hash check request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Hash check failed ({}): {}", status, body));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse hash check response: {}", e))?;

    let known = body
        .get("known")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|v| v.as_str() == Some(hash)))
        .unwrap_or(false);

    Ok(known)
}

/// Upload a file to the server via base64-encoded JSON payload.
async fn upload_file(
    client: &reqwest::Client,
    base_url: &str,
    jwt: &str,
    path: &Path,
    hash: &str,
) -> Result<(), String> {
    // Read file
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?;

    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let file_size = contents.len() as i64;
    let encoded = base64_encode(&contents);

    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mime_type = guess_mime(path);

    let payload = serde_json::json!({
        "metadata": {
            "file_path": path.to_string_lossy(),
            "file_name": file_name,
            "file_size": file_size,
            "modified_at": 0,
            "created_at": 0,
            "file_hash": hash,
            "mime_type": mime_type,
            "is_directory": false
        },
        "file_content": encoded,
        "source": "agent"
    });

    let url = format!("{}/api/files/upload?jwt={}", base_url, jwt);

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Upload request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Upload failed ({}): {}", status, body));
    }

    Ok(())
}

// ============================================================================
// Utility functions
// ============================================================================

fn compute_sha256(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open file for hashing: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn load_credentials(
    credentials_path: &Path,
) -> Result<(String, String), String> {
    let json = std::fs::read_to_string(credentials_path)
        .map_err(|e| format!("Failed to read credentials at {}: {}", credentials_path.display(), e))?;

    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse credentials: {}", e))?;

    let device_id = value
        .get("device_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing device_id in credentials")?
        .to_string();

    let device_secret = value
        .get("device_secret")
        .and_then(|v| v.as_str())
        .ok_or("Missing device_secret in credentials")?
        .to_string();

    Ok((device_id, device_secret))
}

fn guess_mime(path: &Path) -> String {
    match path.extension().and_then(|s| s.to_str()) {
        Some("txt") => "text/plain".to_string(),
        Some("json") => "application/json".to_string(),
        Some("pdf") => "application/pdf".to_string(),
        Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
        Some("png") => "image/png".to_string(),
        Some("zip") => "application/zip".to_string(),
        Some("gz") => "application/gzip".to_string(),
        Some("html") | Some("htm") => "text/html".to_string(),
        Some("css") => "text/css".to_string(),
        Some("js") => "application/javascript".to_string(),
        Some("rs") => "text/x-rust".to_string(),
        Some("py") => "text/x-python".to_string(),
        Some(ext) => format!("application/{}", ext),
        None => "application/octet-stream".to_string(),
    }
}

/// Simple base64 encoder (no external dependency needed).
fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);

        let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARSET[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}
