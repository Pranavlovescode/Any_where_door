# Network Service Integration Guide

## Quick Start

### 1. Initialize Everything

```rust
use anywhere_door_agent::auth::AuthService;
use anywhere_door_agent::net::{NetworkService, AgentInfo};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Auth
    let mut auth = AuthService::new("my-secret".to_string());
    let login = auth.user_login("user_001", "john")?;
    let device = auth.register_device("user_001", &login.jwt)?;

    // Step 2: Network
    let net = NetworkService::new(
        "http://localhost:8080".to_string(),
        auth,
        login.jwt,
        device.device_id,
        device.device_secret,
    );

    // Step 3: Send Agent Info
    let agent = AgentInfo {
        agent_id: uuid::Uuid::new_v4().to_string(),
        agent_version: "0.1.0".to_string(),
        os: std::env::consts::OS.to_string(),
        hostname: "my-laptop".to_string(),
        sync_root: "/home/user/docs".to_string(),
        last_sync: chrono::Utc::now().timestamp(),
        status: "online".to_string(),
    };

    net.send_agent_info(&agent).await?;

    Ok(())
}
```

### 2. Send Metadata Only

```rust
use anywhere_door_agent::net::FileMetadata;

let metadata = FileMetadata {
    file_path: "/home/user/report.txt".to_string(),
    file_name: "report.txt".to_string(),
    file_size: 2048,
    modified_at: 1704067200,
    created_at: 1704067200,
    file_hash: "abc123".to_string(),
    mime_type: "text/plain".to_string(),
    is_directory: false,
};

let response = net.send_file_metadata(&metadata).await?;
println!("{:?}", response);
```

### 3. Upload a File

```rust
use std::path::Path;

let file = Path::new("/home/user/document.pdf");
let response = net.upload_file(file).await?;
```

### 4. Sync Entire Directory

```rust
let dir = Path::new("/home/user/documents");
let result = net.sync_directory(dir, &agent).await?;

println!("Files uploaded: {}/{}", result.uploaded_files, result.total_files);
if !result.errors.is_empty() {
    println!("Errors: {:?}", result.errors);
}
```

## Project Structure

```
src/
├── auth/          ← User authentication (JWT + device)
├── net/           ← Network service (file upload + sync)
├── filesystem/    ← File watcher (detects changes)
└── service.rs     ← Main service logic
```

## Integration Flow

```
┌─────────────────────────────────────────────────────┐
│ 1. FILE CHANGES                                     │
│    FileSystem Watcher detects new/modified files   │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│ 2. EXTRACT METADATA                                 │
│    - File size, path, timestamps                   │
│    - Calculate SHA256 hash                         │
│    - Determine MIME type                           │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│ 3. SEND METADATA                                    │
│    net.send_file_metadata(&metadata)?              │
│    (Small, fast request)                           │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│ 4. UPLOAD FILE CONTENT                              │
│    net.upload_file(&file_path)?                    │
│    (Includes JWT + signature for auth)             │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│ 5. SERVER VERIFICATION                              │
│    ✓ Verify JWT token                              │
│    ✓ Check device registered                       │
│    ✓ Verify HMAC signature                         │
│    ✓ Store file + metadata                         │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│ 6. ACKNOWLEDGMENT                                   │
│    Server sends success response                   │
│    Agent continues with next file                  │
└─────────────────────────────────────────────────────┘
```

## Complete Example: Service Integration

```rust
use anywhere_door_agent::auth::AuthService;
use anywhere_door_agent::net::{NetworkService, AgentInfo};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Anywhere Door Agent ===\n");

    // === AUTHENTICATION ===
    println!("[1/4] Setting up authentication...");
    let mut auth = AuthService::new("my-secret-key".to_string());
    
    let login = auth.user_login("user_001", "john_doe")?;
    println!("✓ Logged in as: {}", login.user_id);

    let device = auth.register_device("user_001", &login.jwt)?;
    println!("✓ Device registered: {}\n", device.device_id);

    // === NETWORK SERVICE ===
    println!("[2/4] Initializing network service...");
    let net = NetworkService::new(
        "http://localhost:8080".to_string(),
        auth,
        login.jwt,
        device.device_id,
        device.device_secret,
    );
    println!("✓ Network service ready\n");

    // === AGENT INFO ===
    println!("[3/4] Sending agent information...");
    let agent_info = AgentInfo {
        agent_id: uuid::Uuid::new_v4().to_string(),
        agent_version: "0.1.0".to_string(),
        os: std::env::consts::OS.to_string(),
        hostname: hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string()),
        sync_root: "/home/user/documents".to_string(),
        last_sync: chrono::Utc::now().timestamp(),
        status: "syncing".to_string(),
    };

    match net.send_agent_info(&agent_info).await {
        Ok(response) => println!("✓ Agent info sent: {}\n", response.status),
        Err(e) => println!("✗ Failed to send agent info: {}\n", e),
    }

    // === FILE SYNC ===
    println!("[4/4] Syncing files...");
    let dir = Path::new("/home/user/documents");
    
    match net.sync_directory(dir, &agent_info).await {
        Ok(result) => {
            println!("✓ Sync completed!");
            println!("  Total files: {}", result.total_files);
            println!("  Uploaded: {}", result.uploaded_files);
            println!("  Failed: {}", result.failed_files);
            println!("  Total size: {} MB", result.total_size / 1_000_000);
            
            if !result.errors.is_empty() {
                println!("\n✗ Errors:");
                for error in result.errors {
                    println!("  - {}", error);
                }
            }
        }
        Err(e) => println!("✗ Sync failed: {}", e),
    }

    println!("\n=== Sync Completed ===");
    Ok(())
}
```

## Server Stub (for testing)

```rust
// Simple HTTP server to test agent communication
use std::net::TcpListener;
use std::io::{Read, Write};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming().{
        if let Ok(mut stream) = stream {
            let mut buffer = [0; 512];
            if let Ok(n) = stream.read(&mut buffer) {
                let request = String::from_utf8_lossy(&buffer[..n]);
                println!("Received: {}", request);

                let response = "{\"status\":\"success\",\"message\":\"OK\"}";
                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                    response.len(),
                    response
                );
                let _ = stream.write_all(http_response.as_bytes());
            }
        }
    }
}
```

## Testing

### Run the Network Demo
```bash
cargo run --example network_demo
```

### Run Tests
```bash
cargo test --lib net
```

### Manual Testing with curl
```bash
# Test server endpoint
curl -X POST http://localhost:8080/api/metadata/file \
  -H "Content-Type: application/json" \
  -d '{
    "file_path": "/home/user/file.txt",
    "file_name": "file.txt",
    "file_size": 1024,
    "modified_at": 1704067200,
    "created_at": 1704067200,
    "file_hash": "abc123",
    "mime_type": "text/plain",
    "is_directory": false
  }'
```

## API Endpoints

### Metadata Endpoints
```
POST /api/metadata/file          Send single file metadata
POST /api/metadata/batch         Send multiple files metadata
POST /api/metadata/directory     Send directory with all files
```

### File Upload
```
POST /api/files/upload           Upload single file
POST /api/files/batch            Upload multiple files
```

### Agent
```
POST /api/agent/info             Send agent status
GET  /api/agent/status           Get agent status
```

### Sync
```
POST /api/sync/directory         Sync entire directory
GET  /api/sync/status            Get sync status
```

## Authentication Headers

All requests include:
```json
{
  "jwt": "<user_token>",
  "device_id": "<device_uuid>",
  "timestamp": 1704067200,
  "signature": "<hmac_sha256>",
  "data": "<request_payload>"
}
```

## Error Responses

### Server Error Response
```json
{
  "status": "error",
  "message": "Authentication failed",
  "data": null
}
```

### Agent Error Handling
```rust
match net.upload_file(file).await {
    Ok(response) => {
        if response.status == "success" {
            println!("✓ Uploaded: {}", file.display());
        } else {
            println!("✗ Failed: {}", response.message);
        }
    }
    Err(e) => {
        println!("✗ Error: {}", e);
        // Retry or skip this file
    }
}
```

## Performance Tips

1. **Batch Metadata**: Send metadata for 10-20 files together
2. **Sequential Uploads**: Upload files one at a time
3. **Compression**: Consider gzip for text files
4. **Chunking**: For large files, split into 5MB chunks
5. **Retry Logic**: Implement exponential backoff on failure
6. **Rate Limiting**: Respect server rate limits
7. **Connection Pooling**: Reuse HTTP connections

## Security Checklist

- [ ] Use HTTPS/TLS (not HTTP)
- [ ] Verify JWT before processing
- [ ] Check device is registered
- [ ] Verify HMAC signature
- [ ] Validate file hash
- [ ] Check file permissions
- [ ] Sanitize file paths
- [ ] Limit file size
- [ ] Log all uploads
- [ ] Encrypt stored files

## Files Created

- ✅ `src/net/net.rs` - Network service implementation
- ✅ `src/net/mod.rs` - Module exports
- ✅ `examples/network_demo.rs` - Complete working example
- ✅ `NETWORK_SERVICE.md` - Comprehensive documentation
- ✅ `AUTH_INTEGRATION.md` - Integration guide

## Next Steps

1. Implement server endpoints
2. Add database storage for files
3. Implement retry logic in agent
4. Add progress callbacks
5. Support chunked uploads
6. Add file versioning
