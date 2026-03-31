"""
FastAPI Application Main Entry Point
"""
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from contextlib import asynccontextmanager
from datetime import datetime
from database import init_db
from routes_auth import router as auth_router
from routes_files import router as files_router
from routes_agent import router as agent_router
from routes_sync import router as sync_router

# Startup and shutdown events
@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    Lifespan context manager for FastAPI startup and shutdown
    """
    # Startup
    print("Starting AnywhereDoor Server...")
    print(f"[{datetime.utcnow().isoformat()}] Initializing database...")
    try:
        init_db()
        print(f"[{datetime.utcnow().isoformat()}] Database initialized successfully")
    except Exception as e:
        print(f"[{datetime.utcnow().isoformat()}] Error initializing database: {str(e)}")
    
    yield
    
    # Shutdown
    print(f"\n[{datetime.utcnow().isoformat()}] Shutting down AnywhereDoor Server...")


# Create FastAPI app
app = FastAPI(
    title="AnywhereDoor Server",
    description="File sync and management server for AnywhereDoor agent",
    version="1.0.0",
    lifespan=lifespan
)


# Add CORS middleware to allow requests from agent
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # In production, specify exact origins
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# Include routers
app.include_router(auth_router)
app.include_router(files_router)
app.include_router(agent_router)
app.include_router(sync_router)


# Health check endpoint
@app.get("/health")
async def health_check():
    """
    Health check endpoint
    """
    return {
        "status": "healthy",
        "service": "AnywhereDoor Server",
        "timestamp": datetime.utcnow().isoformat()
    }


# Root endpoint
@app.get("/")
async def root():
    """
    Root endpoint with API information
    """
    return {
        "service": "AnywhereDoor Server",
        "version": "1.0.0",
        "description": "File sync and management server",
        "timestamp": datetime.utcnow().isoformat(),
        "endpoints": {
            "auth": {
                "login": "POST /auth/login",
                "register_device": "POST /auth/register-device",
                "create_user": "POST /auth/create-user"
            },
            "files": {
                "upload": "POST /api/files/upload",
                "list": "GET /api/files/list",
                "download": "GET /api/files/{file_id}/download",
                "delete": "DELETE /api/files/{file_id}",
                "batch_metadata": "POST /api/files/batch-metadata"
            },
            "agent": {
                "register": "POST /api/agent/register",
                "status": "GET /api/agent/{agent_id}/status",
                "list": "GET /api/agent",
                "update_status": "POST /api/agent/update-status"
            },
            "sync": {
                "directory": "POST /api/sync/directory",
                "file_sync": "POST /api/sync/file-sync",
                "status": "GET /api/sync/status/{agent_id}",
                "mark_failed": "POST /api/sync/mark-failed"
            }
        }
    }


# Error handlers
@app.exception_handler(HTTPException)
async def http_exception_handler(request, exc):
    """
    Custom HTTP exception handler
    """
    return {
        "status": "error",
        "detail": exc.detail,
        "status_code": exc.status_code
    }


if __name__ == "__main__":
    import uvicorn
    
    # Run server with hot reload in development
    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
        log_level="info"
    )
