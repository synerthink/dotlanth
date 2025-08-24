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

//! Role management and assignment

use crate::error::{ApiError, ApiResult};
use crate::rbac::audit::AuditLogger;
use crate::rbac::permissions::{DotPermission, Permission};
use crate::rbac::roles::{Role, RoleHierarchy, UserRoleAssignment, create_default_roles};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Role manager for handling role operations
#[derive(Debug, Clone)]
pub struct RoleManager {
    /// Role hierarchy
    hierarchy: Arc<RwLock<RoleHierarchy>>,

    /// User role assignments
    user_assignments: Arc<RwLock<HashMap<String, Vec<UserRoleAssignment>>>>,

    /// Audit logger
    audit_logger: Arc<AuditLogger>,
}

impl RoleManager {
    /// Create a new role manager
    pub fn new(audit_logger: Arc<AuditLogger>) -> Self {
        let mut hierarchy = RoleHierarchy::new();

        // Add default system roles
        for role in create_default_roles() {
            hierarchy.add_role(role);
        }

        Self {
            hierarchy: Arc::new(RwLock::new(hierarchy)),
            user_assignments: Arc::new(RwLock::new(HashMap::new())),
            audit_logger,
        }
    }

    /// Create a new role
    pub async fn create_role(&self, role: Role, created_by: &str) -> ApiResult<()> {
        let mut hierarchy = self.hierarchy.write().await;

        // Check if role already exists
        if hierarchy.get_role(&role.id).is_some() {
            return Err(ApiError::Conflict {
                message: format!("Role with ID '{}' already exists", role.id),
            });
        }

        // Validate role hierarchy
        let mut temp_hierarchy = hierarchy.clone();
        temp_hierarchy.add_role(role.clone());
        temp_hierarchy.validate_hierarchy()?;

        // Add the role
        hierarchy.add_role(role.clone());

        // Log the action
        self.audit_logger.log_role_created(&role.id, created_by).await;

        Ok(())
    }

    /// Update an existing role
    pub async fn update_role(&self, role: Role, updated_by: &str) -> ApiResult<()> {
        let mut hierarchy = self.hierarchy.write().await;

        // Check if role exists
        if hierarchy.get_role(&role.id).is_none() {
            return Err(ApiError::NotFound {
                message: format!("Role with ID '{}' not found", role.id),
            });
        }

        // Check if it's a system role
        if let Some(existing_role) = hierarchy.get_role(&role.id) {
            if existing_role.is_system_role {
                return Err(ApiError::Forbidden {
                    message: "Cannot modify system roles".to_string(),
                });
            }
        }

        // Validate role hierarchy
        let mut temp_hierarchy = hierarchy.clone();
        temp_hierarchy.add_role(role.clone());
        temp_hierarchy.validate_hierarchy()?;

        // Update the role
        hierarchy.add_role(role.clone());

        // Log the action
        self.audit_logger.log_role_updated(&role.id, updated_by).await;

        Ok(())
    }

    /// Delete a role
    pub async fn delete_role(&self, role_id: &str, deleted_by: &str) -> ApiResult<()> {
        let mut hierarchy = self.hierarchy.write().await;
        let mut assignments = self.user_assignments.write().await;

        // Check if role exists
        if hierarchy.get_role(role_id).is_none() {
            return Err(ApiError::NotFound {
                message: format!("Role with ID '{}' not found", role_id),
            });
        }

        // Remove role from hierarchy
        hierarchy.remove_role(role_id)?;

        // Remove all assignments for this role
        for user_assignments in assignments.values_mut() {
            user_assignments.retain(|assignment| assignment.role_id != role_id);
        }

        // Log the action
        self.audit_logger.log_role_deleted(role_id, deleted_by).await;

        Ok(())
    }

    /// Get a role by ID
    pub async fn get_role(&self, role_id: &str) -> Option<Role> {
        let hierarchy = self.hierarchy.read().await;
        hierarchy.get_role(role_id).cloned()
    }

    /// Get all roles
    pub async fn get_all_roles(&self) -> Vec<Role> {
        let hierarchy = self.hierarchy.read().await;
        hierarchy.get_all_roles().into_iter().cloned().collect()
    }

    /// Assign a role to a user
    pub async fn assign_role(&self, user_id: &str, role_id: &str, assigned_by: &str) -> ApiResult<()> {
        let hierarchy = self.hierarchy.read().await;
        let mut assignments = self.user_assignments.write().await;

        // Check if role exists
        if hierarchy.get_role(role_id).is_none() {
            return Err(ApiError::NotFound {
                message: format!("Role with ID '{}' not found", role_id),
            });
        }

        // Get or create user assignments
        let user_assignments = assignments.entry(user_id.to_string()).or_insert_with(Vec::new);

        // Check if assignment already exists
        if user_assignments.iter().any(|a| a.role_id == role_id && !a.is_expired()) {
            return Err(ApiError::Conflict {
                message: format!("User '{}' already has role '{}'", user_id, role_id),
            });
        }

        // Create new assignment
        let assignment = UserRoleAssignment::new(user_id.to_string(), role_id.to_string(), assigned_by.to_string());

        user_assignments.push(assignment);

        // Log the action
        self.audit_logger.log_role_assigned(user_id, role_id, assigned_by).await;

        Ok(())
    }

    /// Assign a temporary role to a user
    pub async fn assign_temporary_role(&self, user_id: &str, role_id: &str, expires_at: DateTime<Utc>, assigned_by: &str, reason: Option<String>) -> ApiResult<()> {
        let hierarchy = self.hierarchy.read().await;
        let mut assignments = self.user_assignments.write().await;

        // Check if role exists
        if hierarchy.get_role(role_id).is_none() {
            return Err(ApiError::NotFound {
                message: format!("Role with ID '{}' not found", role_id),
            });
        }

        // Get or create user assignments
        let user_assignments = assignments.entry(user_id.to_string()).or_insert_with(Vec::new);

        // Create temporary assignment
        let mut assignment = UserRoleAssignment::temporary(user_id.to_string(), role_id.to_string(), assigned_by.to_string(), expires_at);

        if let Some(reason) = reason {
            assignment = assignment.with_reason(reason);
        }

        user_assignments.push(assignment);

        // Log the action
        self.audit_logger.log_temporary_role_assigned(user_id, role_id, expires_at, assigned_by).await;

        Ok(())
    }

    /// Revoke a role from a user
    pub async fn revoke_role(&self, user_id: &str, role_id: &str, revoked_by: &str) -> ApiResult<()> {
        let mut assignments = self.user_assignments.write().await;

        // Get user assignments
        let user_assignments = assignments.get_mut(user_id).ok_or_else(|| ApiError::NotFound {
            message: format!("No role assignments found for user '{}'", user_id),
        })?;

        // Find and remove the assignment
        let initial_len = user_assignments.len();
        user_assignments.retain(|assignment| assignment.role_id != role_id);

        if user_assignments.len() == initial_len {
            return Err(ApiError::NotFound {
                message: format!("User '{}' does not have role '{}'", user_id, role_id),
            });
        }

        // Log the action
        self.audit_logger.log_role_revoked(user_id, role_id, revoked_by).await;

        Ok(())
    }

    /// Get user role assignments
    pub async fn get_user_roles(&self, user_id: &str) -> Vec<UserRoleAssignment> {
        let assignments = self.user_assignments.read().await;

        assignments
            .get(user_id)
            .map(|assignments| assignments.iter().filter(|assignment| !assignment.is_expired()).cloned().collect())
            .unwrap_or_default()
    }

    /// Get effective permissions for a user
    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        let assignments = self.get_user_roles(user_id).await;
        let mut hierarchy = self.hierarchy.write().await;
        let mut all_permissions = Vec::new();

        for assignment in assignments {
            let permissions = hierarchy.get_effective_permissions(&assignment.role_id)?;
            all_permissions.extend(permissions);
        }

        // Remove duplicates
        all_permissions.sort_by(|a, b| a.key().cmp(&b.key()));
        all_permissions.dedup_by(|a, b| a.key() == b.key());

        Ok(all_permissions)
    }

    /// Get effective dot permissions for a user
    pub async fn get_user_dot_permissions(&self, user_id: &str) -> ApiResult<Vec<DotPermission>> {
        let assignments = self.get_user_roles(user_id).await;
        let mut hierarchy = self.hierarchy.write().await;
        let mut all_dot_permissions = Vec::new();

        for assignment in assignments {
            let dot_permissions = hierarchy.get_effective_dot_permissions(&assignment.role_id)?;
            all_dot_permissions.extend(dot_permissions);
        }

        // Remove duplicates based on dot_id and operations
        all_dot_permissions.sort_by(|a, b| a.dot_id.cmp(&b.dot_id).then_with(|| a.operations.join(",").cmp(&b.operations.join(","))));
        all_dot_permissions.dedup_by(|a, b| a.dot_id == b.dot_id && a.operations == b.operations);

        Ok(all_dot_permissions)
    }

    /// Clean up expired role assignments
    pub async fn cleanup_expired_assignments(&self) {
        let mut assignments = self.user_assignments.write().await;

        for user_assignments in assignments.values_mut() {
            user_assignments.retain(|assignment| !assignment.is_expired());
        }

        // Remove users with no assignments
        assignments.retain(|_, user_assignments| !user_assignments.is_empty());
    }

    /// Get role hierarchy
    pub async fn get_role_hierarchy(&self, role_id: &str) -> ApiResult<Vec<String>> {
        let mut hierarchy = self.hierarchy.write().await;
        hierarchy.resolve_hierarchy(role_id)
    }

    /// Validate role hierarchy
    pub async fn validate_hierarchy(&self) -> ApiResult<()> {
        let mut hierarchy = self.hierarchy.write().await;
        hierarchy.validate_hierarchy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::audit::AuditLogger;

    async fn create_test_manager() -> RoleManager {
        let audit_logger = Arc::new(AuditLogger::new());
        RoleManager::new(audit_logger)
    }

    #[tokio::test]
    async fn test_role_creation() {
        let manager = create_test_manager().await;

        let role = Role::new("test_role".to_string(), "Test Role".to_string(), "A test role".to_string());

        assert!(manager.create_role(role, "admin").await.is_ok());
    }

    #[tokio::test]
    async fn test_role_assignment() {
        let manager = create_test_manager().await;

        // Assign existing system role
        assert!(manager.assign_role("user123", "user", "admin").await.is_ok());

        // Check assignment
        let assignments = manager.get_user_roles("user123").await;
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].role_id, "user");
    }

    #[tokio::test]
    async fn test_role_revocation() {
        let manager = create_test_manager().await;

        // Assign and then revoke role
        manager.assign_role("user123", "user", "admin").await.unwrap();
        assert!(manager.revoke_role("user123", "user", "admin").await.is_ok());

        // Check no assignments
        let assignments = manager.get_user_roles("user123").await;
        assert!(assignments.is_empty());
    }

    #[tokio::test]
    async fn test_effective_permissions() {
        let manager = create_test_manager().await;

        // Assign role and get permissions
        manager.assign_role("user123", "user", "admin").await.unwrap();
        let permissions = manager.get_user_permissions("user123").await.unwrap();

        assert!(!permissions.is_empty());
    }
}
