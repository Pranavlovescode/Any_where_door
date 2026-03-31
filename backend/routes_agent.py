"""
Agent API routes
"""
from datetime import datetime
from fastapi import APIRouter, HTTPException, Depends
from sqlalchemy.orm import Session
import uuid

from database import get_db
from schemas import AgentInfoRequest, AgentStatusResponse
from models import Agent, Device
from auth_utils import verify_jwt

router = APIRouter(prefix="/api/agent", tags=["agent"])


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


@router.post("/register")
async def register_agent(
    agent_info: AgentInfoRequest,
    jwt: str = None,
    db: Session = Depends(get_db)
):
    """
    Register or update agent information
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    # Find existing agent or create new
    agent = db.query(Agent).filter(
        Agent.agent_id == agent_info.agent_id,
        Agent.user_id == user_id
    ).first()
    
    if agent:
        # Update existing agent
        agent.agent_version = agent_info.agent_version
        agent.os = agent_info.os
        agent.hostname = agent_info.hostname
        agent.sync_root = agent_info.sync_root
        agent.status = agent_info.status
        agent.last_sync = datetime.fromtimestamp(agent_info.last_sync)
        agent.updated_at = datetime.utcnow()
    else:
        # Create new agent
        agent = Agent(
            agent_id=agent_info.agent_id,
            user_id=user_id,
            device_id=None,  # Can be set later
            agent_version=agent_info.agent_version,
            os=agent_info.os,
            hostname=agent_info.hostname,
            sync_root=agent_info.sync_root,
            status=agent_info.status,
            last_sync=datetime.fromtimestamp(agent_info.last_sync),
            created_at=datetime.utcnow(),
            updated_at=datetime.utcnow()
        )
        db.add(agent)
    
    db.commit()
    db.refresh(agent)
    
    return {
        "status": "success",
        "message": "Agent registered",
        "agent_id": agent.agent_id
    }


@router.get("/{agent_id}/status", response_model=AgentStatusResponse)
async def get_agent_status(
    agent_id: str,
    jwt: str = None,
    db: Session = Depends(get_db)
):
    """
    Get agent status and sync information
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    # Find agent
    agent = db.query(Agent).filter(
        Agent.agent_id == agent_id,
        Agent.user_id == user_id
    ).first()
    
    if not agent:
        raise HTTPException(status_code=404, detail="Agent not found")
    
    # Count synced files
    from models import FileSync
    synced_files = db.query(FileSync).filter(
        FileSync.agent_id == agent_id,
        FileSync.status == "success"
    ).count()
    
    # Calculate total size
    total_size = db.query(FileSync).filter(
        FileSync.agent_id == agent_id
    ).with_entities(
        db.func.sum(FileSync.file_size)
    ).scalar() or 0
    
    return AgentStatusResponse(
        agent_id=agent_id,
        status=agent.status,
        last_sync=int(agent.last_sync.timestamp()),
        files_synced=synced_files,
        total_size=total_size
    )


@router.get("")
async def list_agents(jwt: str = None, db: Session = Depends(get_db)):
    """
    List all agents for the user
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    agents = db.query(Agent).filter(Agent.user_id == user_id).all()
    
    return {
        "status": "success",
        "total": len(agents),
        "agents": [
            {
                "agent_id": a.agent_id,
                "hostname": a.hostname,
                "os": a.os,
                "status": a.status,
                "last_sync": int(a.last_sync.timestamp()),
                "sync_root": a.sync_root
            }
            for a in agents
        ]
    }


@router.post("/update-status")
async def update_agent_status(
    agent_id: str,
    status: str,
    jwt: str = None,
    db: Session = Depends(get_db)
):
    """
    Update agent status
    """
    if not jwt:
        raise HTTPException(status_code=401, detail="Missing JWT token")
    
    user_id = get_user_from_jwt(jwt, db)
    
    # Find agent
    agent = db.query(Agent).filter(
        Agent.agent_id == agent_id,
        Agent.user_id == user_id
    ).first()
    
    if not agent:
        raise HTTPException(status_code=404, detail="Agent not found")
    
    agent.status = status
    agent.updated_at = datetime.utcnow()
    
    db.commit()
    
    return {
        "status": "success",
        "message": "Agent status updated",
        "agent_id": agent_id,
        "new_status": status
    }
