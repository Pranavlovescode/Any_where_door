use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Data Structures
// ============================================================================

/// JWT Claims for user authentication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserClaims {
    pub sub: String,        // user ID
    pub username: String,
    pub iat: i64,          // issued at
    pub exp: i64,          // expiration
}

/// Device registration data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_secret: String,
    pub user_id: String,
    pub registered_at: i64,
}

/// Request authentication data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthRequest {
    pub jwt: String,
    pub device_id: String,
    pub timestamp: i64,
    pub signature: String,  // HMAC-SHA256 signature
    pub data: String,       // The actual request data to be signed
}

/// Response for user login
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub jwt: String,
    pub expires_in: i64,    // seconds
    pub user_id: String,
}

/// Response for device registration
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterDeviceResponse {
    pub device_id: String,
    pub device_secret: String,
    pub created_at: i64,
}

/// Verification result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub error: Option<String>,
}

// ============================================================================
// AuthService: Central authentication handler
// ============================================================================

pub struct AuthService {
    jwt_secret: String,
    devices: HashMap<String, DeviceInfo>,  // device_id -> DeviceInfo
}

impl AuthService {
    /// Initialize the auth service with a JWT secret
    pub fn new(jwt_secret: String) -> Self {
        AuthService {
            jwt_secret,
            devices: HashMap::new(),
        }
    }

    // ========================================================================
    // User Authentication (JWT)
    // ========================================================================

    /// User login - Issues a JWT token
    /// Returns: (JWT token, expires_in_seconds)
    pub fn user_login(&self, user_id: &str, username: &str) -> Result<LoginResponse, String> {
        let now = Utc::now();
        let expires_in = 86400; // 24 hours

        let claims = UserClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(expires_in)).timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        ).map_err(|e| format!("Failed to encode JWT: {}", e))?;

        Ok(LoginResponse {
            jwt: token,
            expires_in,
            user_id: user_id.to_string(),
        })
    }

    /// Verify a JWT token
    pub fn verify_jwt(&self, token: &str) -> Result<UserClaims, String> {
        decode::<UserClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| format!("JWT verification failed: {}", e))
    }

    // ========================================================================
    // Device Authentication
    // ========================================================================

    /// Register a device for an authenticated user
    /// Returns: device_id and device_secret
    pub fn register_device(
        &mut self,
        user_id: &str,
        _jwt_token: &str, // Verify in caller: this token should be valid and belong to user_id
    ) -> Result<RegisterDeviceResponse, String> {
        let device_id = Uuid::new_v4().to_string();
        let device_secret = self.generate_device_secret();

        let device_info = DeviceInfo {
            device_id: device_id.clone(),
            device_secret: device_secret.clone(),
            user_id: user_id.to_string(),
            registered_at: Utc::now().timestamp(),
        };

        self.devices.insert(device_id.clone(), device_info);

        Ok(RegisterDeviceResponse {
            device_id,
            device_secret,
            created_at: Utc::now().timestamp(),
        })
    }

    /// Generate a random device secret
    fn generate_device_secret(&self) -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    }

    // ========================================================================
    // Request Signing & Verification
    // ========================================================================

    /// Generate a signature for a request using device secret
    /// Signature = HMAC-SHA256(device_secret, timestamp + device_id + data)
    pub fn generate_signature(
        device_secret: &str,
        device_id: &str,
        timestamp: i64,
        data: &str,
    ) -> Result<String, String> {
        let message = format!("{}:{}:{}", timestamp, device_id, data);

        let mut mac = Hmac::<Sha256>::new_from_slice(device_secret.as_bytes())
            .map_err(|e| format!("Invalid secret: {}", e))?;

        mac.update(message.as_bytes());
        let result = mac.finalize();

        Ok(hex::encode(result.into_bytes()))
    }

    /// Verify a complete authenticated request
    pub fn verify_request(&self, auth_request: &AuthRequest) -> VerificationResult {
        // Step 1: Verify JWT token
        let claims = match self.verify_jwt(&auth_request.jwt) {
            Ok(claims) => claims,
            Err(e) => {
                return VerificationResult {
                    valid: false,
                    user_id: None,
                    device_id: None,
                    error: Some(format!("JWT verification failed: {}", e)),
                };
            }
        };

        // Step 2: Verify device is registered and get secret
        let device_info = match self.devices.get(&auth_request.device_id) {
            Some(info) => info,
            None => {
                return VerificationResult {
                    valid: false,
                    user_id: None,
                    device_id: None,
                    error: Some("Device not registered".to_string()),
                };
            }
        };

        // Step 3: Verify device belongs to the user
        if device_info.user_id != claims.sub {
            return VerificationResult {
                valid: false,
                user_id: None,
                device_id: None,
                error: Some("Device does not belong to this user".to_string()),
            };
        }

        // Step 4: Verify signature
        match Self::generate_signature(
            &device_info.device_secret,
            &auth_request.device_id,
            auth_request.timestamp,
            &auth_request.data,
        ) {
            Ok(expected_signature) => {
                if expected_signature != auth_request.signature {
                    return VerificationResult {
                        valid: false,
                        user_id: Some(claims.sub),
                        device_id: Some(auth_request.device_id.clone()),
                        error: Some("Signature mismatch".to_string()),
                    };
                }
            }
            Err(e) => {
                return VerificationResult {
                    valid: false,
                    user_id: Some(claims.sub),
                    device_id: Some(auth_request.device_id.clone()),
                    error: Some(format!("Signature verification error: {}", e)),
                };
            }
        }

        // Step 5: All checks passed
        VerificationResult {
            valid: true,
            user_id: Some(claims.sub),
            device_id: Some(auth_request.device_id.clone()),
            error: None,
        }
    }

    /// Get device info (for debugging/admin)
    pub fn get_device(&self, device_id: &str) -> Option<DeviceInfo> {
        self.devices.get(device_id).cloned()
    }

    /// List all devices for a user
    pub fn list_user_devices(&self, user_id: &str) -> Vec<DeviceInfo> {
        self.devices
            .values()
            .filter(|d| d.user_id == user_id)
            .cloned()
            .collect()
    }

    /// Unregister a device
    pub fn unregister_device(&mut self, device_id: &str) -> Result<(), String> {
        if self.devices.remove(device_id).is_some() {
            Ok(())
        } else {
            Err("Device not found".to_string())
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_login_and_verify_jwt() {
        let auth = AuthService::new("test-secret".to_string());
        
        let login = auth.user_login("user123", "john_doe").unwrap();
        assert!(!login.jwt.is_empty());
        assert_eq!(login.user_id, "user123");

        let claims = auth.verify_jwt(&login.jwt).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "john_doe");
    }

    #[test]
    fn test_device_registration() {
        let mut auth = AuthService::new("test-secret".to_string());
        let jwt = auth.user_login("user123", "john_doe").unwrap().jwt;

        let device = auth.register_device("user123", &jwt).unwrap();
        assert!(!device.device_id.is_empty());
        assert!(!device.device_secret.is_empty());
    }

    #[test]
    fn test_request_signature_and_verification() {
        let mut auth = AuthService::new("test-secret".to_string());
        
        // User login
        let jwt = auth.user_login("user123", "john_doe").unwrap().jwt;
        
        // Register device
        let device = auth.register_device("user123", &jwt).unwrap();
        
        // Create a signed request
        let timestamp = Utc::now().timestamp();
        let data = r#"{"action":"sync","files":["file1.txt","file2.txt"]}"#;
        let signature = AuthService::generate_signature(
            &device.device_secret,
            &device.device_id,
            timestamp,
            data,
        ).unwrap();

        // Verify the request
        let auth_request = AuthRequest {
            jwt: jwt.clone(),
            device_id: device.device_id,
            timestamp,
            signature,
            data: data.to_string(),
        };

        let result = auth.verify_request(&auth_request);
        assert!(result.valid);
        assert_eq!(result.user_id, Some("user123".to_string()));
    }

    #[test]
    fn test_invalid_signature() {
        let mut auth = AuthService::new("test-secret".to_string());
        
        let jwt = auth.user_login("user123", "john_doe").unwrap().jwt;
        let device = auth.register_device("user123", &jwt).unwrap();
        
        let timestamp = Utc::now().timestamp();
        let data = r#"{"action":"sync"}"#;

        // Create request with wrong signature
        let auth_request = AuthRequest {
            jwt: jwt.clone(),
            device_id: device.device_id,
            timestamp,
            signature: "invalid_signature".to_string(),
            data: data.to_string(),
        };

        let result = auth.verify_request(&auth_request);
        assert!(!result.valid);
        assert_eq!(result.error, Some("Signature mismatch".to_string()));
    }

    #[test]
    fn test_device_not_found() {
        let auth = AuthService::new("test-secret".to_string());
        let jwt = auth.user_login("user123", "john_doe").unwrap().jwt;

        let auth_request = AuthRequest {
            jwt,
            device_id: "nonexistent".to_string(),
            timestamp: Utc::now().timestamp(),
            signature: "fake".to_string(),
            data: "data".to_string(),
        };

        let result = auth.verify_request(&auth_request);
        assert!(!result.valid);
        assert_eq!(result.error, Some("Device not registered".to_string()));
    }
}
