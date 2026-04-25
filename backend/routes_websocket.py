"""
WebSocket routes for real-time file synchronization
"""
import logging
from fastapi import APIRouter, WebSocket, WebSocketDisconnect, HTTPException, Query
from auth_utils import verify_jwt
from sync_broadcast import get_broadcaster

logger = logging.getLogger("anywhere_door.sync")

router = APIRouter(prefix="/ws", tags=["websocket"])


@router.websocket("/sync/{jwt_token}")
async def websocket_sync(websocket: WebSocket, jwt_token: str):
    """
    WebSocket endpoint for real-time file sync events.
    Client connects with JWT token for authentication.
    """
    try:
        # Verify JWT before accepting connection
        payload = verify_jwt(jwt_token)
        user_id = payload.get("user_id")
        
        if not user_id:
            await websocket.close(code=4001, reason="Invalid token: no user_id")
            return
        
        logger.info(f"WebSocket connection attempt from user {user_id[:8]}...")
        
    except Exception as e:
        logger.warning(f"WebSocket auth failed: {e}")
        await websocket.close(code=4001, reason="Unauthorized")
        return
    
    # Accept connection
    await websocket.accept()
    broadcaster = get_broadcaster()
    broadcaster.add_connection(user_id, websocket)
    
    try:
        # Keep connection alive and listen for client messages (heartbeat/ping)
        while True:
            # Receive from client (mostly for heartbeat/keep-alive)
            data = await websocket.receive_text()
            if data == "ping":
                await websocket.send_text("pong")
    
    except WebSocketDisconnect:
        logger.info(f"WebSocket disconnected for user {user_id[:8]}...")
        await broadcaster.remove_connection(user_id, websocket)
    
    except Exception as e:
        logger.error(f"WebSocket error: {e}")
        try:
            await broadcaster.remove_connection(user_id, websocket)
        except Exception:
            pass
