# AnywhereDoor Server - Quick Start Guide

## Overview

This is a complete FastAPI backend server for the AnywhereDoor file synchronization system. It's designed to be simple, easy to read, and straightforward to run.

**Key Features:**
- JWT-based user authentication
- Device registration with cryptographic signing
- File upload with integrity verification (SHA256)
- Automatic sync tracking and status monitoring
- SQLite database (or PostgreSQL for production)
- Complete API documentation via Swagger UI

## Prerequisites

- Python 3.8 or higher
- pip (Python package manager)
- ~2GB disk space for file storage
- Port 8000 (or any available port)

## Installation

### Step 1: Install Dependencies

```bash
cd /home/deilsy/Any_where_door/backend

# On Linux/Mac:
pip3 install -r requirements.txt

# On Windows (in PowerShell or Command Prompt):
pip install -r requirements.txt
```

### Step 2: Configure Environment

Copy the example configuration:

```bash
cp .env.example .env
```

For **development**, the default settings are fine. For **production**, edit `.env` and change:
- `JWT_SECRET`: Use a strong random value
- `DATABASE_URL`: Use PostgreSQL instead of SQLite
- `ALLOWED_ORIGINS`: Specify exact client domains

### Step 3: Start the Server

**On Linux/Mac:**
```bash
./run_server.sh
```

**On Windows:**
```cmd
run_server.bat
```

**Or manually:**
```bash
# Development mode (with auto-reload):
uvicorn main:app --reload

# Production mode:
uvicorn main:app --host 0.0.0.0 --port 8000
```

The server will:
1. Create the SQLite database
2. Create the storage directory
3. Start listening on http://localhost:8000

## Verify Installation

Open your browser:
- **Health Check:** http://localhost:8000/health
- **API Documentation:** http://localhost:8000/docs
- **Root Endpoint:** http://localhost:8000/

You should see JSON responses confirming the server is running.

## First Steps

### 1. Create a User

```bash
curl -X POST "http://localhost:8000/auth/create-user?username=testuser&password=testpass123"
```

Response:
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "testuser",
  "status": "created"
}
```

### 2. Login

```bash
curl -X POST "http://localhost:8000/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "testpass123"}'
```

Response:
```json
{
  "jwt": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "expires_in": 86400
}
```

Save the `jwt` value - you'll need it for all other requests.

### 3. Register a Device

```bash
export JWT="eyJ0eXAiOiJKV1QiLCJhbGc..."  # Use your JWT from above

curl -X POST "http://localhost:8000/auth/register-device" \
  -H "Content-Type: application/json" \
  -d "{\"jwt\": \"$JWT\"}"
```

Response:
```json
{
  "device_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "device_secret": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "created_at": 1712000000
}
```

These credentials (`device_id` and `device_secret`) are used by the agent to sign requests.

## Testing with Example Client

A complete example client is provided:

```bash
cd /home/deilsy/Any_where_door/backend
python3 example_client.py
```

This will demonstrate:
- Creating a user
- Logging in
- Registering a device
- Registering an agent
- Uploading a test file
- Listing files
- Checking agent status

## API Documentation

The server provides interactive API documentation at:

**http://localhost:8000/docs**

This Swagger UI allows you to:
- View all available endpoints
- See request/response schemas
- Test endpoints directly in the browser
- Copy curl commands

## Directory Structure

After running the server, you'll see:

```
backend/
├── any_where_door.db           # SQLite database (auto-created)
├── storage/
│   └── files/                  # Uploaded files stored here
│       └── user_id/            # Organized by user
│           └── file.ext
├── venv/                       # Virtual environment (auto-created)
├── main.py                     # FastAPI app
├── routes_*.py                 # API endpoint modules
├── auth_utils.py               # Authentication functions
├── schemas.py                  # Pydantic validation models
├── models.py                   # Database models
├── database.py                 # Database setup
├── config.py                   # Configuration
├── requirements.txt            # Python dependencies
└── README.md                   # Full documentation
```

## Common Tasks

### Reset Database

```bash
rm any_where_door.db
# Restart server - database will be recreated
```

### Change Port

```bash
# Linux/Mac:
./run_server.sh --port 8080

# Windows:
run_server.bat --port 8080

# Manual:
uvicorn main:app --port 8080
```

### Change Database Location

Edit `.env`:
```
DATABASE_URL=sqlite:////path/to/database.db
```

### Use PostgreSQL (Production)

Edit `.env`:
```
DATABASE_URL=postgresql://user:password@localhost:5432/anywhere_door
```

Then reinstall dependencies (psycopg2 is needed):
```bash
pip install -r requirements.txt psycopg2-binary
```

### See Request Logs

```bash
# Development mode already shows logs
uvicorn main:app --log-level debug
```

## Connecting Your Agent

To connect the Rust agent to this server:

1. Update agent's server URL in the agent code
2. Run the agent with server connection enabled
3. Agent performs login and device registration
4. Agent starts uploading files

See the agent documentation for integration steps.

## Troubleshooting

### Port Already in Use

If port 8000 is already in use:
```bash
# Find what's using port 8000
lsof -i :8000  # Linux/Mac
netstat -ano | findstr :8000  # Windows

# Use a different port
./run_server.sh --port 8888
```

### Database Lock Error

This usually means multiple processes are accessing the SQLite database. Solutions:
1. Only run one server instance
2. Use PostgreSQL for production (supports concurrent connections)

### Import Errors

Ensure all dependencies are installed:
```bash
pip install -r requirements.txt
```

If you get "No module named 'fastapi'", the virtual environment may not be activated:
```bash
# Linux/Mac:
source venv/bin/activate

# Windows:
venv\Scripts\activate.bat
```

### File Upload Fails

Check:
1. Storage directory has write permissions: `ls -la storage/`
2. Disk has available space: `df -h`
3. File size is under MAX_FILE_SIZE_MB (default: 500MB)
4. JWT token is valid and not expired

### Server Won't Start

Common causes:
1. Python not installed: `python3 --version`
2. Dependencies not installed: `pip install -r requirements.txt`
3. Port in use: Use `--port` option
4. Corrupt database: Delete `any_where_door.db` and restart

## Security Checklist

For **production deployment**:

- [ ] Change `JWT_SECRET` to a strong random value (32+ characters)
- [ ] Use PostgreSQL instead of SQLite
- [ ] Enable HTTPS/TLS on the server
- [ ] Set `ALLOWED_ORIGINS` to specific domains (not `*`)
- [ ] Run with appropriate user permissions (not root)
- [ ] Enable database backups
- [ ] Monitor disk space and file sizes
- [ ] Implement rate limiting
- [ ] Add request logging and monitoring
- [ ] Validate file types strictly
- [ ] Run behind reverse proxy (nginx, Apache)
- [ ] Keep Python and dependencies updated

## Performance Notes

The server is designed for:
- **Small deployments**: < 10 concurrent users
- **Medium deployments**: < 100 concurrent users with PostgreSQL
- **Large deployments**: Requires additional infrastructure:
  - Database replication
  - Load balancing
  - Object storage (S3)
  - CDN for downloads

## Stopping the Server

Press `Ctrl+C` in the terminal running the server.

## Next Steps

1. **Connect the Agent**: Configure the Rust agent to use this server
2. **Configure Users**: Create user accounts for your team
3. **Monitor Syncs**: Use the API to check sync status
4. **Setup Backups**: Regular database and file backups
5. **Scale Up**: As needed, migrate to PostgreSQL and multi-instance setup

## Getting Help

- Check the full [README.md](README.md) for detailed API documentation
- Look at [example_client.py](example_client.py) for usage examples
- Review API docs at http://localhost:8000/docs
- Check `config.py` for all available settings

---

**Enjoy AnywhereDoor! 🚀**
