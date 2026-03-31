"""
Configuration settings for AnywhereDoor Server
Uses Pydantic Settings for environment-based configuration
"""
from pydantic_settings import BaseSettings
from typing import Optional
import os


class Settings(BaseSettings):
    """
    Application settings loaded from environment variables or .env file
    """
    
    # API Settings
    API_TITLE: str = "AnywhereDoor Server"
    API_VERSION: str = "1.0.0"
    API_DESCRIPTION: str = "File sync and management server for AnywhereDoor agent"
    
    # Database Settings
    DATABASE_URL: str = "sqlite:///./any_where_door.db"
    
    # JWT Settings
    JWT_SECRET: str = "your-super-secret-key-change-this-in-production"
    JWT_ALGORITHM: str = "HS256"
    JWT_EXPIRATION_HOURS: int = 24
    
    # File Storage Settings
    FILE_STORAGE_DIR: str = "./storage/files"
    MAX_FILE_SIZE_MB: int = 500
    
    # Server Settings
    HOST: str = "0.0.0.0"
    PORT: int = 8000
    RELOAD: bool = True
    LOG_LEVEL: str = "info"
    
    # CORS Settings
    ALLOWED_ORIGINS: str = "*"
    
    class Config:
        env_file = ".env"
        case_sensitive = True
    
    def __init__(self, **data):
        super().__init__(**data)
        # Create storage directory if it doesn't exist
        os.makedirs(self.FILE_STORAGE_DIR, exist_ok=True)


# Create settings instance
settings = Settings()
