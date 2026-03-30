// ⚠️  NETWORK SERVICE EXAMPLE & USAGE
//
// This demonstrates the complete agent → server communication:
//
//   Agent Collects Files/Metadata
//        ↓
//   Signs Request with Device Secret
//        ↓
//   Sends to Server with JWT + Signature
//        ↓
//   Server Verifies Authentication
//        ↓
//   Data Stored on Server ✓

use anywhere_door_agent::auth::{AuthService};
use anywhere_door_agent::net::{NetworkService, FileMetadata, AgentInfo};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Anywhere Door Network Service Demo ===\n");

    // ========================================================================
    // STEP 1: INITIALIZE AUTH SERVICE
    // ========================================================================
    println!("--- STEP 1: Initialize Auth Service ---");
    let mut auth_service = AuthService::new("my-secret-jwt-key".to_string());
    println!("✓ Auth service initialized\n");

    // ========================================================================
    // STEP 2: USER LOGIN & DEVICE REGISTRATION
    // ========================================================================
    println!("--- STEP 2: User Login & Device Registration ---");
    
    let login = auth_service
        .user_login("user_001", "john_doe")
        .expect("Failed to login");
    
    println!("User logged in: {} (JWT: {}...)", login.user_id, &login.jwt[..50]);

    let device = auth_service
        .register_device("user_001", &login.jwt)
        .expect("Failed to register device");
    
    println!("Device registered: {}\n", device.device_id);

    // ========================================================================
    // STEP 3: INITIALIZE NETWORK SERVICE
    // ========================================================================
    println!("--- STEP 3: Initialize Network Service ---");
    
    let _network_service = NetworkService::new(
        "http://localhost:8080".to_string(),  // Server URL
        auth_service,
        login.jwt.clone(),
        device.device_id.clone(),
        device.device_secret.clone(),
    );
    
    println!("✓ Network service initialized");
    println!("  Server: http://localhost:8080");
    println!("  Device ID: {}\n", device.device_id);

    // ========================================================================
    // STEP 4: CREATE AGENT INFO
    // ========================================================================
    println!("--- STEP 4: Create Agent Info ---");
    
    let agent_info = AgentInfo {
        agent_id: uuid::Uuid::new_v4().to_string(),
        agent_version: "0.1.0".to_string(),
        os: std::env::consts::OS.to_string(),
        hostname: hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string()),
        sync_root: "/home/user/documents".to_string(),
        last_sync: Utc::now().timestamp(),
        status: "ready".to_string(),
    };
    
    println!("Agent Info:");
    println!("  Agent ID: {}", agent_info.agent_id);
    println!("  Version: {}", agent_info.agent_version);
    println!("  OS: {}", agent_info.os);
    println!("  Status: {}\n", agent_info.status);

    // ========================================================================
    // STEP 5: DEMONSTRATE FILE METADATA
    // ========================================================================
    println!("--- STEP 5: Create Sample File Metadata ---");
    
    let file_metadata = FileMetadata {
        file_path: "/home/user/documents/report.txt".to_string(),
        file_name: "report.txt".to_string(),
        file_size: 2048,
        modified_at: Utc::now().timestamp() - 3600,
        created_at: Utc::now().timestamp() - 86400,
        file_hash: "abc123def456".to_string(),
        mime_type: "text/plain".to_string(),
        is_directory: false,
    };
    
    println!("File Metadata:");
    println!("  File: {}", file_metadata.file_name);
    println!("  Size: {} bytes", file_metadata.file_size);
    println!("  MIME Type: {}", file_metadata.mime_type);
    println!("  Hash: {}\n", file_metadata.file_hash);

    // ========================================================================
    // STEP 6: DEMONSTRATE NETWORK OPERATIONS
    // ========================================================================
    println!("--- STEP 6: Network Operations (Simulated) ---");
    
    println!("✓ Would send file metadata to: POST /api/metadata/file");
    println!("  Payload includes: file_path, file_size, hash, mime_type\n");
    
    println!("✓ Would send batch metadata to: POST /api/metadata/batch");
    println!("  Payload includes: array of file metadata\n");
    
    println!("✓ Would send agent info to: POST /api/agent/info");
    println!("  Payload includes: agent_id, version, status, sync_root\n");
    
    println!("✓ Would upload file to: POST /api/files/upload");
    println!("  Payload includes: metadata + base64 encoded file content\n");

    // ========================================================================
    // STEP 7: AUTHENTICATION FLOW
    // ========================================================================
    println!("--- STEP 7: Authentication Flow (Each Request) ---");
    
    let timestamp = Utc::now().timestamp();
    let request_data = r#"{"action":"sync", "files": ["file1.txt", "file2.txt"]}"#;
    
    println!("Request Structure:");
    println!("  {{");
    println!("    \"jwt\": \"{}\",", &login.jwt[..30]);
    println!("    \"device_id\": \"{}\",", device.device_id);
    println!("    \"timestamp\": {},", timestamp);
    println!("    \"signature\": \"<HMAC-SHA256>\",");
    println!("    \"data\": \"{}\"", request_data);
    println!("  }}\n");

    println!("Server Verification:");
    println!("  ✓ Decrypt JWT token");
    println!("  ✓ Verify device belongs to user");
    println!("  ✓ Regenerate signature and compare");
    println!("  ✓ Process request if all checks pass\n");

    // ========================================================================
    // STEP 8: COMPLETE WORKFLOW
    // ========================================================================
    println!("--- STEP 8: Complete Sync Workflow ---");
    
    println!("Workflow Steps:");
    println!("  1. Agent detects file changes via inotify/ReadDirectoryChangesW");
    println!("  2. Agent collects file metadata (size, hash, timestamp, mime)");
    println!("  3. Agent creates authentication request:");
    println!("     - Adds JWT token");
    println!("     - Adds device_id");
    println!("     - Generates HMAC-SHA256 signature");
    println!("  4. Agent sends metadata to /api/metadata/batch");
    println!("  5. Agent uploads file content to /api/files/upload");
    println!("  6. Server receives and verifies each request");
    println!("  7. Server stores file with metadata");
    println!("  8. Server sends acknowledgment to agent\n");

    // ========================================================================
    // STEP 9: RESPONSE HANDLING
    // ========================================================================
    println!("--- STEP 9: Server Response Format ---");
    
    let response_example = serde_json::json!({
        "status": "success",
        "message": "File uploaded successfully",
        "data": {
            "file_id": "550e8400-e29b-41d4-a716-446655440000",
            "stored_at": "/storage/files/user_001/report.txt",
            "timestamp": timestamp
        }
    });
    
    println!("Response Structure:");
    println!("{}\n", serde_json::to_string_pretty(&response_example)?);

    // ========================================================================
    // SUMMARY
    // ========================================================================
    println!("=== Demo Summary ===");
    println!("✓ Authentication: JWT token + device signature");
    println!("✓ Metadata: File info (size, hash, type, timestamps)");
    println!("✓ Files: Base64 encoded file content");
    println!("✓ Batch: Multiple files in single request");
    println!("✓ Verification: Server validates JWT + device + signature");
    println!("\nReady for actual server implementation!");

    Ok(())
}
