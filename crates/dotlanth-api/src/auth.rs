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
use crate::models::{LoginRequest, RegisterRequest, TokenResponse, UserProfile};
use crate::rbac::system::RBACSystem;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;

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
    pub exp: usize,

    /// Issued at (Unix timestamp)
    pub iat: usize,

    /// Not before (Unix timestamp)
    pub nbf: usize,

    /// User roles
    pub roles: Vec<String>,

    /// User permissions
    pub permissions: Vec<String>,

    /// Dot-specific permissions
    pub dot_permissions: HashMap<String, Vec<String>>,
}

/// Refresh token data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenData {
    /// User ID
    pub user_id: String,

    /// Token expiration time
    pub expires_at: DateTime<Utc>,

    /// Token creation time
    pub created_at: DateTime<Utc>,

    /// Whether the token is revoked
    pub is_revoked: bool,
}

/// Token pair containing access and refresh tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    /// JWT access token
    pub access_token: String,

    /// Refresh token
    pub refresh_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Access token expiration time in seconds
    pub expires_in: u64,
}

impl Claims {
    /// Create new claims for a user
    pub fn new(user_id: String, roles: Vec<String>, permissions: Vec<String>, dot_permissions: HashMap<String, Vec<String>>, expires_in: Duration) -> Self {
        let now = Utc::now();
        let exp = (now + expires_in).timestamp() as usize;

        Self {
            sub: user_id,
            iss: "dotlanth-api".to_string(),
            aud: "dotlanth".to_string(),
            exp,
            iat: now.timestamp() as usize,
            nbf: now.timestamp() as usize,
            roles,
            permissions,
            dot_permissions,
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() as usize > self.exp
    }

    /// Check if the user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if the user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Check if the user has a specific permission for a dot
    pub fn has_dot_permission(&self, dot_id: &str, permission: &str) -> bool {
        self.dot_permissions.get(dot_id).map(|perms| perms.contains(&permission.to_string())).unwrap_or(false)
    }
}

/// JWT Authentication System with dot-level permissions
pub struct JWTAuthSystem {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    token_blacklist: Arc<DashMap<String, SystemTime>>,
    refresh_tokens: Arc<DashMap<String, RefreshTokenData>>,
    argon2: Argon2<'static>,
}

impl JWTAuthSystem {
    /// Create a new JWT authentication system
    pub fn new(secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["dotlanth-api"]);
        validation.set_audience(&["dotlanth"]);

        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
            token_blacklist: Arc::new(DashMap::new()),
            refresh_tokens: Arc::new(DashMap::new()),
            argon2: Argon2::default(),
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

    /// Hash a password using Argon2
    pub fn hash_password(&self, password: &str) -> ApiResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self.argon2.hash_password(password.as_bytes(), &salt).map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to hash password: {}", e),
        })?;
        Ok(password_hash.to_string())
    }

    /// Verify a password against its hash
    pub fn verify_password(&self, password: &str, hash: &str) -> ApiResult<bool> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| ApiError::InternalServerError {
            message: format!("Invalid password hash: {}", e),
        })?;

        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Generate a token pair (access + refresh)
    pub fn generate_token(&self, user: &User) -> ApiResult<TokenPair> {
        // Create access token
        let access_expires_in = Duration::hours(1); // 1 hour for access token
        let claims = Claims::new(user.id.clone(), user.roles.clone(), user.permissions.clone(), user.dot_permissions.clone(), access_expires_in);

        let access_token = self.create_token(&claims)?;

        // Create refresh token
        let refresh_token = self.generate_refresh_token(&user.id)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: access_expires_in.num_seconds() as u64,
        })
    }

    /// Create a JWT token
    fn create_token(&self, claims: &Claims) -> ApiResult<String> {
        let header = Header::new(Algorithm::HS256);
        encode(&header, claims, &self.encoding_key).map_err(|e| ApiError::JwtError(e))
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> ApiResult<Claims> {
        // Check if token is blacklisted
        if self.token_blacklist.contains_key(token) {
            return Err(ApiError::Unauthorized {
                message: "Token has been revoked".to_string(),
            });
        }

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

    /// Generate a refresh token
    fn generate_refresh_token(&self, user_id: &str) -> ApiResult<String> {
        let rng = SystemRandom::new();
        let mut token_bytes = vec![0u8; 32];
        rng.fill(&mut token_bytes).map_err(|_| ApiError::InternalServerError {
            message: "Failed to generate refresh token".to_string(),
        })?;

        let refresh_token = base64::encode(&token_bytes);
        let expires_at = Utc::now() + Duration::days(30); // 30 days for refresh token

        let token_data = RefreshTokenData {
            user_id: user_id.to_string(),
            expires_at,
            created_at: Utc::now(),
            is_revoked: false,
        };

        self.refresh_tokens.insert(refresh_token.clone(), token_data);
        Ok(refresh_token)
    }

    /// Refresh an access token using a refresh token
    pub fn refresh_token(&self, refresh_token: &str) -> ApiResult<TokenPair> {
        let token_data = self.refresh_tokens.get(refresh_token).ok_or_else(|| ApiError::Unauthorized {
            message: "Invalid refresh token".to_string(),
        })?;

        if token_data.is_revoked {
            return Err(ApiError::Unauthorized {
                message: "Refresh token has been revoked".to_string(),
            });
        }

        if Utc::now() > token_data.expires_at {
            return Err(ApiError::Unauthorized {
                message: "Refresh token has expired".to_string(),
            });
        }

        // For this implementation, we need access to user data
        // In a real implementation, this would fetch from a database
        Err(ApiError::InternalServerError {
            message: "Refresh token functionality requires user database integration".to_string(),
        })
    }

    /// Blacklist a token (for logout)
    pub fn blacklist_token(&self, token: &str) -> ApiResult<()> {
        self.token_blacklist.insert(token.to_string(), SystemTime::now());
        Ok(())
    }

    /// Revoke a refresh token
    pub fn revoke_refresh_token(&self, refresh_token: &str) -> ApiResult<()> {
        if let Some(mut token_data) = self.refresh_tokens.get_mut(refresh_token) {
            token_data.is_revoked = true;
        }
        Ok(())
    }

    /// Clean up expired tokens (should be called periodically)
    pub fn cleanup_expired_tokens(&self) {
        let now = SystemTime::now();
        let utc_now = Utc::now();

        // Clean up blacklisted tokens older than 24 hours
        self.token_blacklist.retain(|_, &mut timestamp| {
            now.duration_since(timestamp)
                .map(|duration| duration.as_secs() < 86400) // 24 hours
                .unwrap_or(false)
        });

        // Clean up expired refresh tokens
        self.refresh_tokens.retain(|_, token_data| utc_now < token_data.expires_at && !token_data.is_revoked);
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
    pub dot_permissions: HashMap<String, Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Authentication service
pub struct AuthService {
    jwt_auth: JWTAuthSystem,
    // In a real implementation, this would connect to a user database
    // For now, we'll use a simple in-memory store
    users: std::collections::HashMap<String, User>,
    // RBAC system integration
    rbac_system: Option<std::sync::Arc<RBACSystem>>,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(jwt_secret: &str) -> ApiResult<Self> {
        let jwt_auth = JWTAuthSystem::new(jwt_secret);
        let mut users = std::collections::HashMap::new();

        // Create default admin user with proper password hashing
        let admin_password_hash = jwt_auth.hash_password("admin")?;
        let mut admin_dot_permissions = HashMap::new();
        admin_dot_permissions.insert(
            "*".to_string(),
            vec!["read".to_string(), "write".to_string(), "execute".to_string(), "deploy".to_string(), "delete".to_string()],
        );

        users.insert(
            "admin".to_string(),
            User {
                id: "admin".to_string(),
                username: "admin".to_string(),
                email: "admin@dotlanth.com".to_string(),
                password_hash: admin_password_hash,
                roles: vec!["admin".to_string(), "user".to_string()],
                permissions: vec![
                    "read:documents".to_string(),
                    "write:documents".to_string(),
                    "delete:documents".to_string(),
                    "deploy:dots".to_string(),
                    "execute:dots".to_string(),
                    "admin:users".to_string(),
                ],
                dot_permissions: admin_dot_permissions,
                created_at: Utc::now(),
                last_login: None,
                is_active: true,
            },
        );

        // Create default user with proper password hashing
        let user_password_hash = jwt_auth.hash_password("user")?;
        let mut user_dot_permissions = HashMap::new();
        user_dot_permissions.insert("user-dots".to_string(), vec!["read".to_string(), "write".to_string(), "execute".to_string()]);

        users.insert(
            "user".to_string(),
            User {
                id: "user".to_string(),
                username: "user".to_string(),
                email: "user@dotlanth.com".to_string(),
                password_hash: user_password_hash,
                roles: vec!["user".to_string()],
                permissions: vec!["read:documents".to_string(), "write:documents".to_string(), "execute:dots".to_string()],
                dot_permissions: user_dot_permissions,
                created_at: Utc::now(),
                last_login: None,
                is_active: true,
            },
        );

        Ok(Self { jwt_auth, users, rbac_system: None })
    }

    /// Register a new user
    pub async fn register(&mut self, request: RegisterRequest) -> ApiResult<UserProfile> {
        // Check if username already exists
        if self.users.contains_key(&request.username) {
            return Err(ApiError::Conflict {
                message: "Username already exists".to_string(),
            });
        }

        // Check if email already exists
        for user in self.users.values() {
            if user.email == request.email {
                return Err(ApiError::Conflict {
                    message: "Email already exists".to_string(),
                });
            }
        }

        // Hash the password
        let password_hash = self.jwt_auth.hash_password(&request.password)?;

        // Create new user
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: request.username.clone(),
            email: request.email.clone(),
            password_hash,
            roles: vec!["user".to_string()],
            permissions: vec!["read:documents".to_string(), "write:documents".to_string(), "execute:dots".to_string()],
            dot_permissions: HashMap::new(),
            created_at: Utc::now(),
            last_login: None,
            is_active: true,
        };

        let profile = UserProfile {
            id: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            permissions: user.permissions.clone(),
            created_at: user.created_at,
            last_login: user.last_login,
        };

        self.users.insert(request.username, user);
        Ok(profile)
    }

    /// Authenticate a user and return a JWT token pair
    pub async fn login(&mut self, request: LoginRequest) -> ApiResult<TokenPair> {
        // Find the user
        let user = self.users.get_mut(&request.username).ok_or_else(|| ApiError::Unauthorized {
            message: "Invalid username or password".to_string(),
        })?;

        // Verify password
        if !self.jwt_auth.verify_password(&request.password, &user.password_hash)? {
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

        // Generate token pair
        let token_pair = self.jwt_auth.generate_token(user)?;
        Ok(token_pair)
    }

    /// Refresh an access token
    pub async fn refresh_token(&self, refresh_token: &str) -> ApiResult<TokenPair> {
        self.jwt_auth.refresh_token(refresh_token)
    }

    /// Logout a user (blacklist their token)
    pub async fn logout(&self, access_token: &str) -> ApiResult<()> {
        self.jwt_auth.blacklist_token(access_token)
    }

    /// Validate a JWT token and return the claims
    pub fn validate_token(&self, token: &str) -> ApiResult<Claims> {
        self.jwt_auth.validate_token(token)
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

    /// Check if a user has the required dot permissions
    pub fn check_dot_permissions(&self, claims: &Claims, dot_id: &str, required_permissions: &[&str]) -> ApiResult<()> {
        for permission in required_permissions {
            if !claims.has_dot_permission(dot_id, permission) && !claims.has_dot_permission("*", permission) {
                return Err(ApiError::Forbidden {
                    message: format!("Missing required dot permission: {} for dot: {}", permission, dot_id),
                });
            }
        }
        Ok(())
    }

    /// Clean up expired tokens
    pub fn cleanup_expired_tokens(&self) {
        self.jwt_auth.cleanup_expired_tokens();
    }

    /// Set RBAC system for integration
    pub fn set_rbac_system(&mut self, rbac_system: std::sync::Arc<RBACSystem>) {
        self.rbac_system = Some(rbac_system);
    }

    /// Get RBAC system
    pub fn rbac_system(&self) -> Option<&std::sync::Arc<RBACSystem>> {
        self.rbac_system.as_ref()
    }

    /// Update user permissions from RBAC system
    pub async fn update_user_permissions(&mut self, user_id: &str) -> ApiResult<()> {
        if let Some(rbac_system) = &self.rbac_system {
            if let Some(user) = self.users.get_mut(user_id) {
                // Get effective permissions from RBAC
                let permissions = rbac_system.get_user_permissions(user_id).await?;
                let dot_permissions = rbac_system.get_user_dot_permissions(user_id).await?;

                // Update user permissions
                user.permissions = permissions.iter().map(|p| p.key()).collect();

                // Update dot permissions
                user.dot_permissions.clear();
                for dot_perm in dot_permissions {
                    user.dot_permissions.insert(dot_perm.dot_id, dot_perm.operations);
                }

                // Notify RBAC system of user update
                rbac_system.update_user_from_auth(user).await?;
            }
        }
        Ok(())
    }

    /// Assign role to user (delegates to RBAC system)
    pub async fn assign_role_to_user(&self, user_id: &str, role_id: &str, assigned_by: &str) -> ApiResult<()> {
        if let Some(rbac_system) = &self.rbac_system {
            rbac_system.assign_role(user_id, role_id, assigned_by).await?;
            // Note: In a mutable context, we would call update_user_permissions here
        } else {
            return Err(ApiError::InternalServerError {
                message: "RBAC system not configured".to_string(),
            });
        }
        Ok(())
    }

    /// Revoke role from user (delegates to RBAC system)
    pub async fn revoke_role_from_user(&self, user_id: &str, role_id: &str, revoked_by: &str) -> ApiResult<()> {
        if let Some(rbac_system) = &self.rbac_system {
            rbac_system.revoke_role(user_id, role_id, revoked_by).await?;
            // Note: In a mutable context, we would call update_user_permissions here
        } else {
            return Err(ApiError::InternalServerError {
                message: "RBAC system not configured".to_string(),
            });
        }
        Ok(())
    }
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

/// Cookie configuration for secure token handling
#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub max_age: Option<Duration>,
    pub domain: Option<String>,
    pub path: String,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            secure: true,
            http_only: true,
            same_site: SameSite::Strict,
            max_age: Some(Duration::hours(1)),
            domain: None,
            path: "/".to_string(),
        }
    }
}

/// Same-site cookie attribute
#[derive(Debug, Clone)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SameSite::Strict => write!(f, "Strict"),
            SameSite::Lax => write!(f, "Lax"),
            SameSite::None => write!(f, "None"),
        }
    }
}
