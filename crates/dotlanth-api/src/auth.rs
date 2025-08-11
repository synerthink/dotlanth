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

//! Authentication and authorization utilities

use crate::error::{ApiError, ApiResult};
use crate::models::{LoginRequest, TokenResponse, UserProfile};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: String,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// Not before (Unix timestamp)
    pub nbf: i64,

    /// User roles
    pub roles: Vec<String>,

    /// User permissions
    pub permissions: Vec<String>,
}

impl Claims {
    /// Create new claims for a user
    pub fn new(user_id: String, roles: Vec<String>, permissions: Vec<String>, expires_in: Duration) -> Self {
        let now = Utc::now();
        let exp = (now + expires_in).timestamp();

        Self {
            sub: user_id,
            iss: "dotlanth-api".to_string(),
            aud: "dotlanth".to_string(),
            exp,
            iat: now.timestamp(),
            nbf: now.timestamp(),
            roles,
            permissions,
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Check if the user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if the user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}

/// JWT token manager
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtManager {
    /// Create a new JWT manager with a secret key
    pub fn new(secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["dotlanth-api"]);
        validation.set_audience(&["dotlanth"]);

        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
        }
    }

    /// Generate a random secret key
    pub fn generate_secret() -> ApiResult<String> {
        let rng = SystemRandom::new();
        let mut secret = vec![0u8; 32];
        rng.fill(&mut secret).map_err(|_| ApiError::InternalServerError {
            message: "Failed to generate random secret".to_string(),
        })?;
        Ok(base64::encode(&secret))
    }

    /// Create a JWT token
    pub fn create_token(&self, claims: &Claims) -> ApiResult<String> {
        let header = Header::new(Algorithm::HS256);
        encode(&header, claims, &self.encoding_key).map_err(|e| ApiError::JwtError(e))
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> ApiResult<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation).map_err(|e| ApiError::JwtError(e))?;

        let claims = token_data.claims;

        // Additional validation
        if claims.is_expired() {
            return Err(ApiError::Unauthorized {
                message: "Token has expired".to_string(),
            });
        }

        Ok(claims)
    }
}

/// Authentication service
pub struct AuthService {
    jwt_manager: JwtManager,
    // In a real implementation, this would connect to a user database
    // For now, we'll use a simple in-memory store
    users: std::collections::HashMap<String, User>,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(jwt_secret: &str) -> Self {
        let mut users = std::collections::HashMap::new();

        // Add a default admin user for testing
        users.insert(
            "admin".to_string(),
            User {
                id: "admin".to_string(),
                username: "admin".to_string(),
                email: "admin@dotlanth.com".to_string(),
                password_hash: "admin".to_string(), // In production, this would be hashed
                roles: vec!["admin".to_string(), "user".to_string()],
                permissions: vec![
                    "read:documents".to_string(),
                    "write:documents".to_string(),
                    "delete:documents".to_string(),
                    "deploy:dots".to_string(),
                    "execute:dots".to_string(),
                    "admin:users".to_string(),
                ],
                created_at: Utc::now(),
                last_login: None,
                is_active: true,
            },
        );

        // Add a default user for testing
        users.insert(
            "user".to_string(),
            User {
                id: "user".to_string(),
                username: "user".to_string(),
                email: "user@dotlanth.com".to_string(),
                password_hash: "user".to_string(), // In production, this would be hashed
                roles: vec!["user".to_string()],
                permissions: vec!["read:documents".to_string(), "write:documents".to_string(), "execute:dots".to_string()],
                created_at: Utc::now(),
                last_login: None,
                is_active: true,
            },
        );

        Self {
            jwt_manager: JwtManager::new(jwt_secret),
            users,
        }
    }

    /// Authenticate a user and return a JWT token
    pub async fn login(&mut self, request: LoginRequest) -> ApiResult<TokenResponse> {
        // Find the user
        let user = self.users.get_mut(&request.username).ok_or_else(|| ApiError::Unauthorized {
            message: "Invalid username or password".to_string(),
        })?;

        // In production, you would hash the password and compare
        if user.password_hash != request.password {
            return Err(ApiError::Unauthorized {
                message: "Invalid username or password".to_string(),
            });
        }

        if !user.is_active {
            return Err(ApiError::Forbidden {
                message: "Account is disabled".to_string(),
            });
        }

        // Update last login
        user.last_login = Some(Utc::now());

        // Create JWT claims
        let expires_in = Duration::hours(24); // 24 hour expiration
        let claims = Claims::new(user.id.clone(), user.roles.clone(), user.permissions.clone(), expires_in);

        // Generate token
        let token = self.jwt_manager.create_token(&claims)?;

        Ok(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: expires_in.num_seconds() as u64,
        })
    }

    /// Validate a JWT token and return the claims
    pub fn validate_token(&self, token: &str) -> ApiResult<Claims> {
        self.jwt_manager.validate_token(token)
    }

    /// Get user profile by user ID
    pub fn get_user_profile(&self, user_id: &str) -> ApiResult<UserProfile> {
        let user = self.users.get(user_id).ok_or_else(|| ApiError::NotFound {
            message: "User not found".to_string(),
        })?;

        Ok(UserProfile {
            id: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            permissions: user.permissions.clone(),
            created_at: user.created_at,
            last_login: user.last_login,
        })
    }

    /// Check if a user has the required permissions
    pub fn check_permissions(&self, claims: &Claims, required_permissions: &[&str]) -> ApiResult<()> {
        for permission in required_permissions {
            if !claims.has_permission(permission) {
                return Err(ApiError::Forbidden {
                    message: format!("Missing required permission: {}", permission),
                });
            }
        }
        Ok(())
    }
}

/// User data structure (would typically be in a database)
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Extract JWT token from Authorization header
pub fn extract_token_from_header(auth_header: &str) -> ApiResult<&str> {
    if let Some(token) = auth_header.strip_prefix("Bearer ") {
        Ok(token)
    } else {
        Err(ApiError::Unauthorized {
            message: "Invalid authorization header format".to_string(),
        })
    }
}
