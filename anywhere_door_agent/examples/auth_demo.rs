// ⚠️  AUTHENTICATION SERVICE EXAMPLE & USAGE
// 
// This demonstrates the complete JWT + Device Signature authentication flow:
//
//   User Login → JWT Token
//        ↓
//   Device Register → device_id + device_secret
//        ↓
//   Agent Signs Request → HMAC-SHA256(secret, timestamp:device_id:data)
//        ↓
//   Server Verifies → JWT + Device + Signature
//        ↓
//   Request Accepted ✓

use anywhere_door_agent::auth::{AuthService, AuthRequest};
use chrono::Utc;

fn main() {
    println!("=== Anywhere Door Authentication Service Demo ===\n");

    // ========================================================================
    // STEP 1: INITIALIZE AUTH SERVICE
    // ========================================================================
    let mut auth_service = AuthService::new("my-super-secret-jwt-key".to_string());
    println!("✓ Auth service initialized\n");

    // ========================================================================
    // STEP 2: USER LOGIN (Get JWT Token)
    // ========================================================================
    println!("--- STEP 1: User Login ---");
    let login_response = auth_service
        .user_login("user_001", "john_doe")
        .expect("Failed to login");

    println!("User logged in successfully!");
    println!("  User ID: {}", login_response.user_id);
    println!("  JWT Token: {}...", &login_response.jwt[..50]);
    println!("  Expires in: {} seconds\n", login_response.expires_in);

    let user_jwt = login_response.jwt;

    // ========================================================================
    // STEP 3: DEVICE REGISTRATION (Get device_id + device_secret)
    // ========================================================================
    println!("--- STEP 2: Device Registration ---");
    let device_response = auth_service
        .register_device("user_001", &user_jwt)
        .expect("Failed to register device");

    println!("Device registered successfully!");
    println!("  Device ID: {}", device_response.device_id);
    println!("  Device Secret: {}...", &device_response.device_secret[..32]);
    println!("  Created at: {}\n", device_response.created_at);

    let device_id = device_response.device_id;
    let device_secret = device_response.device_secret;

    // ========================================================================
    // STEP 4: CREATE AND SIGN A REQUEST (Agent side)
    // ========================================================================
    println!("--- STEP 3: Agent Signs Request ---");
    
    let timestamp = Utc::now().timestamp();
    let request_data = r#"{"action":"sync", "directory":"C:\\Users\\john\\Documents"}"#;

    let signature = AuthService::generate_signature(
        &device_secret,
        &device_id,
        timestamp,
        request_data,
    )
    .expect("Failed to generate signature");

    println!("Request signed successfully!");
    println!("  Timestamp: {}", timestamp);
    println!("  Data: {}", request_data);
    println!("  Signature: {}...\n", &signature[..32]);

    // ========================================================================
    // STEP 5: SEND AUTHENTICATED REQUEST (Server receives this)
    // ========================================================================
    println!("--- STEP 4: Server Verifies Request ---");
    
    let auth_request = AuthRequest {
        jwt: user_jwt.clone(),
        device_id: device_id.clone(),
        timestamp,
        signature: signature.clone(),
        data: request_data.to_string(),
    };

    let verification = auth_service.verify_request(&auth_request);

    println!("Request verification result:");
    println!("  Valid: {}", verification.valid);
    println!("  User ID: {:?}", verification.user_id);
    println!("  Device ID: {:?}", verification.device_id);
    println!("  Error: {:?}\n", verification.error);

    if verification.valid {
        println!("✓ REQUEST ACCEPTED - Authentication successful!\n");
    } else {
        println!("✗ REQUEST REJECTED - Authentication failed!\n");
    }

    // ========================================================================
    // ADDITIONAL OPERATIONS
    // ========================================================================
    println!("--- Additional Operations ---");

    // List devices for user
    let devices = auth_service.list_user_devices("user_001");
    println!("Devices for user_001: {} device(s)", devices.len());
    for device in devices {
        println!("  - {} (registered: {})", device.device_id, device.registered_at);
    }

    println!("\n--- Testing Invalid Signature ---");
    
    // Test with invalid signature
    let invalid_request = AuthRequest {
        jwt: user_jwt,
        device_id,
        timestamp,
        signature: "invalid_signature_12345".to_string(),
        data: request_data.to_string(),
    };

    let invalid_result = auth_service.verify_request(&invalid_request);
    println!("  Valid: {}", invalid_result.valid);
    println!("  Error: {:?}\n", invalid_result.error);

    println!("=== Demo Complete ===");
}
