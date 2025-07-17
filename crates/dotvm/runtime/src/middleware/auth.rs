// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Authentication middleware for gRPC services

use std::sync::Arc;
use tonic::{Request, Status};
use tracing::{error, info, warn};
use ring::{hmac, rand};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // Subject (user ID)
    pub exp: u64,       // Expiration time
    pub iat: u64,       // Issued at
    pub roles: Vec<String>, // User roles
    pub permissions: Vec<String>, // User permissions
}

/// JWT Validator for authentication
pub struct JwtValidator {
    secret_key: hmac::Key,
}

impl JwtValidator {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret_key: hmac::Key::new(hmac::HMAC_SHA256, secret),
        }
    }

    pub fn generate_secret() -> Vec<u8> {
        let rng = rand::SystemRandom::new();
        let mut secret = vec![0u8; 32];
        ring::rand::SecureRandom::fill(&rng, &mut secret).unwrap();
        secret
    }

    pub fn create_token(&self, claims: &Claims) -> Result<String, String> {
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let header_b64 = general_purpose::URL_SAFE_NO_PAD.encode(header);
        
        let claims_json = serde_json::to_string(claims)
            .map_err(|e| format!("Failed to serialize claims: {}", e))?;
        let claims_b64 = general_purpose::URL_SAFE_NO_PAD.encode(claims_json);
        
        let message = format!("{}.{}", header_b64, claims_b64);
        let signature = hmac::sign(&self.secret_key, message.as_bytes());
        let signature_b64 = general_purpose::URL_SAFE_NO_PAD.encode(signature.as_ref());
        
        Ok(format!("{}.{}", message, signature_b64))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid token format".to_string());
        }

        let message = format!("{}.{}", parts[0], parts[1]);
        let signature = general_purpose::URL_SAFE_NO_PAD.decode(parts[2])
            .map_err(|_| "Invalid signature encoding")?;

        // Verify signature
        hmac::verify(&self.secret_key, message.as_bytes(), &signature)
            .map_err(|_| "Invalid signature")?;

        // Decode claims
        let claims_json = general_purpose::URL_SAFE_NO_PAD.decode(parts[1])
            .map_err(|_| "Invalid claims encoding")?;
        let claims: Claims = serde_json::from_slice(&claims_json)
            .map_err(|e| format!("Invalid claims format: {}", e))?;

        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if claims.exp < now {
            return Err("Token expired".to_string());
        }

        Ok(claims)
    }
}

/// Authentication interceptor
#[derive(Clone)]
pub struct AuthInterceptor {
    validator: Arc<JwtValidator>,
    public_methods: Vec<String>,
}

impl AuthInterceptor {
    pub fn new(validator: Arc<JwtValidator>) -> Self {
        Self {
            validator,
            public_methods: vec![
                "/runtime.Runtime/Ping".to_string(),
                "/vm_service.VmService/GetArchitectures".to_string(),
            ],
        }
    }

    pub fn with_public_methods(mut self, methods: Vec<String>) -> Self {
        self.public_methods = methods;
        self
    }

    pub fn intercept<T>(&self, request: Request<T>) -> Result<Request<T>, Status> {
        let method = "grpc_method"; // Simplified for now
        
        // Skip authentication for public methods
        if self.public_methods.contains(&method.to_string()) {
            info!("Allowing public method: {}", method);
            return Ok(request);
        }

        // Extract authorization header
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| {
                warn!("Missing authorization header for method: {}", method);
                Status::unauthenticated("Missing authorization header")
            })?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;

        // Extract Bearer token
        let token = auth_str
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("Invalid authorization format"))?;

        // Validate token
        let claims = self.validator.validate_token(token)
            .map_err(|e| {
                error!("Token validation failed: {}", e);
                Status::unauthenticated("Invalid token")
            })?;

        info!("Authenticated user: {} with roles: {:?}", claims.sub, claims.roles);

        // Add claims to request extensions for use in handlers
        let mut request = request;
        request.extensions_mut().insert(claims);

        Ok(request)
    }
}

/// Helper to extract claims from request
pub fn extract_claims<T>(request: &Request<T>) -> Option<&Claims> {
    request.extensions().get::<Claims>()
}

/// Helper to check if user has required role
pub fn has_role(claims: &Claims, required_role: &str) -> bool {
    claims.roles.contains(&required_role.to_string())
}

/// Helper to check if user has required permission
pub fn has_permission(claims: &Claims, required_permission: &str) -> bool {
    claims.permissions.contains(&required_permission.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_creation_and_validation() {
        let secret = JwtValidator::generate_secret();
        let validator = JwtValidator::new(&secret);

        let claims = Claims {
            sub: "user123".to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600,
            iat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            roles: vec!["admin".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
        };

        let token = validator.create_token(&claims).unwrap();
        let validated_claims = validator.validate_token(&token).unwrap();

        assert_eq!(claims.sub, validated_claims.sub);
        assert_eq!(claims.roles, validated_claims.roles);
    }

    #[test]
    fn test_expired_token() {
        let secret = JwtValidator::generate_secret();
        let validator = JwtValidator::new(&secret);

        let claims = Claims {
            sub: "user123".to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 1, // Expired
            iat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            roles: vec!["admin".to_string()],
            permissions: vec!["read".to_string()],
        };

        let token = validator.create_token(&claims).unwrap();
        assert!(validator.validate_token(&token).is_err());
    }
}