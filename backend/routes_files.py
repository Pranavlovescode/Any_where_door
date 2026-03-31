"""
File API routes
"""
import base64
import hashlib
from datetime import datetime
from typing import List
from fastapi import APIRouter, HTTPException, Depends
from sqlalchemy.orm import Session
import uuid

from database import get_db
from schemas import FileUploadPayload, FileResponse, MetadataBatchRequest
from models import File, FileMetadata, Device
from auth_utils import verify_jwt
from config import Settings

router = APIRouter(prefix="/api/files", tags=["files"])
settings = Settings()


def verify_device_from_jwt(jwt_token: str, db: Session) -> tuple:
    """
    Verify JWT token and return user_id and device info
    """
    try:
        payload = verify_jwt(jwt_token)
        user_id = payload.get("user_id")
        
        if not user_id:
            raise HTTPException(status_code=401, detail="Invalid token: no user_id")
        
        return user_id
    except Exception as e:
        raise HTTPException(status_code=401, detail=f"Invalid token: {str(e)}")


@router.post("/upload", response_model=FileResponse)
async def upload_file(payload: FileUploadPayload, jwt: str = None, db: Session = Depends(get_db)):
    """
    Upload a file with metadata
    File content is Base64 encoded in the request
    """
    # Extract user_id from JWT
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = verify_device_from_jwt(jwt, db)
    
    # Get metadata
    metadata = payload.metadata
    
    # Decode file content from Base64
    try:
        file_content = base64.b64decode(payload.file_content)
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Invalid Base64 encoding: {str(e)}")
    
    # Verify file hash
    calculated_hash = hashlib.sha256(file_content).hexdigest()
    if calculated_hash != metadata.file_hash:
        raise HTTPException(status_code=400, detail="File hash mismatch - data integrity check failed")
    
    # Create storage path: storage/files/user_id/file_name
    import os
    file_id = str(uuid.uuid4())
    storage_dir = os.path.join(settings.FILE_STORAGE_DIR, user_id)
    os.makedirs(storage_dir, exist_ok=True)
    
    file_path = os.path.join(storage_dir, f"{file_id}_{metadata.file_name}")
    
    # Write file to disk
    try:
        with open(file_path, "wb") as f:
            f.write(file_content)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to write file: {str(e)}")
    
    # Create file record in database
    file_record = File(
        file_id=file_id,
        user_id=user_id,
        file_path=metadata.file_path,
        file_name=metadata.file_name,
        file_size=metadata.file_size,
        file_hash=metadata.file_hash,
        mime_type=metadata.mime_type,
        stored_at=file_path,
        uploaded_at=datetime.utcnow()
    )
    
    db.add(file_record)
    db.commit()
    db.refresh(file_record)
    
    return FileResponse(
        file_id=file_id,
        stored_at=file_path,
        timestamp=int(file_record.uploaded_at.timestamp()),
        size_bytes=metadata.file_size,
        hash_verified=True
    )


@router.post("/batch-metadata")
async def batch_metadata(request: MetadataBatchRequest, jwt: str = None, db: Session = Depends(get_db)):
    """
    Receive batch of file metadata (for sync tracking)
    Does not require file content, just metadata
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = verify_device_from_jwt(jwt, db)
    
    created_count = 0
    errors = []
    
    for metadata in request.files:
        try:
            # Check if file already exists by hash
            existing = db.query(File).filter(
                File.user_id == user_id,
                File.file_hash == metadata.file_hash
            ).first()
            
            if not existing:
                # Create metadata record
                file_record = File(
                    file_id=str(uuid.uuid4()),
                    user_id=user_id,
                    file_path=metadata.file_path,
                    file_name=metadata.file_name,
                    file_size=metadata.file_size,
                    file_hash=metadata.file_hash,
                    mime_type=metadata.mime_type,
                    stored_at="pending",  # Not yet uploaded
                    uploaded_at=datetime.utcnow()
                )
                
                db.add(file_record)
                created_count += 1
        except Exception as e:
            errors.append(f"{metadata.file_name}: {str(e)}")
    
    db.commit()
    
    return {
        "status": "success",
        "total_files": len(request.files),
        "created_files": created_count,
        "errors": errors
    }


@router.get("/list")
async def list_files(jwt: str = None, limit: int = 100, skip: int = 0, db: Session = Depends(get_db)):
    """
    List files for the authenticated user
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = verify_device_from_jwt(jwt, db)
    
    # Query files
    files = db.query(File).filter(File.user_id == user_id).offset(skip).limit(limit).all()
    
    return {
        "status": "success",
        "total": len(files),
        "files": [
            {
                "file_id": f.file_id,
                "file_name": f.file_name,
                "file_path": f.file_path,
                "file_size": f.file_size,
                "file_hash": f.file_hash,
                "mime_type": f.mime_type,
                "uploaded_at": int(f.uploaded_at.timestamp())
            }
            for f in files
        ]
    }


@router.get("/{file_id}/download")
async def download_file(file_id: str, jwt: str = None, db: Session = Depends(get_db)):
    """
    Download a file if user owns it
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = verify_device_from_jwt(jwt, db)
    
    # Find file
    file_record = db.query(File).filter(
        File.file_id == file_id,
        File.user_id == user_id
    ).first()
    
    if not file_record:
        raise HTTPException(status_code=404, detail="File not found")
    
    # Read file and return Base64 encoded
    try:
        with open(file_record.stored_at, "rb") as f:
            content = f.read()
        
        encoded_content = base64.b64encode(content).decode('utf-8')
        
        return {
            "status": "success",
            "file_id": file_id,
            "file_name": file_record.file_name,
            "file_content": encoded_content,
            "file_hash": file_record.file_hash,
            "mime_type": file_record.mime_type
        }
    except FileNotFoundError:
        raise HTTPException(status_code=404, detail="File not found on disk")
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to read file: {str(e)}")


@router.delete("/{file_id}")
async def delete_file(file_id: str, jwt: str = None, db: Session = Depends(get_db)):
    """
    Delete a file if user owns it
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = verify_device_from_jwt(jwt, db)
    
    # Find file
    file_record = db.query(File).filter(
        File.file_id == file_id,
        File.user_id == user_id
    ).first()
    
    if not file_record:
        raise HTTPException(status_code=404, detail="File not found")
    
    # Delete from disk
    import os
    try:
        if os.path.exists(file_record.stored_at):
            os.remove(file_record.stored_at)
    except Exception as e:
        return {
            "status": "error",
            "message": f"Failed to delete file from disk: {str(e)}"
        }
    
    # Delete from database
    db.delete(file_record)
    db.commit()
    
    return {
        "status": "success",
        "message": "File deleted"
    }
