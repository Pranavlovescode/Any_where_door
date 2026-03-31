"""
Authentication API routes
"""
from fastapi import APIRouter, HTTPException, Depends
from datetime import datetime
import uuid

from database import get_db
from schemas import LoginRequest, LoginResponse, RegisterDeviceRequest, RegisterDeviceResponse
from models import User, Device
from auth_utils import create_jwt, hash_password, verify_password, generate_device_secret

router = APIRouter(prefix="/auth", tags=["auth"])


@router.post("/login", response_model=LoginResponse)
async def login(request: LoginRequest, db=Depends(get_db)):
    """
    User login endpoint
    Returns JWT token and user ID
    """
    # Find user in database
    user = db.query(User).filter(User.username == request.username).first()
    
    if not user:
        raise HTTPException(status_code=401, detail="Invalid credentials")
    
    # Verify password
    if not verify_password(request.password, user.password_hash):
        raise HTTPException(status_code=401, detail="Invalid credentials")
    
    # Create JWT token
    token, expires_in = create_jwt(user.user_id, user.username)
    
    return LoginResponse(
        jwt=token,
        user_id=user.user_id,
        expires_in=expires_in
    )


@router.post("/register-device", response_model=RegisterDeviceResponse)
async def register_device(request: RegisterDeviceRequest, db=Depends(get_db)):
    """
    Register a new device for the user
    Returns device_id and device_secret for future request signing
    """
    try:
        # Verify JWT token
        from auth_utils import verify_jwt
        jwt_payload = verify_jwt(request.jwt)
    except Exception as e:
        raise HTTPException(status_code=401, detail=f"Invalid JWT: {str(e)}")
    
    user_id = jwt_payload.get("user_id")
    
    # Find user
    user = db.query(User).filter(User.user_id == user_id).first()
    if not user:
        raise HTTPException(status_code=404, detail="User not found")
    
    # Generate device credentials
    device_id = str(uuid.uuid4())
    device_secret = generate_device_secret()
    
    # Create device in database
    device = Device(
        device_id=device_id,
        device_secret=device_secret,
        user_id=user_id,
        status="active",
        last_seen=datetime.utcnow()
    )
    
    db.add(device)
    db.commit()
    db.refresh(device)
    
    return RegisterDeviceResponse(
        device_id=device_id,
        device_secret=device_secret,
        created_at=int(device.created_at.timestamp())
    )


@router.post("/create-user")
async def create_user(username: str, password: str, db=Depends(get_db)):
    """
    Create a new user (for testing/initial setup)
    """
    # Check if user exists
    existing = db.query(User).filter(User.username == username).first()
    if existing:
        raise HTTPException(status_code=409, detail="Username already exists")
    
    # Create new user
    user_id = str(uuid.uuid4())
    password_hash = hash_password(password)
    
    user = User(
        user_id=user_id,
        username=username,
        password_hash=password_hash,
        created_at=datetime.utcnow()
    )
    
    db.add(user)
    db.commit()
    db.refresh(user)
    
    return {
        "user_id": user.user_id,
        "username": user.username,
        "status": "created"
    }
