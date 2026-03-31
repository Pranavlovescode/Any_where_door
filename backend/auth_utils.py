"""
Authentication utilities for JWT verification and device signature validation
"""
import hmac
import hashlib
import json
from datetime import datetime, timedelta
from typing import Optional, Tuple
import jwt

from config import Settings

settings = Settings()


class AuthError(Exception):
    """Authentication error"""
    pass


def create_jwt(user_id: str, username: str, expires_in_hours: int = 24) -> Tuple[str, int]:
    """
    Create JWT token for user
    
    Args:
        user_id: User ID
        username: Username
        expires_in_hours: Expiration time in hours
        
    Returns:
        Tuple of (token, expiration_timestamp)
    """
    now = datetime.utcnow()
    expires_at = now + timedelta(hours=expires_in_hours)
    expires_in = int(expires_at.timestamp())
    
    payload = {
        "user_id": user_id,
        "username": username,
        "iat": int(now.timestamp()),
        "exp": int(expires_at.timestamp()),
    }
    
    token = jwt.encode(
        payload,
        settings.JWT_SECRET,
        algorithm=settings.JWT_ALGORITHM
    )
    
    return token, expires_in


def verify_jwt(token: str) -> Optional[dict]:
    """
    Verify and decode JWT token
    
    Args:
        token: JWT token
        
    Returns:
        Decoded token payload or None if invalid
        
    Raises:
        AuthError: If token is invalid/expired
    """
    try:
        payload = jwt.decode(
            token,
            settings.JWT_SECRET,
            algorithms=[settings.JWT_ALGORITHM]
        )
        return payload
    except jwt.ExpiredSignatureError:
        raise AuthError("Token expired")
    except jwt.InvalidTokenError as e:
        raise AuthError(f"Invalid token: {str(e)}")


def extract_user_from_jwt(token: str) -> Optional[str]:
    """
    Extract user_id from JWT token
    
    Args:
        token: JWT token
        
    Returns:
        User ID or None if invalid
    """
    try:
        payload = verify_jwt(token)
        return payload.get("user_id")
    except AuthError:
        return None


def generate_device_secret() -> str:
    """
    Generate a device secret for signing requests
    
    Returns:
        32-character hex string (256 bits)
    """
    import secrets
    return secrets.token_hex(16)  # 32 character hex string


def create_signature(
    device_secret: str,
    device_id: str,
    timestamp: int,
    data: str
) -> str:
    """
    Create HMAC-SHA256 signature for request
    
    Args:
        device_secret: Device secret from registration
        device_id: Device ID
        timestamp: Request timestamp
        data: Request data JSON string
        
    Returns:
        Hex-encoded signature
    """
    message = f"{device_id}:{timestamp}:{data}"
    signature = hmac.new(
        device_secret.encode(),
        message.encode(),
        hashlib.sha256
    ).hexdigest()
    return signature


def verify_signature(
    device_secret: str,
    device_id: str,
    timestamp: int,
    data: str,
    signature: str,
    max_age_seconds: int = 300
) -> bool:
    """
    Verify device request signature
    
    Args:
        device_secret: Device secret from database
        device_id: Device ID
        timestamp: Request timestamp
        data: Request data JSON string
        signature: Signature from request
        max_age_seconds: Maximum age of timestamp (5 minutes default)
        
    Returns:
        True if signature is valid, False otherwise
        
    Raises:
        AuthError: If timestamp is too old
    """
    # Check timestamp freshness (prevent replay attacks)
    now = int(datetime.utcnow().timestamp())
    age = now - timestamp
    
    if age < -60:  # Allow 1 minute clock skew forward
        raise AuthError("Request timestamp is from the future")
    
    if age > max_age_seconds:
        raise AuthError(f"Request is too old ({age} seconds)")
    
    # Verify signature
    expected_signature = create_signature(device_secret, device_id, timestamp, data)
    
    # Use constant-time comparison to prevent timing attacks
    return hmac.compare_digest(signature, expected_signature)


def verify_auth_request(
    jwt_token: str,
    device_id: str,
    timestamp: int,
    signature: str,
    data_str: str,
    device_secret: str
) -> Tuple[bool, Optional[str]]:
    """
    Verify complete authentication request
    
    Args:
        jwt_token: JWT token
        device_id: Device ID
        timestamp: Request timestamp
        signature: Request signature
        data_str: Request data JSON string
        device_secret: Device secret from database
        
    Returns:
        Tuple of (is_valid, error_message)
    """
    try:
        # Verify JWT
        jwt_payload = verify_jwt(jwt_token)
        if not jwt_payload:
            return False, "Invalid JWT token"
        
        # Verify timestamp and signature
        verify_signature(device_secret, device_id, timestamp, data_str, signature)
        
        return True, None
        
    except AuthError as e:
        return False, str(e)


def hash_password(password: str) -> str:
    """
    Hash password using SHA256
    
    Args:
        password: Plain password
        
    Returns:
        Hex-encoded hash
    """
    return hashlib.sha256(password.encode()).hexdigest()


def verify_password(password: str, password_hash: str) -> bool:
    """
    Verify password against hash
    
    Args:
        password: Plain password
        password_hash: Hash from database
        
    Returns:
        True if password matches
    """
    return hmac.compare_digest(hash_password(password), password_hash)
