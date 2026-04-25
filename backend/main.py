"""
FastAPI Application Main Entry Point
"""
import logging
import time
from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from starlette.middleware.base import BaseHTTPMiddleware
from contextlib import asynccontextmanager
from datetime import datetime
from database import init_db
from routes_auth import router as auth_router
from routes_files import router as files_router
from routes_agent import router as agent_router
from routes_sync import router as sync_router
from routes_websocket import router as websocket_router

def configure_app_logging() -> logging.Logger:
    formatter = logging.Formatter(
        "%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        "%Y-%m-%d %H:%M:%S",
    )
    handler = logging.StreamHandler()
    handler.setFormatter(formatter)

    loggers = [
        logging.getLogger("anywhere_door"),
        logging.getLogger("anywhere_door.requests"),
        logging.getLogger("anywhere_door.auth"),
        logging.getLogger("anywhere_door.files"),
        logging.getLogger("anywhere_door.sync"),
        logging.getLogger("anywhere_door.agent"),
    ]

    for logger in loggers:
        logger.setLevel(logging.INFO)
        if not logger.handlers:
            logger.addHandler(handler)
        logger.propagate = False

    return logging.getLogger("anywhere_door")

# Startup and shutdown events
@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    Lifespan context manager for FastAPI startup and shutdown
    """
    # Startup
    logger = configure_app_logging()
    logger.info("Starting AnywhereDoor Server...")
    try:
        init_db()
        logger.info("Database initialized successfully")
    except Exception as e:
        logger.error(f"Error initializing database: {str(e)}")
    
    yield
    
    # Shutdown
    logger.info("Shutting down AnywhereDoor Server...")


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


# ============================================================================
# Request Logging Middleware
# ============================================================================

class RequestLoggingMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next):
        logger = logging.getLogger("anywhere_door.requests")
        start_time = time.time()

        # Log incoming request
        body_size = request.headers.get("content-length", "0")
        request_path = request.url.path
        if request.url.query:
            request_path = f"{request_path}?{request.url.query}"
        logger.info(f"--> {request.method} {request_path} (body: {body_size} bytes)")

        try:
            response = await call_next(request)
        except Exception:
            duration_ms = (time.time() - start_time) * 1000
            logger.exception(
                f"<-- {request.method} {request_path} [500] ({duration_ms:.1f}ms)"
            )
            raise

        duration_ms = (time.time() - start_time) * 1000
        logger.info(
            f"<-- {request.method} {request_path} "
            f"[{response.status_code}] ({duration_ms:.1f}ms)"
        )
        return response

app.add_middleware(RequestLoggingMiddleware)


# Include routers
app.include_router(auth_router)
app.include_router(files_router)
app.include_router(agent_router)
app.include_router(sync_router)
app.include_router(websocket_router)


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
                "batch_metadata": "POST /api/files/batch-metadata",
                "check_hashes": "POST /api/files/check-hashes"
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
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "status": "error",
            "detail": exc.detail,
            "status_code": exc.status_code
        },
    )


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
