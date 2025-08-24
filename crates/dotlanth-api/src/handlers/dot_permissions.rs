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

//! Dot permission handlers for the REST API

use crate::auth::{Claims, User};
use crate::error::{ApiError, ApiResult};
use crate::rbac::{DotABI, RBACSystem};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Request to load dot permissions from ABI
#[derive(Debug, Deserialize)]
pub struct LoadDotPermissionsRequest {
    /// Dot ID
    pub dot_id: String,
    /// Dot ABI containing permission definitions
    pub abi: DotABI,
}

/// Response for loading dot permissions
#[derive(Debug, Serialize)]
pub struct LoadDotPermissionsResponse {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Request to check dot operation permission
#[derive(Debug, Deserialize)]
pub struct CheckDotOperationRequest {
    /// Dot ID
    pub dot_id: String,
    /// Operation to check
    pub operation: String,
}

/// Response for dot operation permission check
#[derive(Debug, Serialize)]
pub struct CheckDotOperationResponse {
    /// Whether the operation is allowed
    pub allowed: bool,
    /// Reason for the decision
    pub reason: Option<String>,
}

/// Request to set dot owner
#[derive(Debug, Deserialize)]
pub struct SetDotOwnerRequest {
    /// Dot ID
    pub dot_id: String,
    /// New owner user ID
    pub owner_id: String,
}

/// Response for setting dot owner
#[derive(Debug, Serialize)]
pub struct SetDotOwnerResponse {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Response for getting user dot permissions
#[derive(Debug, Serialize)]
pub struct GetUserDotPermissionsResponse {
    /// List of operations the user can perform on the dot
    pub operations: Vec<String>,
    /// Whether the user is the dot owner
    pub is_owner: bool,
}

/// Dot permission handlers
pub struct DotPermissionHandlers {
    rbac_system: Arc<RBACSystem>,
}

impl DotPermissionHandlers {
    /// Create new dot permission handlers
    pub fn new(rbac_system: Arc<RBACSystem>) -> Self {
        Self { rbac_system }
    }

    /// Load dot permissions from ABI
    #[instrument(skip(self))]
    pub async fn load_dot_permissions(&self, claims: Claims, body: LoadDotPermissionsRequest) -> ApiResult<LoadDotPermissionsResponse> {
        info!("Loading dot permissions for dot: {}", body.dot_id);

        // Check if user has permission to manage dot permissions
        if !claims.has_permission("admin:dots") && !claims.has_dot_permission(&body.dot_id, "admin") {
            return Err(ApiError::Forbidden {
                message: "Insufficient permissions to load dot permissions".to_string(),
            });
        }

        // Load permissions from ABI
        self.rbac_system.load_dot_permissions(&body.dot_id, &body.abi).await?;

        Ok(LoadDotPermissionsResponse {
            success: true,
            message: format!("Permissions loaded successfully for dot {}", body.dot_id),
        })
    }

    /// Check if user can perform an operation on a dot
    #[instrument(skip(self))]
    pub async fn check_dot_operation(&self, claims: Claims, body: CheckDotOperationRequest) -> ApiResult<CheckDotOperationResponse> {
        info!("Checking dot operation: {} on dot: {}", body.operation, body.dot_id);

        // Create user from claims
        let user = User {
            id: claims.sub.clone(),
            username: claims.sub.clone(),                 // In a real implementation, this would be fetched from a user store
            email: format!("{}@example.com", claims.sub), // Placeholder
            password_hash: String::new(),                 // Not needed for permission checks
            roles: claims.roles.clone(),
            permissions: claims.permissions.clone(),
            dot_permissions: claims.dot_permissions.clone(),
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        };

        // Check permission
        let allowed = self.rbac_system.check_dot_operation(&user, &body.dot_id, &body.operation).await?;

        Ok(CheckDotOperationResponse {
            allowed,
            reason: if allowed { Some("Permission granted".to_string()) } else { Some("Permission denied".to_string()) },
        })
    }

    /// Set dot owner
    #[instrument(skip(self))]
    pub async fn set_dot_owner(&self, claims: Claims, body: SetDotOwnerRequest) -> ApiResult<SetDotOwnerResponse> {
        info!("Setting dot owner for dot: {} to user: {}", body.dot_id, body.owner_id);

        // Check if user has permission to set dot ownership
        if !claims.has_permission("admin:dots") && !claims.has_dot_permission(&body.dot_id, "admin") {
            return Err(ApiError::Forbidden {
                message: "Insufficient permissions to set dot ownership".to_string(),
            });
        }

        // Set dot owner
        self.rbac_system.set_dot_owner(&body.dot_id, &body.owner_id).await?;

        Ok(SetDotOwnerResponse {
            success: true,
            message: format!("Dot {} ownership set to user {}", body.dot_id, body.owner_id),
        })
    }

    /// Get user's permissions for a specific dot
    #[instrument(skip(self))]
    pub async fn get_user_dot_permissions(&self, claims: Claims, dot_id: String) -> ApiResult<GetUserDotPermissionsResponse> {
        info!("Getting user dot permissions for user: {} on dot: {}", claims.sub, dot_id);

        // Create user from claims
        let user = User {
            id: claims.sub.clone(),
            username: claims.sub.clone(),
            email: format!("{}@example.com", claims.sub),
            password_hash: String::new(),
            roles: claims.roles.clone(),
            permissions: claims.permissions.clone(),
            dot_permissions: claims.dot_permissions.clone(),
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        };

        // Get user's dot permissions
        let operations = self.rbac_system.get_user_dot_operation_permissions(&user, &dot_id).await?;
        let is_owner = self.rbac_system.is_dot_owner(&user, &dot_id).await?;

        Ok(GetUserDotPermissionsResponse { operations, is_owner })
    }

    /// Check if user is dot owner
    #[instrument(skip(self))]
    pub async fn check_dot_ownership(&self, claims: Claims, dot_id: String) -> ApiResult<serde_json::Value> {
        info!("Checking dot ownership for user: {} on dot: {}", claims.sub, dot_id);

        // Create user from claims
        let user = User {
            id: claims.sub.clone(),
            username: claims.sub.clone(),
            email: format!("{}@example.com", claims.sub),
            password_hash: String::new(),
            roles: claims.roles.clone(),
            permissions: claims.permissions.clone(),
            dot_permissions: claims.dot_permissions.clone(),
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        };

        // Check ownership
        let is_owner = self.rbac_system.is_dot_owner(&user, &dot_id).await?;

        Ok(serde_json::json!({
            "is_owner": is_owner,
            "dot_id": dot_id,
            "user_id": claims.sub
        }))
    }

    /// Get dot permission statistics
    #[instrument(skip(self))]
    pub async fn get_dot_permission_stats(&self, claims: Claims) -> ApiResult<crate::rbac::dot_permissions::PermissionStats> {
        info!("Getting dot permission statistics for user: {}", claims.sub);

        // Check if user has admin permissions
        if !claims.has_permission("admin:system") {
            return Err(ApiError::Forbidden {
                message: "Insufficient permissions to view system statistics".to_string(),
            });
        }

        // Get statistics from dot permission manager
        Ok(self.rbac_system.dot_permission_manager().get_permission_stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::audit::AuditLogger;
    use std::collections::HashMap;

    async fn create_test_rbac_system() -> Arc<RBACSystem> {
        Arc::new(RBACSystem::initialize().await.unwrap())
    }

    fn create_test_claims() -> Claims {
        let mut dot_permissions = HashMap::new();
        dot_permissions.insert("test_dot".to_string(), vec!["read".to_string(), "write".to_string()]);

        Claims {
            sub: "test_user".to_string(),
            iss: "test".to_string(),
            aud: "test".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
            nbf: chrono::Utc::now().timestamp() as usize,
            roles: vec!["user".to_string()],
            permissions: vec!["read:dots".to_string()],
            dot_permissions,
        }
    }

    fn create_test_abi() -> DotABI {
        use crate::rbac::dot_permissions::{OperationPermission2, PermissionConfig, RoleDefinition};

        let mut protected_ops = HashMap::new();
        protected_ops.insert(
            "write".to_string(),
            OperationPermission2 {
                required_roles: vec!["writer".to_string()],
                description: "Write operation".to_string(),
            },
        );

        let mut roles = HashMap::new();
        roles.insert(
            "writer".to_string(),
            RoleDefinition {
                description: "Can write data".to_string(),
                inherits: vec!["reader".to_string()],
                permissions: vec!["write".to_string()],
            },
        );

        DotABI {
            dot_name: "test_dot".to_string(),
            version: "1.0.0".to_string(),
            description: "Test dot".to_string(),
            permissions: Some(PermissionConfig {
                public_operations: vec!["read".to_string()],
                protected_operations: protected_ops,
                roles,
            }),
        }
    }

    #[tokio::test]
    async fn test_check_dot_operation() {
        let rbac_system = create_test_rbac_system().await;
        let handlers = DotPermissionHandlers::new(rbac_system.clone());
        let claims = create_test_claims();
        let abi = create_test_abi();

        // Load permissions first
        rbac_system.load_dot_permissions("test_dot", &abi).await.unwrap();

        // Test check operation
        let request_body = CheckDotOperationRequest {
            dot_id: "test_dot".to_string(),
            operation: "read".to_string(),
        };

        let response = handlers.check_dot_operation(claims, request_body).await.unwrap();

        assert!(response.allowed); // Should be allowed for public operations
    }

    #[tokio::test]
    async fn test_set_dot_owner() {
        let rbac_system = create_test_rbac_system().await;
        let handlers = DotPermissionHandlers::new(rbac_system);
        let mut claims = create_test_claims();
        claims.permissions.push("admin:dots".to_string());

        let request_body = SetDotOwnerRequest {
            dot_id: "test_dot".to_string(),
            owner_id: "new_owner".to_string(),
        };

        let response = handlers.set_dot_owner(claims, request_body).await.unwrap();

        assert!(response.success);
    }
}
