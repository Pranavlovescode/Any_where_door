use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use chrono::Utc;
use crate::auth::{AuthService, AuthRequest};

// ============================================================================
// Data Structures
// ============================================================================

/// File metadata for transmission
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub modified_at: i64,
    pub created_at: i64,
    pub file_hash: String,           // SHA256 hash for integrity
    pub mime_type: String,
    pub is_directory: bool,
}

/// Metadata about a directory or collection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectoryMetadata {
    pub directory_path: String,
    pub directory_name: String,
    pub total_files: u64,
    pub total_size: u64,
    pub scanned_at: i64,
    pub files: Vec<FileMetadata>,
}

/// Agent information to be sent to server
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentInfo {
    pub agent_id: String,
    pub agent_version: String,
    pub os: String,
    pub hostname: String,
    pub sync_root: String,
    pub last_sync: i64,
    pub status: String,  // "online", "syncing", "idle", "error"
}

/// File upload request with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileUploadPayload {
    pub metadata: FileMetadata,
    pub file_content: String,  // Base64 encoded for JSON transfer
}

/// Metadata sync request (without file contents)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataSyncPayload {
    pub agent_info: AgentInfo,
    pub files: Vec<FileMetadata>,
}

/// Server response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
    pub status: String,           // "success", "error"
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Sync job for tracking uploads
#[derive(Debug, Clone)]
pub struct SyncJob {
    pub job_id: String,
    pub file_path: PathBuf,
    pub status: SyncStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Pending,
    Uploading,
    Completed,
    Failed(String),
}

// ============================================================================
// NetworkService: Central network handler
// ============================================================================

pub struct NetworkService {
    server_url: String,
    client: Client,
    #[allow(dead_code)]
    auth_service: AuthService,
    user_jwt: String,
    device_id: String,
    device_secret: String,
}

impl NetworkService {
    /// Initialize network service with server URL and authentication
    pub fn new(
        server_url: String,
        auth_service: AuthService,
        user_jwt: String,
        device_id: String,
        device_secret: String,
    ) -> Self {
        NetworkService {
            server_url,
            client: Client::new(),
            auth_service,
            user_jwt,
            device_id,
            device_secret,
        }
    }

    // ========================================================================
    // Metadata Operations
    // ========================================================================

    /// Send file metadata to server (without file contents)
    pub async fn send_file_metadata(&self, metadata: &FileMetadata) -> Result<ServerResponse, String> {
        let endpoint = format!("{}/api/metadata/file", self.server_url);
        let data = serde_json::to_string(metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        self.authenticated_post(&endpoint, &data).await
    }

    /// Send multiple files metadata in batch
    pub async fn send_metadata_batch(
        &self,
        files: Vec<FileMetadata>,
    ) -> Result<ServerResponse, String> {
        let endpoint = format!("{}/api/metadata/batch", self.server_url);
        let payload = serde_json::json!({ "files": files });
        let data = serde_json::to_string(&payload)
            .map_err(|e| format!("Failed to serialize batch: {}", e))?;

        self.authenticated_post(&endpoint, &data).await
    }

    /// Send directory metadata with all files in it
    pub async fn send_directory_metadata(
        &self,
        metadata: &DirectoryMetadata,
    ) -> Result<ServerResponse, String> {
        let endpoint = format!("{}/api/metadata/directory", self.server_url);
        let data = serde_json::to_string(metadata)
            .map_err(|e| format!("Failed to serialize directory: {}", e))?;

        self.authenticated_post(&endpoint, &data).await
    }

    /// Send agent info/status to server
    pub async fn send_agent_info(&self, agent_info: &AgentInfo) -> Result<ServerResponse, String> {
        let endpoint = format!("{}/api/agent/info", self.server_url);
        let data = serde_json::to_string(agent_info)
            .map_err(|e| format!("Failed to serialize agent info: {}", e))?;

        self.authenticated_post(&endpoint, &data).await
    }

    // ========================================================================
    // File Upload Operations
    // ========================================================================

    /// Upload a single file to the server
    pub async fn upload_file(&self, file_path: &Path) -> Result<ServerResponse, String> {
        // Get file metadata
        let metadata = self.extract_file_metadata(file_path)?;

        // Read file content
        let mut file = File::open(file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Encode to base64 for JSON transfer
        let encoded_content = base64_encode(&file_content);

        let payload = FileUploadPayload {
            metadata,
            file_content: encoded_content,
        };

        let endpoint = format!("{}/api/files/upload", self.server_url);
        let data = serde_json::to_string(&payload)
            .map_err(|e| format!("Failed to serialize upload payload: {}", e))?;

        self.authenticated_post(&endpoint, &data).await
    }

    /// Batch upload multiple files
    pub async fn upload_files(&self, file_paths: Vec<&Path>) -> Result<Vec<ServerResponse>, String> {
        let mut responses = Vec::new();

        for file_path in file_paths {
            match self.upload_file(file_path).await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    eprintln!("Failed to upload {}: {}", file_path.display(), e);
                    // Continue with next file instead of failing completely
                }
            }
        }

        if responses.is_empty() {
            return Err("All uploads failed".to_string());
        }

        Ok(responses)
    }

    // ========================================================================
    // Sync Operations
    // ========================================================================

    /// Perform complete sync: send metadata and files
    pub async fn sync_directory(
        &self,
        directory_path: &Path,
        agent_info: &AgentInfo,
    ) -> Result<SyncResult, String> {
        let mut sync_result = SyncResult {
            total_files: 0,
            uploaded_files: 0,
            failed_files: 0,
            total_size: 0,
            errors: Vec::new(),
        };

        // Step 1: Send agent info
        self.send_agent_info(agent_info).await?;

        // Step 2: Collect all files in directory
        let file_entries = self.collect_directory_files(directory_path)?;
        sync_result.total_files = file_entries.len() as u64;

        // Step 3: Send metadata for all files
        let metadatas: Vec<FileMetadata> = file_entries
            .iter()
            .filter_map(|entry| entry.metadata.clone())
            .collect();

        if !metadatas.is_empty() {
            self.send_metadata_batch(metadatas).await?;
        }

        // Step 4: Upload files
        for entry in file_entries {
            if let Some(path) = entry.path {
                match self.upload_file(&path).await {
                    Ok(_) => {
                        sync_result.uploaded_files += 1;
                        if let Some(metadata) = &entry.metadata {
                            sync_result.total_size += metadata.file_size;
                        }
                    }
                    Err(e) => {
                        sync_result.failed_files += 1;
                        sync_result.errors.push(format!("{}: {}", path.display(), e));
                    }
                }
            }
        }

        Ok(sync_result)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Create authenticated request with JWT + signature
    async fn authenticated_post(
        &self,
        endpoint: &str,
        data: &str,
    ) -> Result<ServerResponse, String> {
        // Create auth request
        let timestamp = Utc::now().timestamp();
        let signature = AuthService::generate_signature(&self.device_secret, &self.device_id, timestamp, data)
            .map_err(|e| format!("Failed to generate signature: {}", e))?;

        let auth_request = AuthRequest {
            jwt: self.user_jwt.clone(),
            device_id: self.device_id.clone(),
            timestamp,
            signature,
            data: data.to_string(),
        };

        // Send request with auth header
        let response = self
            .client
            .post(endpoint)
            .json(&auth_request)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let server_response = response
            .json::<ServerResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(server_response)
    }

    /// Extract metadata from a file
    fn extract_file_metadata(&self, file_path: &Path) -> Result<FileMetadata, String> {
        let metadata = std::fs::metadata(file_path)
            .map_err(|e| format!("Failed to read file metadata: {}", e))?;

        let file_hash = self.calculate_file_hash(file_path)?;

        Ok(FileMetadata {
            file_path: file_path.to_string_lossy().to_string(),
            file_name: file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_size: metadata.len(),
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            created_at: Utc::now().timestamp(),
            file_hash,
            mime_type: self.guess_mime_type(file_path),
            is_directory: metadata.is_dir(),
        })
    }

    /// Calculate SHA256 hash of file
    fn calculate_file_hash(&self, file_path: &Path) -> Result<String, String> {
        use sha2::{Sha256, Digest};
        
        let mut file = File::open(file_path)
            .map_err(|e| format!("Failed to open file for hashing: {}", e))?;

        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

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

    /// Guess MIME type based on file extension
    fn guess_mime_type(&self, file_path: &Path) -> String {
        match file_path.extension().and_then(|s| s.to_str()) {
            Some("txt") => "text/plain".to_string(),
            Some("json") => "application/json".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("png") => "image/png".to_string(),
            Some("zip") => "application/zip".to_string(),
            Some("gz") => "application/gzip".to_string(),
            Some(ext) => format!("application/{}", ext),
            None => "application/octet-stream".to_string(),
        }
    }

    /// Collect all files in directory recursively
    fn collect_directory_files(&self, dir_path: &Path) -> Result<Vec<DirectoryEntry>, String> {
        let mut entries = Vec::new();

        for entry in walkdir::WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path().to_path_buf();
                let metadata = self.extract_file_metadata(&path).ok();

                entries.push(DirectoryEntry {
                    path: Some(path),
                    metadata,
                });
            }
        }

        Ok(entries)
    }
}

// ============================================================================
// Supporting Structures
// ============================================================================

#[derive(Debug, Clone)]
struct DirectoryEntry {
    path: Option<PathBuf>,
    metadata: Option<FileMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult {
    pub total_files: u64,
    pub uploaded_files: u64,
    pub failed_files: u64,
    pub total_size: u64,
    pub errors: Vec<String>,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Base64 encode helper
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        let data = b"Hello, World!";
        let encoded = base64_encode(data);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_file_metadata_creation() {
        // This would require actual files to test
        // Demonstration only
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

        assert_eq!(metadata.file_name, "file.txt");
        assert_eq!(metadata.file_size, 1024);
    }
}
