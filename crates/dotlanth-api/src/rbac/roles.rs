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

//! Role definitions and management

use crate::error::{ApiError, ApiResult};
use crate::rbac::permissions::{DotPermission, Permission};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Role definition with hierarchical support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Role {
    /// Unique role identifier
    pub id: String,

    /// Human-readable role name
    pub name: String,

    /// Role description
    pub description: String,

    /// Direct permissions assigned to this role
    pub permissions: Vec<Permission>,

    /// Dot-specific permissions
    pub dot_permissions: Vec<DotPermission>,

    /// Parent roles for inheritance
    pub parent_roles: Vec<String>,

    /// Whether this is a system-defined role
    pub is_system_role: bool,

    /// Role creation timestamp
    pub created_at: DateTime<Utc>,

    /// Role last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Role metadata
    pub metadata: HashMap<String, String>,
}

impl Role {
    /// Create a new role
    pub fn new(id: String, name: String, description: String) -> Self {
        let now = Utc::now();

        Self {
            id,
            name,
            description,
            permissions: Vec::new(),
            dot_permissions: Vec::new(),
            parent_roles: Vec::new(),
            is_system_role: false,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Create a system role
    pub fn system_role(id: String, name: String, description: String) -> Self {
        let mut role = Self::new(id, name, description);
        role.is_system_role = true;
        role
    }

    /// Add a permission to this role
    pub fn add_permission(&mut self, permission: Permission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a permission from this role
    pub fn remove_permission(&mut self, permission: &Permission) {
        if let Some(pos) = self.permissions.iter().position(|p| p == permission) {
            self.permissions.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Add a dot permission to this role
    pub fn add_dot_permission(&mut self, dot_permission: DotPermission) {
        if !self.dot_permissions.contains(&dot_permission) {
            self.dot_permissions.push(dot_permission);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a dot permission from this role
    pub fn remove_dot_permission(&mut self, dot_permission: &DotPermission) {
        if let Some(pos) = self.dot_permissions.iter().position(|p| p == dot_permission) {
            self.dot_permissions.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Add a parent role
    pub fn add_parent_role(&mut self, parent_role_id: String) {
        if !self.parent_roles.contains(&parent_role_id) {
            self.parent_roles.push(parent_role_id);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a parent role
    pub fn remove_parent_role(&mut self, parent_role_id: &str) {
        if let Some(pos) = self.parent_roles.iter().position(|id| id == parent_role_id) {
            self.parent_roles.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// User role assignment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserRoleAssignment {
    /// User ID
    pub user_id: String,

    /// Role ID
    pub role_id: String,

    /// Assignment timestamp
    pub assigned_at: DateTime<Utc>,

    /// Assignment expiration (optional)
    pub expires_at: Option<DateTime<Utc>>,

    /// Who assigned this role
    pub assigned_by: String,

    /// Assignment reason/context
    pub reason: Option<String>,
}

impl UserRoleAssignment {
    /// Create a new role assignment
    pub fn new(user_id: String, role_id: String, assigned_by: String) -> Self {
        Self {
            user_id,
            role_id,
            assigned_at: Utc::now(),
            expires_at: None,
            assigned_by,
            reason: None,
        }
    }

    /// Create a temporary role assignment
    pub fn temporary(user_id: String, role_id: String, assigned_by: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            user_id,
            role_id,
            assigned_at: Utc::now(),
            expires_at: Some(expires_at),
            assigned_by,
            reason: None,
        }
    }

    /// Set assignment reason
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    /// Check if assignment is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at { Utc::now() > expires_at } else { false }
    }
}

/// Role hierarchy resolver for computing effective permissions
#[derive(Debug, Clone)]
pub struct RoleHierarchy {
    /// All roles in the system
    roles: HashMap<String, Role>,

    /// Cached role hierarchies to avoid recomputation
    hierarchy_cache: HashMap<String, Vec<String>>,
}

impl RoleHierarchy {
    /// Create a new role hierarchy
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            hierarchy_cache: HashMap::new(),
        }
    }

    /// Add a role to the hierarchy
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.id.clone(), role);
        self.invalidate_cache();
    }

    /// Remove a role from the hierarchy
    pub fn remove_role(&mut self, role_id: &str) -> ApiResult<()> {
        if let Some(role) = self.roles.get(role_id) {
            if role.is_system_role {
                return Err(ApiError::Forbidden {
                    message: "Cannot remove system roles".to_string(),
                });
            }
        }

        self.roles.remove(role_id);
        self.invalidate_cache();
        Ok(())
    }

    /// Get a role by ID
    pub fn get_role(&self, role_id: &str) -> Option<&Role> {
        self.roles.get(role_id)
    }

    /// Get all roles
    pub fn get_all_roles(&self) -> Vec<&Role> {
        self.roles.values().collect()
    }

    /// Resolve the complete hierarchy for a role (including inherited roles)
    pub fn resolve_hierarchy(&mut self, role_id: &str) -> ApiResult<Vec<String>> {
        if let Some(cached) = self.hierarchy_cache.get(role_id) {
            return Ok(cached.clone());
        }

        let mut visited = HashSet::new();
        let mut hierarchy = Vec::new();

        self.resolve_hierarchy_recursive(role_id, &mut visited, &mut hierarchy)?;

        self.hierarchy_cache.insert(role_id.to_string(), hierarchy.clone());
        Ok(hierarchy)
    }

    /// Recursively resolve role hierarchy
    fn resolve_hierarchy_recursive(&self, role_id: &str, visited: &mut HashSet<String>, hierarchy: &mut Vec<String>) -> ApiResult<()> {
        if visited.contains(role_id) {
            return Err(ApiError::BadRequest {
                message: format!("Circular role dependency detected for role: {}", role_id),
            });
        }

        visited.insert(role_id.to_string());

        if let Some(role) = self.roles.get(role_id) {
            // Add current role to hierarchy
            hierarchy.push(role_id.to_string());

            // Recursively resolve parent roles
            for parent_role_id in &role.parent_roles {
                self.resolve_hierarchy_recursive(parent_role_id, visited, hierarchy)?;
            }
        }

        visited.remove(role_id);
        Ok(())
    }

    /// Get effective permissions for a role (including inherited permissions)
    pub fn get_effective_permissions(&mut self, role_id: &str) -> ApiResult<Vec<Permission>> {
        let hierarchy = self.resolve_hierarchy(role_id)?;
        let mut permissions = Vec::new();
        let mut seen_permissions = HashSet::new();

        for role_id in hierarchy {
            if let Some(role) = self.roles.get(&role_id) {
                for permission in &role.permissions {
                    let key = permission.key();
                    if !seen_permissions.contains(&key) {
                        permissions.push(permission.clone());
                        seen_permissions.insert(key);
                    }
                }
            }
        }

        Ok(permissions)
    }

    /// Get effective dot permissions for a role (including inherited permissions)
    pub fn get_effective_dot_permissions(&mut self, role_id: &str) -> ApiResult<Vec<DotPermission>> {
        let hierarchy = self.resolve_hierarchy(role_id)?;
        let mut dot_permissions = Vec::new();
        let mut seen_permissions = HashSet::new();

        for role_id in hierarchy {
            if let Some(role) = self.roles.get(&role_id) {
                for dot_permission in &role.dot_permissions {
                    let key = format!("{}:{}", dot_permission.dot_id, dot_permission.operations.join(","));
                    if !seen_permissions.contains(&key) {
                        dot_permissions.push(dot_permission.clone());
                        seen_permissions.insert(key);
                    }
                }
            }
        }

        Ok(dot_permissions)
    }

    /// Validate role hierarchy (check for cycles)
    pub fn validate_hierarchy(&mut self) -> ApiResult<()> {
        for role_id in self.roles.keys().cloned().collect::<Vec<_>>() {
            self.resolve_hierarchy(&role_id)?;
        }
        Ok(())
    }

    /// Invalidate hierarchy cache
    fn invalidate_cache(&mut self) {
        self.hierarchy_cache.clear();
    }
}

impl Default for RoleHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

/// Create default system roles
pub fn create_default_roles() -> Vec<Role> {
    vec![
        // Super Admin role
        {
            let mut role = Role::system_role("super_admin".to_string(), "Super Administrator".to_string(), "Full system access with all permissions".to_string());

            role.add_permission(Permission::new("*".to_string(), "*".to_string()));
            role.add_dot_permission(DotPermission::new("*".to_string(), vec!["*".to_string()]));
            role
        },
        // Admin role
        {
            let mut role = Role::system_role("admin".to_string(), "Administrator".to_string(), "Administrative access to most system functions".to_string());

            role.add_permission(Permission::new("users".to_string(), "*".to_string()));
            role.add_permission(Permission::new("roles".to_string(), "*".to_string()));
            role.add_permission(Permission::new("dots".to_string(), "*".to_string()));
            role.add_permission(Permission::new("collections".to_string(), "*".to_string()));
            role.add_dot_permission(DotPermission::new(
                "*".to_string(),
                vec!["read".to_string(), "write".to_string(), "execute".to_string(), "deploy".to_string()],
            ));
            role
        },
        // User role
        {
            let mut role = Role::system_role("user".to_string(), "User".to_string(), "Standard user access".to_string());

            role.add_permission(Permission::new("dots".to_string(), "read".to_string()));
            role.add_permission(Permission::new("dots".to_string(), "execute".to_string()));
            role.add_permission(Permission::new("collections".to_string(), "read".to_string()));
            role.add_permission(Permission::new("collections".to_string(), "write".to_string()));
            role
        },
        // Read-only role
        {
            let mut role = Role::system_role("readonly".to_string(), "Read Only".to_string(), "Read-only access to system resources".to_string());

            role.add_permission(Permission::new("*".to_string(), "read".to_string()));
            role.add_dot_permission(DotPermission::new("*".to_string(), vec!["read".to_string()]));
            role
        },
        // Dot Developer role
        {
            let mut role = Role::system_role("dot_developer".to_string(), "Dot Developer".to_string(), "Can develop and deploy dots".to_string());

            role.add_permission(Permission::new("dots".to_string(), "*".to_string()));
            role.add_dot_permission(DotPermission::new(
                "*".to_string(),
                vec!["read".to_string(), "write".to_string(), "execute".to_string(), "deploy".to_string()],
            ));
            role.add_parent_role("user".to_string());
            role
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_creation() {
        let role = Role::new("test_role".to_string(), "Test Role".to_string(), "A test role".to_string());

        assert_eq!(role.id, "test_role");
        assert_eq!(role.name, "Test Role");
        assert!(!role.is_system_role);
        assert!(role.permissions.is_empty());
        assert!(role.parent_roles.is_empty());
    }

    #[test]
    fn test_role_permissions() {
        let mut role = Role::new("test_role".to_string(), "Test Role".to_string(), "A test role".to_string());

        let permission = Permission::new("dots".to_string(), "read".to_string());
        role.add_permission(permission.clone());

        assert_eq!(role.permissions.len(), 1);
        assert_eq!(role.permissions[0], permission);

        role.remove_permission(&permission);
        assert!(role.permissions.is_empty());
    }

    #[test]
    fn test_role_hierarchy() {
        let mut hierarchy = RoleHierarchy::new();

        let parent_role = Role::new("parent".to_string(), "Parent Role".to_string(), "Parent role".to_string());

        let mut child_role = Role::new("child".to_string(), "Child Role".to_string(), "Child role".to_string());
        child_role.add_parent_role("parent".to_string());

        hierarchy.add_role(parent_role);
        hierarchy.add_role(child_role);

        let resolved = hierarchy.resolve_hierarchy("child").unwrap();
        assert!(resolved.contains(&"child".to_string()));
        assert!(resolved.contains(&"parent".to_string()));
    }

    #[test]
    fn test_user_role_assignment() {
        let assignment = UserRoleAssignment::new("user123".to_string(), "admin".to_string(), "system".to_string());

        assert_eq!(assignment.user_id, "user123");
        assert_eq!(assignment.role_id, "admin");
        assert!(!assignment.is_expired());
    }

    #[test]
    fn test_temporary_assignment() {
        let expires_at = Utc::now() - chrono::Duration::hours(1);
        let assignment = UserRoleAssignment::temporary("user123".to_string(), "admin".to_string(), "system".to_string(), expires_at);

        assert!(assignment.is_expired());
    }
}
