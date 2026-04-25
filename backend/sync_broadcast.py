"""
WebSocket broadcast manager for real-time file synchronization events
"""
import json
import logging
from typing import Dict, Set
from fastapi import WebSocket, WebSocketDisconnect
from datetime import datetime

logger = logging.getLogger("anywhere_door.sync")


class SyncEventBroadcaster:
    """
    Manages WebSocket connections and broadcasts file sync events to all connected clients.
    """

    def __init__(self):
        # user_id -> set of WebSocket connections
        self.active_connections: Dict[str, Set[WebSocket]] = {}
        self.lock = None

    def add_connection(self, user_id: str, websocket: WebSocket):
        """Register a new WebSocket connection for a user"""
        if user_id not in self.active_connections:
            self.active_connections[user_id] = set()
        self.active_connections[user_id].add(websocket)
        logger.info(f"WebSocket client connected for user {user_id[:8]}... ({len(self.active_connections[user_id])} active)")

    async def remove_connection(self, user_id: str, websocket: WebSocket):
        """Unregister a WebSocket connection"""
        if user_id in self.active_connections:
            self.active_connections[user_id].discard(websocket)
            if not self.active_connections[user_id]:
                del self.active_connections[user_id]
            logger.info(f"WebSocket client disconnected for user {user_id[:8]}...")

    async def broadcast_file_event(
        self,
        user_id: str,
        event_type: str,  # "upload", "delete", "update", "scan"
        file_id: str,
        file_name: str,
        file_path: str,
        file_size: int,
        source: str = "frontend",  # "agent" or "frontend"
        timestamp: float = None,
    ):
        """
        Broadcast a file change event to all connected clients for a user
        """
        if timestamp is None:
            timestamp = datetime.utcnow().timestamp()

        event = {
            "type": "file_event",
            "event_type": event_type,
            "file_id": file_id,
            "file_name": file_name,
            "file_path": file_path,
            "file_size": file_size,
            "source": source,
            "timestamp": timestamp,
        }

        message = json.dumps(event)
        logger.info(f"Broadcasting {event_type} event for {file_name} (source: {source}) to user {user_id[:8]}...")

        # Send to all connected clients for this user
        if user_id in self.active_connections:
            disconnected = set()
            for ws in self.active_connections[user_id]:
                try:
                    await ws.send_text(message)
                except Exception as e:
                    logger.warning(f"Failed to broadcast to client: {e}")
                    disconnected.add(ws)

            # Clean up disconnected clients
            for ws in disconnected:
                await self.remove_connection(user_id, ws)

    async def broadcast_sync_status(
        self,
        user_id: str,
        status: str,  # "syncing", "synced", "error"
        file_count: int = 0,
        message: str = "",
    ):
        """Broadcast sync status update"""
        event = {
            "type": "sync_status",
            "status": status,
            "file_count": file_count,
            "message": message,
            "timestamp": datetime.utcnow().timestamp(),
        }

        message_str = json.dumps(event)
        logger.info(f"Broadcasting sync status '{status}' to user {user_id[:8]}...")

        if user_id in self.active_connections:
            disconnected = set()
            for ws in self.active_connections[user_id]:
                try:
                    await ws.send_text(message_str)
                except Exception:
                    disconnected.add(ws)

            for ws in disconnected:
                await self.remove_connection(user_id, ws)


# Global instance
sync_broadcaster = SyncEventBroadcaster()


def get_broadcaster() -> SyncEventBroadcaster:
    """Get the global broadcaster instance"""
    return sync_broadcaster
