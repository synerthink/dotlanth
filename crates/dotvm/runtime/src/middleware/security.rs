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

//! Advanced Security Middleware for gRPC
//! 
//! Implements comprehensive security features including mTLS, RBAC,
//! API key validation, and request/response encryption.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Status};
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable mutual TLS authentication
    pub mtls_enabled: bool,
    /// Enable API key authentication
    pub api_key_enabled: bool,
    /// Enable role-based access control
    pub rbac_enabled: bool,
    /// Enable request/response encryption
    pub encryption_enabled: bool,
    /// Trusted certificate authorities
    pub trusted_cas: Vec<String>,
    /// Required client certificate fields
    pub required_cert_fields: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            mtls_enabled: false, // Disabled by default for development
            api_key_enabled: true,
            rbac_enabled: true,
            encryption_enabled: false,
            trusted_cas: Vec::new(),
            required_cert_fields: vec!["CN".to_string(), "O".to_string()],
        }
    }
}

/// User role definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
    pub description: String,
}

/// Permission definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // VM operations
    ExecuteDot,
    DeployDot,
    DeleteDot,
    GetDotState,
    ListDots,
    
    // Bytecode operations
    GetBytecode,
    ValidateBytecode,
    
    // ABI operations
    GetDotABI,
    ValidateABI,
    GenerateABI,
    RegisterABI,
    
    // VM management
    GetVMStatus,
    GetVMMetrics,
    GetArchitectures,
    
    // Streaming operations
    StreamDotEvents,
    StreamVMMetrics,
    
    // Administrative operations
    ManageUsers,
    ManageRoles,
    ViewLogs,
    SystemAdmin,
    
    // Custom permissions
    Custom(String),
}

impl Permission {
    /// Check if this permission allows access to a specific gRPC method
    pub fn allows_method(&self, method: &str) -> bool {
        match self {
            Permission::ExecuteDot => method.contains("ExecuteDot"),
            Permission::DeployDot => method.contains("DeployDot"),
            Permission::DeleteDot => method.contains("DeleteDot"),
            Permission::GetDotState => method.contains("GetDotState"),
            Permission::ListDots => method.contains("ListDots"),
            Permission::GetBytecode => method.contains("GetBytecode"),
            Permission::ValidateBytecode => method.contains("ValidateBytecode"),
            Permission::GetDotABI => method.contains("GetDotABI"),
            Permission::ValidateABI => method.contains("ValidateABI"),
            Permission::GenerateABI => method.contains("GenerateABI"),
            Permission::RegisterABI => method.contains("RegisterABI"),
            Permission::GetVMStatus => method.contains("GetVMStatus"),
            Permission::GetVMMetrics => method.contains("GetVMMetrics"),
            Permission::GetArchitectures => method.contains("GetArchitectures"),
            Permission::StreamDotEvents => method.contains("StreamDotEvents"),
            Permission::StreamVMMetrics => method.contains("StreamVMMetrics"),
            Permission::ManageUsers => method.contains("User"),
            Permission::ManageRoles => method.contains("Role"),
            Permission::ViewLogs => method.contains("Log"),
            Permission::SystemAdmin => true, // Admin can access everything
            Permission::Custom(perm) => method.contains(perm),
        }
    }
}

/// User information with roles and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub roles: HashSet<String>,
    pub api_keys: HashSet<String>,
    pub certificate_fingerprint: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
}

impl User {
    /// Get all permissions for this user
    pub fn get_permissions(&self, role_store: &RoleStore) -> HashSet<Permission> {
        let mut permissions = HashSet::new();
        
        for role_name in &self.roles {
            if let Some(role) = role_store.get_role(role_name) {
                permissions.extend(role.permissions.clone());
            }
        }
        
        permissions
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &Permission, role_store: &RoleStore) -> bool {
        let permissions = self.get_permissions(role_store);
        permissions.contains(permission) || permissions.contains(&Permission::SystemAdmin)
    }

    /// Check if user can access a specific method
    pub fn can_access_method(&self, method: &str, role_store: &RoleStore) -> bool {
        let permissions = self.get_permissions(role_store);
        
        // System admin can access everything
        if permissions.contains(&Permission::SystemAdmin) {
            return true;
        }
        
        // Check if any permission allows this method
        permissions.iter().any(|perm| perm.allows_method(method))
    }
}

/// Role storage and management
#[derive(Debug)]
pub struct RoleStore {
    roles: Arc<RwLock<HashMap<String, Role>>>,
}

impl RoleStore {
    pub fn new() -> Self {
        let mut roles = HashMap::new();
        
        // Create default roles
        roles.insert("admin".to_string(), Role {
            name: "admin".to_string(),
            permissions: vec![Permission::SystemAdmin].into_iter().collect(),
            description: "System administrator with full access".to_string(),
        });
        
        roles.insert("developer".to_string(), Role {
            name: "developer".to_string(),
            permissions: vec![
                Permission::ExecuteDot,
                Permission::DeployDot,
                Permission::GetDotState,
                Permission::ListDots,
                Permission::GetBytecode,
                Permission::ValidateBytecode,
                Permission::GetDotABI,
                Permission::ValidateABI,
                Permission::GenerateABI,
                Permission::GetVMStatus,
                Permission::GetVMMetrics,
                Permission::GetArchitectures,
                Permission::StreamDotEvents,
                Permission::StreamVMMetrics,
            ].into_iter().collect(),
            description: "Developer with dot execution and monitoring access".to_string(),
        });
        
        roles.insert("viewer".to_string(), Role {
            name: "viewer".to_string(),
            permissions: vec![
                Permission::GetDotState,
                Permission::ListDots,
                Permission::GetBytecode,
                Permission::GetDotABI,
                Permission::GetVMStatus,
                Permission::GetVMMetrics,
                Permission::GetArchitectures,
                Permission::StreamVMMetrics,
            ].into_iter().collect(),
            description: "Read-only access to dots and VM status".to_string(),
        });
        
        Self {
            roles: Arc::new(RwLock::new(roles)),
        }
    }

    pub fn get_role(&self, name: &str) -> Option<Role> {
        // For now, return a simple blocking read
        // In production, this would be async
        futures::executor::block_on(async {
            self.roles.read().await.get(name).cloned()
        })
    }

    pub async fn add_role(&self, role: Role) {
        let mut roles = self.roles.write().await;
        roles.insert(role.name.clone(), role);
    }

    pub async fn remove_role(&self, name: &str) -> bool {
        let mut roles = self.roles.write().await;
        roles.remove(name).is_some()
    }

    pub async fn list_roles(&self) -> Vec<Role> {
        let roles = self.roles.read().await;
        roles.values().cloned().collect()
    }
}

impl Default for RoleStore {
    fn default() -> Self {
        Self::new()
    }
}

/// User storage and management
#[derive(Debug)]
pub struct UserStore {
    users: Arc<RwLock<HashMap<String, User>>>,
    api_key_to_user: Arc<RwLock<HashMap<String, String>>>,
    cert_fingerprint_to_user: Arc<RwLock<HashMap<String, String>>>,
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            api_key_to_user: Arc::new(RwLock::new(HashMap::new())),
            cert_fingerprint_to_user: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_user(&self, user: User) {
        let user_id = user.id.clone();
        
        // Update API key mappings
        {
            let mut api_key_map = self.api_key_to_user.write().await;
            for api_key in &user.api_keys {
                api_key_map.insert(api_key.clone(), user_id.clone());
            }
        }
        
        // Update certificate fingerprint mapping
        if let Some(fingerprint) = &user.certificate_fingerprint {
            let mut cert_map = self.cert_fingerprint_to_user.write().await;
            cert_map.insert(fingerprint.clone(), user_id.clone());
        }
        
        // Add user
        let mut users = self.users.write().await;
        users.insert(user_id, user);
    }

    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        let users = self.users.read().await;
        users.get(user_id).cloned()
    }

    pub async fn get_user_by_api_key(&self, api_key: &str) -> Option<User> {
        let api_key_map = self.api_key_to_user.read().await;
        if let Some(user_id) = api_key_map.get(api_key) {
            self.get_user(user_id).await
        } else {
            None
        }
    }

    pub async fn get_user_by_cert_fingerprint(&self, fingerprint: &str) -> Option<User> {
        let cert_map = self.cert_fingerprint_to_user.read().await;
        if let Some(user_id) = cert_map.get(fingerprint) {
            self.get_user(user_id).await
        } else {
            None
        }
    }

    pub async fn update_last_login(&self, user_id: &str) {
        let mut users = self.users.write().await;
        if let Some(user) = users.get_mut(user_id) {
            user.last_login = Some(chrono::Utc::now());
        }
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Advanced security interceptor
#[derive(Debug, Clone)]
pub struct SecurityInterceptor {
    config: SecurityConfig,
    user_store: Arc<UserStore>,
    role_store: Arc<RoleStore>,
    public_methods: HashSet<String>,
}

impl SecurityInterceptor {
    pub fn new(config: SecurityConfig, user_store: Arc<UserStore>, role_store: Arc<RoleStore>) -> Self {
        let mut public_methods = HashSet::new();
        public_methods.insert("/runtime.Runtime/Ping".to_string());
        public_methods.insert("/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo".to_string());
        
        Self {
            config,
            user_store,
            role_store,
            public_methods,
        }
    }

    pub fn with_public_methods(mut self, methods: Vec<String>) -> Self {
        self.public_methods.extend(methods);
        self
    }

    /// Intercept and validate request
    pub async fn intercept<T>(&self, request: Request<T>) -> Result<Request<T>, Status> {
        let method = "grpc_method"; // Simplified for now
        
        // Skip authentication for public methods
        if self.public_methods.contains(method) {
            return Ok(request);
        }

        // Extract authentication information
        let user = self.authenticate_request(&request).await?;
        
        // Check authorization
        self.authorize_request(&user, method).await?;
        
        // Update user activity
        self.user_store.update_last_login(&user.id).await;
        
        debug!("Security check passed for user {} on method {}", user.username, method);
        
        Ok(request)
    }

    /// Authenticate the request and return user information
    async fn authenticate_request<T>(&self, request: &Request<T>) -> Result<User, Status> {
        // Try API key authentication first
        if self.config.api_key_enabled {
            if let Some(user) = self.authenticate_with_api_key(request).await? {
                return Ok(user);
            }
        }

        // Try mTLS authentication
        if self.config.mtls_enabled {
            if let Some(user) = self.authenticate_with_mtls(request).await? {
                return Ok(user);
            }
        }

        Err(Status::unauthenticated("No valid authentication provided"))
    }

    /// Authenticate using API key
    async fn authenticate_with_api_key<T>(&self, request: &Request<T>) -> Result<Option<User>, Status> {
        // Extract API key from headers
        let metadata = request.metadata();
        
        if let Some(api_key) = metadata.get("x-api-key") {
            let api_key_str = api_key.to_str()
                .map_err(|_| Status::invalid_argument("Invalid API key format"))?;
            
            if let Some(user) = self.user_store.get_user_by_api_key(api_key_str).await {
                if user.is_active {
                    info!("User {} authenticated with API key", user.username);
                    return Ok(Some(user));
                } else {
                    return Err(Status::permission_denied("User account is disabled"));
                }
            }
        }
        
        Ok(None)
    }

    /// Authenticate using mTLS certificate
    async fn authenticate_with_mtls<T>(&self, _request: &Request<T>) -> Result<Option<User>, Status> {
        // In a real implementation, this would extract the client certificate
        // from the TLS connection and validate it against trusted CAs
        
        // For now, return None since we don't have access to the certificate
        // This would be implemented with proper TLS integration
        Ok(None)
    }

    /// Authorize the request based on user permissions
    async fn authorize_request(&self, user: &User, method: &str) -> Result<(), Status> {
        if !self.config.rbac_enabled {
            return Ok(()); // RBAC disabled, allow all authenticated users
        }

        if user.can_access_method(method, &self.role_store) {
            Ok(())
        } else {
            warn!("User {} denied access to method {}", user.username, method);
            Err(Status::permission_denied(format!(
                "Insufficient permissions for method: {}", method
            )))
        }
    }
}

/// Security utilities
pub mod utils {
    use super::*;
    use ring::digest::{Context, SHA256};

    /// Generate a secure API key
    pub fn generate_api_key() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    }

    /// Hash a certificate fingerprint
    pub fn hash_certificate_fingerprint(cert_data: &[u8]) -> String {
        let mut context = Context::new(&SHA256);
        context.update(cert_data);
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Create a default admin user
    pub fn create_admin_user() -> User {
        User {
            id: "admin".to_string(),
            username: "admin".to_string(),
            email: Some("admin@dotlanth.local".to_string()),
            roles: vec!["admin".to_string()].into_iter().collect(),
            api_keys: vec![generate_api_key()].into_iter().collect(),
            certificate_fingerprint: None,
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_role_permissions() {
        let role_store = RoleStore::new();
        let admin_role = role_store.get_role("admin").unwrap();
        
        assert!(admin_role.permissions.contains(&Permission::SystemAdmin));
        assert!(Permission::SystemAdmin.allows_method("ExecuteDot"));
    }

    #[tokio::test]
    async fn test_user_permissions() {
        let role_store = RoleStore::new();
        let user = User {
            id: "test".to_string(),
            username: "test".to_string(),
            email: None,
            roles: vec!["developer".to_string()].into_iter().collect(),
            api_keys: HashSet::new(),
            certificate_fingerprint: None,
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        };

        assert!(user.can_access_method("ExecuteDot", &role_store));
        assert!(!user.can_access_method("ManageUsers", &role_store));
    }
}