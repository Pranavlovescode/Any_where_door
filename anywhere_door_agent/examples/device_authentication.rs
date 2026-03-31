// Example: Device Registration and Authentication
// This file demonstrates how to use the Anywhere Door device authentication system

use std::path::Path;
use anywhere_door_agent::{
    net::net::NetworkService,
    auth::AuthService,
};

// ============================================================================
// Example 1: Basic Device Registration
// ============================================================================

/// Register a new device with the backend server
async fn example_basic_registration() -> Result<(), String> {
    // Create authentication service
    let auth_service = AuthService::new("jwt-secret".to_string());

    // Create network service with temporary credentials
    let mut network_service = NetworkService::new(
        "https://api.example.com".to_string(),
        auth_service,
        "user-jwt-token".to_string(),
        "temp-device".to_string(),
        "temp-secret".to_string(),
    );

    // Register device and save credentials
    let credentials_path = Path::new(".anywheraoor");
    network_service.register_and_save_device(credentials_path).await?;

    println!("Device registered successfully!");
    println!("Device ID: {}", network_service.get_device_id());

    Ok(())
}

// ============================================================================
// Example 2: Load Existing Credentials
// ============================================================================

/// Load device credentials from a saved file
async fn example_load_credentials() -> Result<(), String> {
    let auth_service = AuthService::new("jwt-secret".to_string());
    let credentials_path = Path::new(".anywhereaoor");

    // Check if credentials exist
    if !NetworkService::has_device_credentials(credentials_path) {
        return Err("Credentials not found. Please register device first.".to_string());
    }

    // Load credentials from file
    let network_service = NetworkService::from_saved_credentials(
        "https://api.example.com".to_string(),
        auth_service,
        credentials_path,
    )?;

    println!("Credentials loaded successfully!");
    println!("Device ID: {}", network_service.get_device_id());

    Ok(())
}

// ============================================================================
// Example 3: Manual Credential Management
// ============================================================================

/// Example of manually managing device credentials
async fn example_manual_credential_management() -> Result<(), String> {
    let auth_service = AuthService::new("jwt-secret".to_string());

    // Create with temporary credentials
    let mut network_service = NetworkService::new(
        "https://api.example.com".to_string(),
        auth_service,
        "user-jwt".to_string(),
        "old-device-id".to_string(),
        "old-secret".to_string(),
    );

    // Update credentials (e.g., after rotation)
    network_service.update_device_credentials(
        "new-device-id".to_string(),
        "new-secret".to_string(),
    );

    println!("Device ID: {}", network_service.get_device_id());

    Ok(())
}

// ============================================================================
// Example 4: Full Registration and Storage Flow
// ============================================================================

/// Complete example showing registration with environment variables
async fn example_full_registration_flow() -> Result<(), String> {
    // Get configuration from environment
    let user_jwt = std::env::var("ANYWHERE_DOOR_USER_JWT")
        .map_err(|_| "ANYWHERE_DOOR_USER_JWT environment variable required".to_string())?;

    let server_url = std::env::var("ANYWHERE_DOOR_SERVER_URL")
        .unwrap_or_else(|_| "https://api.anywhereaoor.com".to_string());

    let credentials_path_str = std::env::var("ANYWHERE_DOOR_CREDENTIALS_PATH")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.anywhereaoor", home)
        });

    let credentials_path = Path::new(&credentials_path_str);

    // Check existing credentials
    if NetworkService::has_device_credentials(credentials_path) {
        println!("Loading existing credentials from: {}", credentials_path.display());
        let network_service = NetworkService::from_saved_credentials(
            server_url,
            AuthService::new("jwt-secret".to_string()),
            credentials_path,
        )?;
        println!("✓ Device loaded: {}", network_service.get_device_id());
    } else {
        println!("Registering new device...");
        let auth_service = AuthService::new("jwt-secret".to_string());
        let mut network_service = NetworkService::new(
            server_url,
            auth_service,
            user_jwt,
            "temp-device".to_string(),
            "temp-secret".to_string(),
        );

        network_service.register_and_save_device(credentials_path).await?;
        println!("✓ Device registered and saved");
    }

    Ok(())
}

// ============================================================================
// Example 5: Error Handling
// ============================================================================

/// Example showing proper error handling
async fn example_error_handling() -> Result<(), String> {
    // Attempt registration with explicit error handling
    match example_basic_registration().await {
        Ok(_) => println!("Registration successful"),
        Err(e) => {
            eprintln!("Registration failed: {}", e);
            eprintln!("Please check:");
            eprintln!("  - Server URL is reachable");
            eprintln!("  - JWT token is valid");
            eprintln!("  - Credentials directory is writable");
            return Err(e);
        }
    }

    Ok(())
}

// ============================================================================
// Example 6: Credential Validation
// ============================================================================

/// Example showing credential validation
fn example_credential_validation() -> Result<(), String> {
    let credentials_path = Path::new(".anywhereaoor");

    // Check if file exists
    if !credentials_path.exists() {
        return Err("Credentials file not found".to_string());
    }

    // Try to load and validate
    match NetworkService::load_device_credentials(credentials_path) {
        Ok((device_id, _device_secret, jwt)) => {
            println!("Valid credentials found:");
            println!("  Device ID: {}", device_id);
            println!("  Device Secret: [REDACTED]");
            println!("  JWT: {}", &jwt[..std::cmp::min(20, jwt.len())]);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to load credentials: {}", e);
            eprintln!("The credentials file may be corrupted.");
            eprintln!("Delete it and re-register: rm {}", credentials_path.display());
            Err(e)
        }
    }
}

// ============================================================================
// Example Usage
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), String> {
    println!("=== Anywhere Door Device Authentication Examples ===\n");

    // Example 1: Basic Registration
    println!("Example 1: Basic Device Registration");
    println!("=====================================");
    if let Err(e) = example_basic_registration().await {
        eprintln!("Error: {}", e);
    }
    println!();

    // Example 2: Load Credentials
    println!("Example 2: Load Existing Credentials");
    println!("====================================");
    if let Err(e) = example_load_credentials().await {
        eprintln!("Error: {}", e);
    }
    println!();

    // Example 3: Manual Management
    println!("Example 3: Manual Credential Management");
    println!("=========================================");
    if let Err(e) = example_manual_credential_management().await {
        eprintln!("Error: {}", e);
    }
    println!();

    // Example 4: Full Flow
    println!("Example 4: Full Registration Flow");
    println!("==================================");
    if let Err(e) = example_full_registration_flow().await {
        eprintln!("Error: {}", e);
    }
    println!();

    // Example 5: Error Handling
    println!("Example 5: Error Handling");
    println!("==========================");
    if let Err(e) = example_error_handling().await {
        eprintln!("Error: {}", e);
    }
    println!();

    // Example 6: Validation
    println!("Example 6: Credential Validation");
    println!("================================");
    if let Err(e) = example_credential_validation() {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

// ============================================================================
// Integration with Service
// ============================================================================

/*
The device registration is automatically integrated into the service layer.
When the service starts, it:

1. Checks if device credentials exist
   - Path: ~/.anywhereaoor (configurable)
   - Or ANYWHERE_DOOR_CREDENTIALS_PATH

2. If credentials exist:
   - Loads them from file
   - Continues to normal operation

3. If credentials don't exist:
   - Prompts for ANYWHERE_DOOR_USER_JWT
   - Calls /auth/register-device endpoint
   - Saves credentials to file (permissions: 0600)
   - Continues to normal operation

For manual setup:
   export ANYWHERE_DOOR_USER_JWT="your-jwt-token"
   cargo run --release

To customize paths:
   export ANYWHERE_DOOR_CREDENTIALS_PATH="/custom/path"
   export ANYWHERE_DOOR_SERVER_URL="https://custom-api.com"
   cargo run --release

For production deployment:
   - Pre-register device and save credentials
   - Use configuration management for JWT
   - Implement credential rotation
   - Monitor device registration failures
   - Maintain audit logs
*/
