# Anywhere Door Agent - Authentication Service

## Overview

The authentication service implements a **two-layer security model**:

1. **User Authentication Layer**: JWT (JSON Web Tokens) for user identity
2. **Device Authentication Layer**: HMAC-SHA256 signatures for device identity & request integrity

## Authentication Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. USER LOGIN                                                   │
│    POST /auth/login {username, password}                        │
│    ↓                                                             │
│    Returns: JWT Token (24 hour expiration)                      │
└─────────────────────────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. DEVICE REGISTRATION                                          │
│    POST /auth/register-device {jwt}                             │
│    ↓                                                             │
│    Returns: device_id + device_secret                           │
│    (Store device_secret securely - it's used for signing)       │
└─────────────────────────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. CLIENT (AGENT) SIGNS REQUEST                                 │
│    For each request:                                            │
│    - Get timestamp (Unix epoch)                                 │
│    - Create message: "timestamp:device_id:request_data"         │
│    - Generate HMAC-SHA256(device_secret, message)               │
│    - Signature = hex-encoded HMAC                               │
└─────────────────────────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. SERVER RECEIVES & VERIFIES                                   │
│    Request format:                                              │
│    {                                                            │
│      "jwt": "<user_jwt_token>",                                 │
│      "device_id": "<device_id>",                                │
│      "timestamp": 1704067200,                                   │
│      "signature": "<hmac_signature>",                           │
│      "data": "<request_payload>"                                │
│    }                                                            │
│                                                                 │
│    Verification steps:                                          │
│    ✓ Verify JWT token (user identity)                           │
│    ✓ Check device is registered & belongs to user               │
│    ✓ Regenerate signature and compare                           │
│    ✓ Request accepted if all checks pass                        │
└─────────────────────────────────────────────────────────────────┘
```

## Data Structures

### UserClaims (JWT Payload)
```rust
{
    "sub": "user_001",              // User ID (subject)
    "username": "john_doe",         // Username
    "iat": 1704067200,              // Issued at (Unix timestamp)
    "exp": 1704153600               // Expiration (Unix timestamp)
}
```

### DeviceInfo (Database/Storage)
```rust
{
    "device_id": "550e8400-e29b-41d4-a716-446655440000",
    "device_secret": "a1b2c3d4...",  // 64-char hex string
    "user_id": "user_001",
    "registered_at": 1704067200
}
```

### AuthRequest (Client to Server)
```json
{
    "jwt": "eyJhbGciOiJIUzI1NiIs...",
    "device_id": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": 1704067200,
    "signature": "a7f3e2d1c4b5a8f7...",
    "data": "{\"action\":\"sync\",\"files\":[\"file1.txt\"]}"
}
```

## Implementation Guide

### 1. Initialize Auth Service

```rust
use anywhere_door_agent::auth::AuthService;

// Server-side initialization (in main.rs or config)
let mut auth_service = AuthService::new("your-secret-jwt-key".to_string());
```

**Important**: The JWT secret should be:
- Long (32+ characters)
- Random
- Kept secure (environment variable or secure config)
- Same for all instances of your server

### 2. User Login Endpoint

```rust
// POST /auth/login
fn login(username: &str, password: &str) -> LoginResponse {
    // 1. Verify username/password against database
    let user_id = verify_credentials(username, password)?;
    
    // 2. Issue JWT
    let response = auth_service.user_login(&user_id, username)?;
    
    // Return JWT to client
    response
}
```

**Client receives**:
```json
{
    "jwt": "eyJhbGc...",
    "user_id": "user_001",
    "expires_in": 86400
}
```

### 3. Device Registration Endpoint

```rust
// POST /auth/register-device
fn register_device(jwt: String) -> RegisterDeviceResponse {
    // 1. Verify JWT token
    let claims = auth_service.verify_jwt(&jwt)?;
    let user_id = claims.sub;
    
    // 2. Register device
    let response = auth_service.register_device(&user_id, &jwt)?;
    
    // Return device credentials to client
    response
}
```

**Client receives**:
```json
{
    "device_id": "550e8400-e29b-41d4-a716-446655440000",
    "device_secret": "a1b2c3d4e5f6g7h8...",
    "created_at": 1704067200
}
```

⚠️ **Client MUST store device_secret securely** (encrypted, protected storage)

### 4. Client-Side: Generate Signatures

```rust
use anywhere_door_agent::auth::AuthService;
use chrono::Utc;

// On agent (client) side
let jwt = "..."; // from step 1
let device_id = "..."; // from step 2
let device_secret = "..."; // from step 2

// For each request
let timestamp = Utc::now().timestamp();
let request_data = r#"{"action":"sync"}"#;

let signature = AuthService::generate_signature(
    &device_secret,
    &device_id,
    timestamp,
    request_data,
)?;

// Build request
let auth_request = AuthRequest {
    jwt: jwt.clone(),
    device_id,
    timestamp,
    signature,
    data: request_data.to_string(),
};

// Send to server
send_request(&auth_request).await?;
```

### 5. Server-Side: Verify Requests

```rust
// For any incoming request
fn verify_and_handle(auth_request: &AuthRequest) -> Result<Response> {
    // 1. Verify authentication
    let result = auth_service.verify_request(auth_request);
    
    if !result.valid {
        return Err(format!("Auth failed: {:?}", result.error));
    }
    
    // 2. Request is valid - process it
    let user_id = result.user_id.unwrap();
    let device_id = result.device_id.unwrap();
    
    handle_request(user_id, device_id, &auth_request.data)?
}
```

## Security Considerations

### ✓ What This Protects Against

1. **Unauthorized Users**: JWT ensures request comes from authenticated user
2. **Device Spoofing**: Signature proves request comes from registered device
3. **Request Tampering**: HMAC-SHA256 ensures request data hasn't been modified
4. **Replay Attacks** (Partial): Timestamp + signature binding helps detect replays
5. **Device Theft**: Stolen device_secret can be revoked via unregister_device()

### ⚠️ What You Should ALSO Implement

1. **Rate Limiting**: Prevent brute force attempts on login
2. **HTTPS/TLS**: Encrypt all communication over network
3. **Replay Attack Prevention**: 
   - Check timestamps aren't too old (e.g., > 5 minutes)
   - Use nonce system or sequence numbers
4. **Device Rotation**: Encourage users to rotate device secrets periodically
5. **Audit Logging**: Log all auth attempts and rejections
6. **Secure Storage**:
   - Store JWT secret in environment variables
   - Hash device secrets (optional) in database
   - Encrypt device_secret on client

## Testing

Run the test suite:

```bash
cargo test --lib auth
```

Run the example demo:

```bash
cargo run --example auth_demo
```

Expected output:
```
=== Anywhere Door Authentication Service Demo ===

✓ Auth service initialized

--- STEP 1: User Login ---
User logged in successfully!
  User ID: user_001
  JWT Token: eyJhbGc...
  Expires in: 86400 seconds

--- STEP 2: Device Registration ---
Device registered successfully!
  Device ID: 550e8400-e29b-41d4-a716-446655440000
  Device Secret: a1b2c3d4e5f6g7h8...
  Created at: 1704067200

--- STEP 3: Agent Signs Request ---
Request signed successfully!
  Timestamp: 1704067200
  Data: {"action":"sync", "directory":"C:\\Users\\john\\Documents"}
  Signature: a7f3e2d1c4b5a8f7...

--- STEP 4: Server Verifies Request ---
Request verification result:
  Valid: true
  User ID: Some("user_001")
  Device ID: Some("550e8400-e29b-41d4-a716-446655440000")
  Error: None

✓ REQUEST ACCEPTED - Authentication successful!
```

## API Reference

### AuthService Methods

| Method | Purpose | Returns |
|--------|---------|---------|
| `new(secret)` | Initialize auth service | AuthService |
| `user_login(user_id, username)` | Issue JWT token | LoginResponse |
| `verify_jwt(token)` | Verify & decode JWT | UserClaims |
| `register_device(user_id, jwt)` | Register new device | RegisterDeviceResponse |
| `generate_signature(secret, device_id, timestamp, data)` | Create HMAC signature | String |
| `verify_request(auth_request)` | Verify complete request | VerificationResult |
| `list_user_devices(user_id)` | Get all user devices | Vec<DeviceInfo> |
| `unregister_device(device_id)` | Revoke device access | Result |

### Error Handling

All methods return `Result<T, String>`:

```rust
match auth_service.user_login(user_id, username) {
    Ok(response) => println!("Login successful: {}", response.jwt),
    Err(e) => eprintln!("Login failed: {}", e),
}

let result = auth_service.verify_request(&auth_request);
if !result.valid {
    eprintln!("Request rejected: {:?}", result.error);
}
```

## Dependencies

Add to `Cargo.toml`:

```toml
jsonwebtoken = "9"
chrono = { version = "0.4", features = ["serde"] }
rand = "0.8"
hex = "0.4"
hmac = "0.12"
sha2 = "0.10"
uuid = { version = "1", features = ["v4", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Future Enhancements

1. **OAuth2 Integration**: Support external identity providers
2. **Refresh Tokens**: Extend JWT validity without re-login
3. **Role-Based Access Control (RBAC)**: Different permissions per device/user
4. **Device Fingerprinting**: Bind device to hardware (prevent theft)
5. **Biometric Authentication**: Face/fingerprint for device unlock
6. **Hardware Keys**: FIDO2/WebAuthn support
7. **Nonce-Based Replay Prevention**: Cryptographic nonce system
