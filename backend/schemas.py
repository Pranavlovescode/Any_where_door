"""
Pydantic schemas for request/response validation
"""
from datetime import datetime
from typing import Optional, List
from pydantic import BaseModel, Field


# ============================================================================
# Authentication Schemas
# ============================================================================

class LoginRequest(BaseModel):
    """User login request"""
    username: str
    password: str


class LoginResponse(BaseModel):
    """User login response"""
    jwt: str = Field(..., description="JWT token")
    user_id: str = Field(..., description="User ID")
    expires_in: int = Field(..., description="Expiration time in seconds")


class RegisterDeviceRequest(BaseModel):
    """Device registration request"""
    jwt: str = Field(..., description="JWT token from login")


class RegisterDeviceResponse(BaseModel):
    """Device registration response"""
    device_id: str = Field(..., description="Unique device ID")
    device_secret: str = Field(..., description="Device secret for signing")
    created_at: int = Field(..., description="Timestamp of creation")


# ============================================================================
# File Metadata Schemas
# ============================================================================

class FileMetadataRequest(BaseModel):
    """File metadata from agent"""
    file_path: str
    file_name: str
    file_size: int
    modified_at: int
    created_at: int
    file_hash: str
    mime_type: str
    is_directory: bool


class FileUploadPayload(BaseModel):
    """File upload with metadata"""
    metadata: FileMetadataRequest
    file_content: str = Field(..., description="Base64 encoded file content")
    source: str = Field("frontend", description="Source of upload: frontend or agent")


class MetadataBatchRequest(BaseModel):
    """Batch metadata request"""
    files: List[FileMetadataRequest]


class DirectoryMetadataRequest(BaseModel):
    """Directory metadata with files"""
    directory_path: str
    directory_name: str
    total_files: int
    total_size: int
    scanned_at: int
    files: List[FileMetadataRequest]


# ============================================================================
# Agent Schemas
# ============================================================================

class AgentInfoRequest(BaseModel):
    """Agent information"""
    agent_id: str
    agent_version: str
    os: str
    hostname: str
    sync_root: str
    last_sync: int
    status: str


class AgentStatusResponse(BaseModel):
    """Agent status response"""
    agent_id: str
    status: str
    last_sync: int
    files_synced: int
    total_size: int


# ============================================================================
# Authentication Request/Response
# ============================================================================

class AuthRequest(BaseModel):
    """Authenticated request from agent"""
    jwt: str
    device_id: str
    timestamp: int
    signature: str
    data: str = Field(..., description="JSON string of request data")


class ServerResponse(BaseModel):
    """Server response"""
    status: str = Field(..., description="success or error")
    message: str
    data: Optional[dict] = None


# ============================================================================
# File Response Schemas
# ============================================================================

class FileResponse(BaseModel):
    """File response after upload"""
    file_id: str
    stored_at: str
    timestamp: int
    size_bytes: int
    hash_verified: bool


class SyncResultResponse(BaseModel):
    """Sync result response"""
    total_files: int
    uploaded_files: int
    failed_files: int
    total_size: int
    errors: List[str] = []


# ============================================================================
# Database Response Models
# ============================================================================

class FileDB(BaseModel):
    """File database model for response"""
    file_id: str
    file_path: str
    file_name: str
    file_size: int
    file_hash: str
    mime_type: str
    stored_at: str
    uploaded_at: datetime
    
    class Config:
        from_attributes = True


class DeviceDB(BaseModel):
    """Device database model for response"""
    device_id: str
    created_at: datetime
    last_seen: datetime
    status: str
    
    class Config:
        from_attributes = True


class UserDB(BaseModel):
    """User database model for response"""
    user_id: str
    username: str
    created_at: datetime
    
    class Config:
        from_attributes = True
