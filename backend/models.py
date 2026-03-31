"""
SQLAlchemy models for database tables
"""
from datetime import datetime
from sqlalchemy import Column, String, Integer, DateTime, Boolean, ForeignKey, Text
from sqlalchemy.orm import relationship
from database import Base
import uuid


class User(Base):
    """User account model"""
    __tablename__ = "users"
    
    user_id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    username = Column(String(255), unique=True, nullable=False, index=True)
    password_hash = Column(String(255), nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow)
    
    # Relationships
    devices = relationship("Device", back_populates="user", cascade="all, delete-orphan")
    files = relationship("File", back_populates="user", cascade="all, delete-orphan")
    agents = relationship("Agent", back_populates="user", cascade="all, delete-orphan")


class Device(Base):
    """Device registration for API signing"""
    __tablename__ = "devices"
    
    device_id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    device_secret = Column(String(64), nullable=False)  # 32-byte hex string
    user_id = Column(String(36), ForeignKey("users.user_id"), nullable=False, index=True)
    status = Column(String(50), default="active")  # active, inactive, revoked
    created_at = Column(DateTime, default=datetime.utcnow)
    last_seen = Column(DateTime, default=datetime.utcnow)
    
    # Relationships
    user = relationship("User", back_populates="devices")
    file_syncs = relationship("FileSync", back_populates="device")


class File(Base):
    """File metadata and storage tracking"""
    __tablename__ = "files"
    
    file_id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = Column(String(36), ForeignKey("users.user_id"), nullable=False, index=True)
    file_path = Column(String(1024), nullable=False)  # Original path from agent
    file_name = Column(String(255), nullable=False)
    file_size = Column(Integer, nullable=False)  # Bytes
    file_hash = Column(String(64), nullable=False, index=True)  # SHA256
    mime_type = Column(String(100), default="application/octet-stream")
    stored_at = Column(String(1024), nullable=False)  # Filesystem path or "pending"
    uploaded_at = Column(DateTime, default=datetime.utcnow)
    
    # Relationships
    user = relationship("User", back_populates="files")
    file_syncs = relationship("FileSync", back_populates="file")


class FileMetadata(Base):
    """Extended metadata for files"""
    __tablename__ = "file_metadata"
    
    metadata_id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    file_id = Column(String(36), ForeignKey("files.file_id"), nullable=False, index=True)
    original_path = Column(String(1024), nullable=True)
    tags = Column(Text, nullable=True)  # JSON string
    description = Column(Text, nullable=True)
    version = Column(Integer, default=1)
    created_at = Column(DateTime, default=datetime.utcnow)


class FileSync(Base):
    """Track file syncs from agents"""
    __tablename__ = "file_syncs"
    
    sync_id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    agent_id = Column(String(36), ForeignKey("agents.agent_id"), nullable=False, index=True)
    device_id = Column(String(36), ForeignKey("devices.device_id"), nullable=True)
    file_id = Column(String(36), ForeignKey("files.file_id"), nullable=False, index=True)
    file_size = Column(Integer, nullable=False)
    status = Column(String(50), default="pending")  # pending, success, failed
    synced_at = Column(DateTime, default=datetime.utcnow)
    
    # Relationships
    agent = relationship("Agent", back_populates="file_syncs")
    device = relationship("Device", back_populates="file_syncs")
    file = relationship("File", back_populates="file_syncs")


class Agent(Base):
    """Agent registration and status"""
    __tablename__ = "agents"
    
    agent_id = Column(String(36), primary_key=True, default=lambda: str(uuid.uuid4()))
    user_id = Column(String(36), ForeignKey("users.user_id"), nullable=False, index=True)
    device_id = Column(String(36), ForeignKey("devices.device_id"), nullable=True)
    agent_version = Column(String(20), nullable=False)
    os = Column(String(50), nullable=False)  # Linux, Windows, macOS, etc.
    hostname = Column(String(255), nullable=False)
    sync_root = Column(String(1024), nullable=False)  # Root directory being synced
    status = Column(String(50), default="active")  # active, inactive, paused
    last_sync = Column(DateTime, default=datetime.utcnow)
    created_at = Column(DateTime, default=datetime.utcnow)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    
    # Relationships
    user = relationship("User", back_populates="agents")
    file_syncs = relationship("FileSync", back_populates="agent", cascade="all, delete-orphan")
