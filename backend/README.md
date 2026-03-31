# AnywhereDoor Server - FastAPI Backend

A simple, easy-to-read FastAPI server that receives, authenticates, and manages file uploads from the AnywhereDoor agent.

## Architecture

The server uses a functional programming approach (no classes) for simplicity and readability:

- **Authentication**: JWT tokens + device secret signatures
- **File Storage**: Local filesystem with metadata in SQLite database
- **Database**: SQLAlchemy ORM with 6 models (User, Device, File, FileMetadata, FileSync, Agent)
- **API**: FastAPI with async endpoints
- **Validation**: Pydantic schemas

## Project Structure

```
backend/
├── main.py                  # FastAPI application entrypoint
├── routes_auth.py           # Authentication endpoints
├── routes_files.py          # File upload/management endpoints
├── routes_agent.py          # Agent registration/status endpoints
├── routes_sync.py           # File sync tracking endpoints
├── auth_utils.py            # JWT and signature verification functions
├── schemas.py               # Pydantic request/response models
├── models.py                # SQLAlchemy database models
├── database.py              # Database setup and dependencies
├── config.py                # Configuration management
├── requirements.txt         # Python dependencies
├── example_client.py        # Example client showing API usage
└── README.md               # This file
```

## Setup & Installation

### 1. Install Python Dependencies

```bash
pip install -r requirements.txt
```

### 2. Run the Server

```bash
python main.py
```

Or with uvicorn directly:

```bash
uvicorn main:app --reload --host 0.0.0.0 --port 8000
```

### 3. Verify Server is Running

```bash
curl http://localhost:8000/health
```

Expected response:
```json
{
  "status": "healthy",
  "service": "AnywhereDoor Server",
  "timestamp": "2026-03-31T12:34:56.789123"
}
```

## API Endpoints

### Authentication (`/auth`)

#### POST `/auth/create-user`
Create a new user account (for testing/initial setup)

```bash
curl -X POST "http://localhost:8000/auth/create-user?username=john&password=secret123"
```

Response:
```json
{
  "user_id": "uuid-here",
  "username": "john",
  "status": "created"
}
```

#### POST `/auth/login`
Login and receive JWT token

```bash
curl -X POST "http://localhost:8000/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username": "john", "password": "secret123"}'
```

Response:
```json
{
  "jwt": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "user_id": "uuid-here",
  "expires_in": 86400
}
```

#### POST `/auth/register-device`
Register a device and get signing credentials

```bash
curl -X POST "http://localhost:8000/auth/register-device" \
  -H "Content-Type: application/json" \
  -d '{"jwt": "eyJ0eXAiOiJKV1QiLCJhbGc..."}'
```

Response:
```json
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "device_secret": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "created_at": 1712000000
}
```

### Files (`/api/files`)

#### POST `/api/files/upload`
Upload a file with metadata (Base64 encoded content)

```bash
curl -X POST "http://localhost:8000/api/files/upload?jwt=YOUR_JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "metadata": {
      "file_path": "/home/user/docs/report.pdf",
      "file_name": "report.pdf",
      "file_size": 102400,
      "modified_at": 1712000000,
      "created_at": 1712000000,
      "file_hash": "sha256_hex_here",
      "mime_type": "application/pdf",
      "is_directory": false
    },
    "file_content": "base64_encoded_content_here"
  }'
```

Response:
```json
{
  "file_id": "uuid-here",
  "stored_at": "/path/to/storage/file.pdf",
  "timestamp": 1712000000,
  "size_bytes": 102400,
  "hash_verified": true
}
```

#### GET `/api/files/list`
List files for authenticated user

```bash
curl "http://localhost:8000/api/files/list?jwt=YOUR_JWT&limit=10&skip=0"
```

Response:
```json
{
  "status": "success",
  "total": 3,
  "files": [
    {
      "file_id": "uuid-1",
      "file_name": "report.pdf",
      "file_path": "/home/user/docs/report.pdf",
      "file_size": 102400,
      "file_hash": "sha256_hex",
      "mime_type": "application/pdf",
      "uploaded_at": 1712000000
    }
  ]
}
```

#### GET `/api/files/{file_id}/download`
Download a file (returns Base64 encoded content)

```bash
curl "http://localhost:8000/api/files/abc123/download?jwt=YOUR_JWT"
```

#### DELETE `/api/files/{file_id}`
Delete a file

```bash
curl -X DELETE "http://localhost:8000/api/files/abc123?jwt=YOUR_JWT"
```

### Agent (`/api/agent`)

#### POST `/api/agent/register`
Register an agent

```bash
curl -X POST "http://localhost:8000/api/agent/register?jwt=YOUR_JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "agent-001",
    "agent_version": "1.0.0",
    "os": "Linux",
    "hostname": "my-laptop",
    "sync_root": "/home/user/Documents",
    "last_sync": 1712000000,
    "status": "active"
  }'
```

#### GET `/api/agent`
List all agents for user

```bash
curl "http://localhost:8000/api/agent?jwt=YOUR_JWT"
```

#### GET `/api/agent/{agent_id}/status`
Get agent sync status

```bash
curl "http://localhost:8000/api/agent/agent-001/status?jwt=YOUR_JWT"
```

#### POST `/api/agent/update-status`
Update agent status

```bash
curl -X POST "http://localhost:8000/api/agent/update-status?agent_id=agent-001&status=inactive&jwt=YOUR_JWT"
```

### Sync (`/api/sync`)

#### POST `/api/sync/directory`
Sync entire directory metadata

```bash
curl -X POST "http://localhost:8000/api/sync/directory?agent_id=agent-001&jwt=YOUR_JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "directory_path": "/home/user/Documents",
    "directory_name": "Documents",
    "total_files": 5,
    "total_size": 512000,
    "scanned_at": 1712000000,
    "files": [
      {
        "file_path": "/home/user/Documents/file1.txt",
        "file_name": "file1.txt",
        "file_size": 1024,
        "modified_at": 1712000000,
        "created_at": 1712000000,
        "file_hash": "sha256_hex",
        "mime_type": "text/plain",
        "is_directory": false
      }
    ]
  }'
```

#### POST `/api/sync/file-sync`
Mark a file as successfully synced

```bash
curl -X POST "http://localhost:8000/api/sync/file-sync?file_id=xyz&agent_id=agent-001&jwt=YOUR_JWT"
```

#### GET `/api/sync/status/{agent_id}`
Get sync statistics for an agent

```bash
curl "http://localhost:8000/api/sync/status/agent-001?jwt=YOUR_JWT"
```

Response:
```json
{
  "status": "success",
  "agent_id": "agent-001",
  "total_syncs": 10,
  "successful_syncs": 9,
  "failed_syncs": 1,
  "pending_syncs": 0,
  "total_size_bytes": 512000,
  "last_sync": 1712000000
}
```

#### POST `/api/sync/mark-failed`
Mark a file sync as failed

```bash
curl -X POST "http://localhost:8000/api/sync/mark-failed?file_id=xyz&agent_id=agent-001&error_message=timeout&jwt=YOUR_JWT"
```

## Database Models

### User
```
- user_id (UUID, primary key)
- username (String, unique)
- password_hash (String)
- created_at (DateTime)
```

### Device
```
- device_id (UUID, primary key)
- device_secret (String) - used for HMAC-SHA256 signing
- user_id (FK to User)
- status (String) - active, inactive, revoked
- created_at (DateTime)
- last_seen (DateTime)
```

### File
```
- file_id (UUID, primary key)
- user_id (FK to User)
- file_path (String) - original path from agent
- file_name (String)
- file_size (Integer) - bytes
- file_hash (String) - SHA256
- mime_type (String)
- stored_at (String) - filesystem path
- uploaded_at (DateTime)
```

### FileMetadata
```
- metadata_id (UUID, primary key)
- file_id (FK to File)
- original_path (String)
- tags (String) - JSON
- description (String)
- version (Integer)
```

### Agent
```
- agent_id (UUID, primary key)
- user_id (FK to User)
- device_id (FK to Device, nullable)
- agent_version (String)
- os (String)
- hostname (String)
- sync_root (String)
- status (String)
- last_sync (DateTime)
- created_at (DateTime)
- updated_at (DateTime)
```

### FileSync
```
- sync_id (UUID, primary key)
- agent_id (FK to Agent)
- file_id (FK to File)
- file_size (Integer)
- status (String) - pending, success, failed
- synced_at (DateTime)
```

## Example Usage

See `example_client.py` for a complete example:

```bash
python example_client.py
```

This demonstrates:
1. Creating a user
2. Logging in
3. Registering a device
4. Registering an agent
5. Uploading a file
6. Listing files
7. Checking agent status

## Authentication Flow

1. **User Login**: Username + password → JWT token
2. **Device Registration**: JWT → device_id + device_secret
3. **Request Signing**: All API requests include:
   - `jwt`: User's JWT token
   - `timestamp`: Current timestamp (prevents replay attacks)
   - `signature`: HMAC-SHA256(device_secret, device_id:timestamp:data)
4. **Server Verification**: Validates JWT and signature on every request

## File Upload Flow

1. Agent reads file from disk
2. Calculate SHA256 hash
3. Base64 encode file content
4. POST to `/api/files/upload` with metadata
5. Server verifies hash matches
6. Server stores file to disk
7. Server creates database records
8. Agent receives file_id and confirmation

## Configuration

Edit `config.py` to customize:

```python
# Database URL
DATABASE_URL = "sqlite:///./any_where_door.db"  # SQLite
# Or: DATABASE_URL = "postgresql://user:pass@localhost/db"

# JWT settings
JWT_SECRET = "your-secret-key-change-in-production"
JWT_ALGORITHM = "HS256"

# File storage
FILE_STORAGE_DIR = "./storage/files"
MAX_FILE_SIZE_MB = 500

# API
API_TITLE = "AnywhereDoor Server"
API_VERSION = "1.0.0"
```

## Security Notes

⚠️ **For Production**:
1. Change `JWT_SECRET` to a strong random value
2. Use environment variables for secrets (see `config.py` for `.env` support)
3. Enable HTTPS/TLS for all connections
4. Use PostgreSQL instead of SQLite
5. Implement rate limiting
6. Add logging and monitoring
7. Validate file types more strictly
8. Implement proper user management/permissions

## Troubleshooting

### Server won't start
- Check if port 8000 is already in use
- Verify Python 3.8+ is installed
- Check that all dependencies are installed

### Database errors
- Delete `any_where_door.db` to reset database
- Check database file permissions

### File upload fails
- Verify device is registered
- Check that JWT token is valid
- Ensure file hash matches
- Check disk space

### GET request parameter issues
- Parameters should be in query string, not request body
- Format: `?jwt=TOKEN&param=value`

## Performance

The server is designed for small to medium deployments (< 1000 users). For larger scale:
- Use PostgreSQL instead of SQLite
- Implement caching (Redis)
- Use object storage (S3) for files
- Add load balancing
- Implement database replication

## Development

### Run with hot reload:
```bash
uvicorn main:app --reload
```

### Run tests:
```bash
pytest
```

### Format code:
```bash
black *.py
```

### Lint code:
```bash
pylint *.py
```

## License

Same as AnywhereDoor project
