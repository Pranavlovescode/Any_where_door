"""
Sync API routes for file synchronization management
"""
from datetime import datetime
from fastapi import APIRouter, HTTPException, Depends
from sqlalchemy.orm import Session
import uuid

from database import get_db
from schemas import DirectoryMetadataRequest
from models import File, FileSync, Agent
from auth_utils import verify_jwt

router = APIRouter(prefix="/api/sync", tags=["sync"])


def get_user_from_jwt(jwt_token: str, db: Session) -> str:
    """
    Extract and verify user_id from JWT
    """
    try:
        payload = verify_jwt(jwt_token)
        user_id = payload.get("user_id")
        if not user_id:
            raise HTTPException(status_code=401, detail="Invalid token")
        return user_id
    except Exception as e:
        raise HTTPException(status_code=401, detail=f"Invalid token: {str(e)}")


@router.post("/directory")
async def sync_directory(
    payload: DirectoryMetadataRequest,
    agent_id: str = None,
    jwt: str = None,
    db: Session = Depends(get_db)
):
    """
    Sync entire directory with metadata
    Records the sync event and tracks all files in the directory
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    if not agent_id:
        raise HTTPException(status_code=400, detail="Missing agent_id")
    
    # Verify agent belongs to user
    agent = db.query(Agent).filter(
        Agent.agent_id == agent_id,
        Agent.user_id == user_id
    ).first()
    
    if not agent:
        raise HTTPException(status_code=404, detail="Agent not found")
    
    # Process directory files
    synced_count = 0
    failed_count = 0
    
    for file_metadata in payload.files:
        try:
            # Check if file already exists
            existing_file = db.query(File).filter(
                File.user_id == user_id,
                File.file_hash == file_metadata.file_hash
            ).first()
            
            file_id = existing_file.file_id if existing_file else str(uuid.uuid4())
            
            # Create or update file record
            if not existing_file:
                file_record = File(
                    file_id=file_id,
                    user_id=user_id,
                    file_path=file_metadata.file_path,
                    file_name=file_metadata.file_name,
                    file_size=file_metadata.file_size,
                    file_hash=file_metadata.file_hash,
                    mime_type=file_metadata.mime_type,
                    stored_at="pending",
                    uploaded_at=datetime.utcnow()
                )
                db.add(file_record)
                db.flush()
            
            # Create sync record
            sync_record = FileSync(
                sync_id=str(uuid.uuid4()),
                agent_id=agent_id,
                file_id=file_id,
                file_size=file_metadata.file_size,
                status="pending",  # Waiting for actual file upload
                synced_at=datetime.utcnow()
            )
            db.add(sync_record)
            synced_count += 1
            
        except Exception as e:
            failed_count += 1
            print(f"Failed to sync {file_metadata.file_name}: {str(e)}")
    
    # Update agent sync time
    agent.last_sync = datetime.utcnow()
    
    db.commit()
    
    return {
        "status": "success",
        "directory": payload.directory_path,
        "total_files": payload.total_files,
        "synced_files": synced_count,
        "failed_files": failed_count,
        "total_size": payload.total_size
    }


@router.post("/file-sync")
async def sync_file(
    file_id: str,
    agent_id: str = None,
    jwt: str = None,
    db: Session = Depends(get_db)
):
    """
    Mark a file as successfully synced
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    if not agent_id:
        raise HTTPException(status_code=400, detail="Missing agent_id")
    
    # Find file
    file_record = db.query(File).filter(
        File.file_id == file_id,
        File.user_id == user_id
    ).first()
    
    if not file_record:
        raise HTTPException(status_code=404, detail="File not found")
    
    # Find or create sync record
    sync_record = db.query(FileSync).filter(
        FileSync.file_id == file_id,
        FileSync.agent_id == agent_id
    ).first()
    
    if sync_record:
        sync_record.status = "success"
        sync_record.synced_at = datetime.utcnow()
    else:
        sync_record = FileSync(
            sync_id=str(uuid.uuid4()),
            agent_id=agent_id,
            file_id=file_id,
            file_size=file_record.file_size,
            status="success",
            synced_at=datetime.utcnow()
        )
        db.add(sync_record)
    
    db.commit()
    
    return {
        "status": "success",
        "file_id": file_id,
        "sync_status": "success"
    }


@router.get("/status/{agent_id}")
async def get_sync_status(
    agent_id: str,
    jwt: str = None,
    db: Session = Depends(get_db)
):
    """
    Get sync status for an agent
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    # Verify agent belongs to user
    agent = db.query(Agent).filter(
        Agent.agent_id == agent_id,
        Agent.user_id == user_id
    ).first()
    
    if not agent:
        raise HTTPException(status_code=404, detail="Agent not found")
    
    # Get sync statistics
    total_syncs = db.query(FileSync).filter(
        FileSync.agent_id == agent_id
    ).count()
    
    successful_syncs = db.query(FileSync).filter(
        FileSync.agent_id == agent_id,
        FileSync.status == "success"
    ).count()
    
    failed_syncs = db.query(FileSync).filter(
        FileSync.agent_id == agent_id,
        FileSync.status == "failed"
    ).count()
    
    pending_syncs = db.query(FileSync).filter(
        FileSync.agent_id == agent_id,
        FileSync.status == "pending"
    ).count()
    
    total_size = db.query(FileSync).filter(
        FileSync.agent_id == agent_id,
        FileSync.status == "success"
    ).with_entities(
        db.func.sum(FileSync.file_size)
    ).scalar() or 0
    
    return {
        "status": "success",
        "agent_id": agent_id,
        "total_syncs": total_syncs,
        "successful_syncs": successful_syncs,
        "failed_syncs": failed_syncs,
        "pending_syncs": pending_syncs,
        "total_size_bytes": total_size,
        "last_sync": int(agent.last_sync.timestamp()) if agent.last_sync else None
    }


@router.post("/mark-failed")
async def mark_file_failed(
    file_id: str,
    agent_id: str = None,
    error_message: str = None,
    jwt: str = None,
    db: Session = Depends(get_db)
):
    """
    Mark a file sync as failed
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    if not agent_id:
        raise HTTPException(status_code=400, detail="Missing agent_id")
    
    # Find or create sync record
    sync_record = db.query(FileSync).filter(
        FileSync.file_id == file_id,
        FileSync.agent_id == agent_id
    ).first()
    
    if sync_record:
        sync_record.status = "failed"
        sync_record.synced_at = datetime.utcnow()
    else:
        # Get file size from file record
        file_record = db.query(File).filter(
            File.file_id == file_id,
            File.user_id == user_id
        ).first()
        
        file_size = file_record.file_size if file_record else 0
        
        sync_record = FileSync(
            sync_id=str(uuid.uuid4()),
            agent_id=agent_id,
            file_id=file_id,
            file_size=file_size,
            status="failed",
            synced_at=datetime.utcnow()
        )
        db.add(sync_record)
    
    db.commit()
    
    return {
        "status": "success",
        "file_id": file_id,
        "sync_status": "failed",
        "error": error_message
    }
