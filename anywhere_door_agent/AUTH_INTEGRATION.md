# Auth Service Integration Guide

## What Was Implemented

A complete **JWT + Device Signature Authentication Service** with:

### ✅ Features
- **User Authentication**: JWT tokens (24-hour expiration)
- **Device Registration**: Generate device_id + device_secret per device
- **Request Signing**: HMAC-SHA256 signatures (timestamp + device_id + request_data)
- **Server Verification**: 4-step verification (JWT + device + signature)
- **Device Management**: List, revoke, and manage devices per user
- **Full Unit Tests**: 5 tests covering all scenarios
- **Working Example**: `cargo run --example auth_demo`

---

## Project Structure

```
src/
├── auth/
│   ├── auth.rs          ← Main auth service implementation (450+ lines)
│   └── mod.rs           ← Auth module exports
├── lib.rs               ← Library entry point (exposes auth module)
└── main.rs              ← Modified to include auth module
examples/
└── auth_demo.rs         ← Complete working example
AUTH_SERVICE.md          ← Comprehensive documentation
AUTH_INTEGRATION.md      ← This file
```

---

## Quick Start

### 1. Use in Your Code

```rust
use anywhere_door_agent::auth::{AuthService, AuthRequest};
use chrono::Utc;

// Initialize (once per server)
let mut auth = AuthService::new("your-secret-key".to_string());

// User login
let login = auth.user_login("user_001", "john_doe")?;
println!("JWT: {}", login.jwt);

// Register device
let device = auth.register_device("user_001", &login.jwt)?;
println!("Device ID: {}", device.device_id);
println!("Device Secret: {}", device.device_secret);

// Client signs request
let timestamp = Utc::now().timestamp();
let data = r#"{"action":"sync"}"#;
let sig = AuthService::generate_signature(
    &device.device_secret,
    &device.device_id,
    timestamp,
    data,
)?;

// Server verifies request
let result = auth.verify_request(&AuthRequest {
    jwt: login.jwt,
    device_id: device.device_id,
    timestamp,
    signature: sig,
    data: data.to_string(),
});

if result.valid {
    println!("✓ Request from user: {:?}", result.user_id);
}
```

### 2. Run the Demo

```bash
cd anywhere_door_agent
cargo run --example auth_demo
```

### 3. Run Tests

```bash
cargo test --lib auth
```

---

## API Overview

| Function | Purpose |
|----------|---------|
| `new(jwt_secret)` | Create auth service |
| `user_login(user_id, username)` | Generate JWT token |
| `verify_jwt(token)` | Verify JWT token |
| `register_device(user_id, jwt)` | Register new device |
| `generate_signature(secret, device_id, timestamp, data)` | Create HMAC signature |
| `verify_request(auth_request)` | Verify complete request |
| `list_user_devices(user_id)` | Get all user devices |
| `unregister_device(device_id)` | Revoke device access |

---

## Authentication Flow

```
┌─────────────────────────────────────────┐
│ 1. User Login                           │
│    → JWT Token (valid 24 hours)         │
└─────────────────────────────────────────┘
              ↓
┌─────────────────────────────────────────┐
│ 2. Device Registration                  │
│    → device_id + device_secret          │
└─────────────────────────────────────────┘
              ↓
┌─────────────────────────────────────────┐
│ 3. Sign Request (on client)             │
│    → HMAC-SHA256(secret, msg)           │
└─────────────────────────────────────────┘
              ↓
┌─────────────────────────────────────────┐
│ 4. Verify Request (on server)           │
│    ✓ Check JWT                          │
│    ✓ Check device registered & owned    │
│    ✓ Check signature matches            │
│    → Request accepted if all pass       │
└─────────────────────────────────────────┘
```

---

## Dependencies Added

```toml
[dependencies]
jsonwebtoken = "9"           # JWT handling
chrono = { version = "0.4", features = ["serde"] }  # Timestamps
rand = "0.8"                 # Random secret generation
hex = "0.4"                  # Hex encoding for signatures
hmac = "0.12"                # HMAC-SHA256 signing
sha2 = "0.10"                # SHA256 hashing
uuid = { version = "1", features = ["v4", "serde"] }  # Device IDs
```

---

## Example Request Format

### Client sends:
```json
{
  "jwt": "eyJhbGciOiJIUzI1NiJ9...",
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": 1704067200,
  "signature": "a7f3e2d1c4b5a8f7e6d5c4b3a2f1e0d9...",
  "data": "{\"action\":\"sync\",\"files\":[\"file1.txt\"]}"
}
```

### Server response (if valid):
```json
{
  "valid": true,
  "user_id": "user_001",
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "error": null
}
```

---

## Testing Output

```
running 5 tests
test auth::auth::tests::test_device_registration ... ok
test auth::auth::tests::test_device_not_found ... ok
test auth::auth::tests::test_invalid_signature ... ok
test auth::auth::tests::test_user_login_and_verify_jwt ... ok
test auth::auth::tests::test_request_signature_and_verification ... ok

test result: ok. 5 passed; 0 failed
```

---

## Security Best Practices

### ✓ Implement These:
1. **Secure JWT Secret**: Use 32+ character random string
2. **HTTPS Only**: Always use TLS for requests
3. **Rate Limiting**: Prevent brute force login attempts
4. **Replay Prevention**: Check timestamp freshness (e.g., max 5 minutes old)
5. **Audit Logging**: Log all auth attempts
6. **Device Revocation**: Allow users to revoke devices
7. **Secure Storage**: Encrypt device_secret on client side

### ⚠️ Not Implemented Yet:
- Rate limiting (add yourself)
- Replay attack prevention via nonce (add yourself)
- Audit logging (add yourself)
- Secure device secret storage (use OS keychain)

---

## Next Steps

### Option 1: Use in Service Main
Integrate auth into `src/service.rs` to authenticate sync requests:

```rust
fn handle_sync_request(auth_request: &AuthRequest) -> Result<()> {
    let auth = AuthService::new(SECRET_KEY.to_string());
    let result = auth.verify_request(auth_request);
    
    if !result.valid {
        return Err(format!("Auth failed: {:?}", result.error));
    }
    
    // Process sync for result.user_id
    sync_files(result.user_id.unwrap())?;
    Ok(())
}
```

### Option 2: Build REST API Endpoints
Create HTTP endpoints:
- `POST /auth/login` → LoginResponse
- `POST /auth/register-device` → RegisterDeviceResponse
- `POST /api/sync` (with AuthRequest) → SyncResponse

### Option 3: Extend Auth Service
Add to AUTH_SERVICE.md future enhancements:
- Refresh tokens
- Role-based access control
- Device fingerprinting
- OAuth2 integration

---

## Documentation Files

1. **AUTH_SERVICE.md** (350+ lines)
   - Complete protocol documentation
   - Data structure definitions
   - Implementation guide
   - Security considerations
   - API reference

2. **Examples/auth_demo.rs**
   - Full working example
   - Shows all 4 steps of auth flow
   - Tests device operations
   - Tests invalid scenarios

3. **This File (AUTH_INTEGRATION.md)**
   - Quick reference
   - Integration instructions
   - Code examples
   - Next steps

---

## Commands Reference

```bash
# Build library
cargo build --lib

# Run authentication demo
cargo run --example auth_demo

# Run tests
cargo test --lib auth

# Build everything
cargo build --release
```

---

## Questions?

Refer to:
- `AUTH_SERVICE.md` - Complete technical documentation
- `examples/auth_demo.rs` - Working example code
- `src/auth/auth.rs` - Source code with inline comments (450+ lines)

---

**Status**: ✅ Complete & Tested
- All 5 unit tests passing
- Example demo working
- Ready for integration
