# Anywhere Door Agent - Network Service

## Overview

The network service enables the agent to send files, metadata, and information to a server with complete authentication and encryption. It provides:

- **Authenticated Communication**: JWT + Device Signature verification
- **File Transfers**: Base64 encoded file uploads
- **Metadata Sync**: File information (size, hash, MIME type, timestamps)
- **Batch Operations**: Send multiple files/metadata in single request
- **Error Handling**: Comprehensive error tracking and reporting

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Agent Process                                               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. FileSystem Watcher (inotify/ReadDirectoryChangesW)    │
│     ↓ Detects file changes                                │
│                                                             │
│  2. Metadata Collector                                     │
│     ↓ Extracts: size, hash, MIME type, timestamps        │
│                                                             │
│  3. Network Service                                        │
│     ├─ Signs request (HMAC-SHA256)                        │
│     ├─ Adds JWT token                                     │
│     ├─ Encodes file content (Base64)                      │
│     └─ Sends to server via HTTPS                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                                ↓
┌─────────────────────────────────────────────────────────────┐
│ Server Process                                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Receive Request                                        │
│     ↓ Parse JSON + authentication data                    │
│                                                             │
│  2. Verify Authentication                                 │
│     ├─ Decode JWT token                                   │
│     ├─ Check device is registered                         │
│     └─ Verify HMAC signature                              │
│                                                             │
│  3. Process Request                                        │
│     ├─ Decode Base64 file content                         │
│     ├─ Store file with metadata                           │
│     └─ Send acknowledgment                                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Data Structures

### FileMetadata
```json
{
  "file_path": "/home/user/documents/report.txt",
  "file_name": "report.txt",
  "file_size": 2048,
  "modified_at": 1704067200,
  "created_at": 1704067200,
  "file_hash": "abc123def456...",
  "mime_type": "text/plain",
  "is_directory": false
}
```

### AgentInfo
```json
{
  "agent_id": "550e8400-e29b-41d4-a716-446655440000",
  "agent_version": "0.1.0",
  "os": "linux",
  "hostname": "my-laptop",
  "sync_root": "/home/user/documents",
  "last_sync": 1704067200,
  "status": "ready"
}
```

### FileUploadPayload
```json
{
  "metadata": { /* FileMetadata */ },
  "file_content": "SGVsbG8gV29ybGQhIFRoaXMgaXMgQmFzZTY0..."
}
```

### AuthRequest (Complete Request)
```json
{
  "jwt": "eyJhbGciOiJIUzI1NiJ9...",
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": 1704067200,
  "signature": "a7f3e2d1c4b5a8f7e6d5c4b3a2f1e0d9...",
  "data": "{\"metadata\": {...}, \"file_content\": \"...\"}"
}
```

### ServerResponse
```json
{
  "status": "success",
  "message": "File uploaded successfully",
  "data": {
    "file_id": "550e8400-e29b-41d4-a716-446655440000",
    "stored_at": "/storage/files/user_001/report.txt",
    "timestamp": 1704067200
  }
}
```

## API Endpoints

### Metadata Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/metadata/file` | Send single file metadata |
| POST | `/api/metadata/batch` | Send multiple files metadata |
| POST | `/api/metadata/directory` | Send directory metadata with all files |

### File Upload Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/files/upload` | Upload single file with metadata |
| POST | `/api/files/batch` | Upload multiple files |

### Agent Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/agent/info` | Send agent status and info |
| GET | `/api/agent/status` | Get agent status from server |

### Sync Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/sync/directory` | Sync entire directory |
| POST | `/api/sync/status` | Get sync status |

## Usage Guide

### 1. Initialize Network Service

```rust
use anywhere_door_agent::auth::AuthService;
use anywhere_door_agent::net::NetworkService;

// Setup authentication first
let mut auth_service = AuthService::new("secret-key".to_string());
let login = auth_service.user_login("user_001", "john_doe")?;
let device = auth_service.register_device("user_001", &login.jwt)?;

// Initialize network service
let net = NetworkService::new(
    "http://server.example.com".to_string(),
    auth_service,
    login.jwt,
    device.device_id,
    device.device_secret,
);
```

### 2. Send File Metadata

```rust
use anywhere_door_agent::net::FileMetadata;

let metadata = FileMetadata {
    file_path: "/home/user/file.txt".to_string(),
    file_name: "file.txt".to_string(),
    file_size: 1024,
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

let file_path = Path::new("/home/user/document.pdf");
let response = net.upload_file(file_path).await?;
println!("File uploaded: {}", response.status);
```

### 4. Batch Upload Files

```rust
let files = vec![
    Path::new("/home/user/file1.txt"),
    Path::new("/home/user/file2.txt"),
    Path::new("/home/user/file3.txt"),
];

let responses = net.upload_files(files).await?;
println!("Uploaded {} files", responses.len());
```

### 5. Send Agent Info

```rust
use anywhere_door_agent::net::AgentInfo;
use chrono::Utc;

let agent_info = AgentInfo {
    agent_id: uuid::Uuid::new_v4().to_string(),
    agent_version: "0.1.0".to_string(),
    os: std::env::consts::OS.to_string(),
    hostname: hostname::get().ok().and_then(|h| h.into_string().ok()).unwrap_or_default(),
    sync_root: "/home/user/sync".to_string(),
    last_sync: Utc::now().timestamp(),
    status: "online".to_string(),
};

let response = net.send_agent_info(&agent_info).await?;
```

### 6. Sync Entire Directory

```rust
use std::path::Path;

let dir = Path::new("/home/user/documents");
let result = net.sync_directory(dir, &agent_info).await?;

println!("Sync completed:");
println!("  Total files: {}", result.total_files);
println!("  Uploaded: {}", result.uploaded_files);
println!("  Failed: {}", result.failed_files);
println!("  Total size: {} bytes", result.total_size);
```

## Security Features

### Authentication
- **JWT Token**: User identity verification
- **Device ID**: Device registration proof
- **HMAC-SHA256 Signature**: Request integrity and authenticity
- **Timestamp**: Replay attack prevention

### Encryption
- **HTTPS/TLS**: In-transit encryption (uses rustls)
- **Base64 Encoding**: Safe JSON transmission of binary files
- **File Hash**: SHA256 checksum for integrity verification

### Server Verification
```
For each request, server performs:
  1. JWT Decode → Verify expiration and user
  2. Device Lookup → Check device is registered to user
  3. Signature Verify → HMAC-SHA256(secret, timestamp:device_id:data)
  4. Accept → Process request only if all checks pass
```

## Performance Characteristics

### Metadata Transfer
- Size: ~500 bytes per file
- Batch Support: Send 100+ files in single request
- Typical Time: < 100ms per batch

### File Upload
- Base64 Overhead: ~33% larger than binary
- Chunking: Not yet implemented (full file in memory)
- Concurrent: Sequential uploads (can be parallelized)

### Example Performance
```
File Size → Network Payload → Typical Time
====================================================
1 MB     → 1.33 MB           → ~50ms
10 MB    → 13.3 MB           → ~500ms
100 MB   → 133 MB            → ~5s
```

## Error Handling

### Common Errors
```rust
// Authentication failed
Err("JWT verification failed: token expired")

// Device not found
Err("Device not registered")

// Signature mismatch
Err("Signature verification error: ...")

// File not found
Err("Failed to open file: No such file or directory")

// Network error
Err("HTTP request failed: Connection refused")
```

### Error Recovery
```rust
// Try upload with automatic retry
let mut attempts = 0;
loop {
    match net.upload_file(file_path).await {
        Ok(response) => break,
        Err(e) if attempts < 3 => {
            attempts += 1;
            tokio::time::sleep(Duration::from_secs(2_u64.pow(attempts))).await;
        }
        Err(e) => return Err(format!("Upload failed: {}", e)),
    }
}
```

## Integration with File Watcher

```rust
use std::sync::mpsc;
use std::path::PathBuf;

// Channel for file change events
let (tx, rx) = mpsc::channel();

// Start file watcher
filesystem::watch_files(watch_roots, tx)?;

// Process file changes
for file_event in rx {
    // Send metadata
    net.send_file_metadata(&file_event.metadata).await?;
    
    // Upload file
    net.upload_file(&file_event.path).await?;
}
```

## Dependencies

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json", "stream", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

## Testing

Run the network demo:
```bash
cargo run --example network_demo
```

Run tests:
```bash
cargo test --lib net
```

## Future Enhancements

1. **Chunked Uploads**: Large files in multiple chunks
2. **Concurrent Uploads**: Parallel file transfers
3. **Resume Support**: In case of connection loss
4. **Compression**: GZIP compression before upload
5. **Deduplication**: Skip already-synced files
6. **Bandwidth Limiting**: Control upload speed
7. **Progress Callbacks**: Real-time upload progress
8. **Versioning**: Track file versions on server

## Troubleshooting

### "Failed to open file"
- Check file permissions
- Ensure path is correct
- File may have been deleted

### "JWT verification failed"
- Token may have expired (24 hours)
- Re-login to get new token
- Check server time synchronization

### "Signature mismatch"
- Device secret may be incorrect
- Request data was modified
- Check network for corruption

### "HTTP request failed"
- Server unreachable
- Check network connectivity
- Verify server URL is correct
- Check firewall rules
