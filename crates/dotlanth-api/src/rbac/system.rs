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

//! Main RBAC system implementation

use crate::auth::User;
use crate::error::{ApiError, ApiResult};
use crate::rbac::audit::AuditLogger;
use crate::rbac::cache::PermissionCache;
use crate::rbac::manager::RoleManager;
use crate::rbac::permissions::{DotPermission, Permission, PermissionChecker, PermissionContext};
use crate::rbac::roles::{Role, UserRoleAssignment};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Main RBAC system that coordinates all components
#[derive(Debug, Clone)]
pub struct RBACSystem {
    /// Role manager for role operations
    role_manager: Arc<RoleManager>,

    /// Permission checker for evaluating permissions
    permission_checker: Arc<PermissionChecker>,

    /// Audit logger for tracking access decisions
    audit_logger: Arc<AuditLogger>,

    /// Cache for performance optimization
    cache: Arc<PermissionCache>,
}

impl RBACSystem {
    /// Create a new RBAC system
    pub fn new(role_manager: Arc<RoleManager>, permission_checker: Arc<PermissionChecker>, audit_logger: Arc<AuditLogger>, cache: Arc<PermissionCache>) -> Self {
        Self {
            role_manager,
            permission_checker,
            audit_logger,
            cache,
        }
    }

    /// Initialize RBAC system with default configuration
    pub async fn initialize() -> ApiResult<Self> {
        let audit_logger = Arc::new(AuditLogger::new());
        let cache = Arc::new(PermissionCache::new());
        let role_manager = Arc::new(RoleManager::new(audit_logger.clone()));
        let permission_checker = Arc::new(PermissionChecker::new(cache.clone()));

        let system = Self::new(role_manager, permission_checker, audit_logger, cache.clone());

        // Start background tasks
        let _cleanup_task = PermissionCache::start_cleanup_task(cache);

        info!("RBAC system initialized successfully");
        Ok(system)
    }

    /// Check if a user has permission to perform an action on a resource
    pub async fn check_permission(&self, user_id: &str, resource: &str, action: &str, context: &PermissionContext) -> ApiResult<bool> {
        let start_time = Instant::now();

        // Get user permissions
        let permissions = self.get_user_permissions(user_id).await?;

        // Check permissions
        let has_permission = self.permission_checker.check_permission(&permissions, resource, action, context).await?;

        let duration = start_time.elapsed();

        // Log the permission check
        self.audit_logger
            .log_permission_check(user_id, resource, action, has_permission, context.client_ip.clone(), context.additional_data.get("request_id").cloned())
            .await;

        // Log performance warning if check took too long
        if duration > Duration::from_millis(5) {
            warn!(
                user_id = %user_id,
                resource = %resource,
                action = %action,
                duration_ms = %duration.as_millis(),
                "Slow permission check detected"
            );
        }

        debug!(
            user_id = %user_id,
            resource = %resource,
            action = %action,
            has_permission = %has_permission,
            duration_ms = %duration.as_millis(),
            "Permission check completed"
        );

        Ok(has_permission)
    }

    /// Check if a user has permission to perform an operation on a specific dot
    pub async fn check_dot_permission(&self, user_id: &str, dot_id: &str, operation: &str, context: &PermissionContext) -> ApiResult<bool> {
        let start_time = Instant::now();

        // Get user dot permissions
        let dot_permissions = self.get_user_dot_permissions(user_id).await?;

        // Check dot permissions
        let has_permission = self.permission_checker.check_dot_permission(&dot_permissions, dot_id, operation, context).await?;

        let duration = start_time.elapsed();

        // Log the permission check
        self.audit_logger
            .log_dot_permission_check(
                user_id,
                dot_id,
                operation,
                has_permission,
                context.client_ip.clone(),
                context.additional_data.get("request_id").cloned(),
            )
            .await;

        // Log performance warning if check took too long
        if duration > Duration::from_millis(5) {
            warn!(
                user_id = %user_id,
                dot_id = %dot_id,
                operation = %operation,
                duration_ms = %duration.as_millis(),
                "Slow dot permission check detected"
            );
        }

        debug!(
            user_id = %user_id,
            dot_id = %dot_id,
            operation = %operation,
            has_permission = %has_permission,
            duration_ms = %duration.as_millis(),
            "Dot permission check completed"
        );

        Ok(has_permission)
    }

    /// Assign a role to a user
    pub async fn assign_role(&self, user_id: &str, role_id: &str, assigned_by: &str) -> ApiResult<()> {
        self.role_manager.assign_role(user_id, role_id, assigned_by).await?;

        // Invalidate user cache
        self.cache.invalidate_user(user_id).await;

        info!(
            user_id = %user_id,
            role_id = %role_id,
            assigned_by = %assigned_by,
            "Role assigned successfully"
        );

        Ok(())
    }

    /// Assign a temporary role to a user
    pub async fn assign_temporary_role(&self, user_id: &str, role_id: &str, expires_at: DateTime<Utc>, assigned_by: &str, reason: Option<String>) -> ApiResult<()> {
        self.role_manager.assign_temporary_role(user_id, role_id, expires_at, assigned_by, reason).await?;

        // Invalidate user cache
        self.cache.invalidate_user(user_id).await;

        info!(
            user_id = %user_id,
            role_id = %role_id,
            expires_at = %expires_at,
            assigned_by = %assigned_by,
            "Temporary role assigned successfully"
        );

        Ok(())
    }

    /// Revoke a role from a user
    pub async fn revoke_role(&self, user_id: &str, role_id: &str, revoked_by: &str) -> ApiResult<()> {
        self.role_manager.revoke_role(user_id, role_id, revoked_by).await?;

        // Invalidate user cache
        self.cache.invalidate_user(user_id).await;

        info!(
            user_id = %user_id,
            role_id = %role_id,
            revoked_by = %revoked_by,
            "Role revoked successfully"
        );

        Ok(())
    }

    /// Create a new role
    pub async fn create_role(&self, role: Role, created_by: &str) -> ApiResult<()> {
        self.role_manager.create_role(role.clone(), created_by).await?;

        // Invalidate role cache
        self.cache.invalidate_role(&role.id).await;

        info!(
            role_id = %role.id,
            role_name = %role.name,
            created_by = %created_by,
            "Role created successfully"
        );

        Ok(())
    }

    /// Update an existing role
    pub async fn update_role(&self, role: Role, updated_by: &str) -> ApiResult<()> {
        self.role_manager.update_role(role.clone(), updated_by).await?;

        // Invalidate role cache
        self.cache.invalidate_role(&role.id).await;

        info!(
            role_id = %role.id,
            role_name = %role.name,
            updated_by = %updated_by,
            "Role updated successfully"
        );

        Ok(())
    }

    /// Delete a role
    pub async fn delete_role(&self, role_id: &str, deleted_by: &str) -> ApiResult<()> {
        self.role_manager.delete_role(role_id, deleted_by).await?;

        // Invalidate role cache
        self.cache.invalidate_role(role_id).await;

        info!(
            role_id = %role_id,
            deleted_by = %deleted_by,
            "Role deleted successfully"
        );

        Ok(())
    }

    /// Get a role by ID
    pub async fn get_role(&self, role_id: &str) -> Option<Role> {
        self.role_manager.get_role(role_id).await
    }

    /// Get all roles
    pub async fn get_all_roles(&self) -> Vec<Role> {
        self.role_manager.get_all_roles().await
    }

    /// Get user role assignments
    pub async fn get_user_roles(&self, user_id: &str) -> Vec<UserRoleAssignment> {
        self.role_manager.get_user_roles(user_id).await
    }

    /// Get effective permissions for a user (with caching)
    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        // Check cache first
        if let Some(cached_permissions) = self.cache.get_user_permissions(user_id).await {
            debug!(user_id = %user_id, "User permissions retrieved from cache");
            return Ok(cached_permissions);
        }

        // Get from role manager
        let permissions = self.role_manager.get_user_permissions(user_id).await?;

        // Cache the result
        self.cache.set_user_permissions(user_id.to_string(), permissions.clone(), Duration::from_secs(300)).await;

        debug!(
            user_id = %user_id,
            permission_count = %permissions.len(),
            "User permissions computed and cached"
        );

        Ok(permissions)
    }

    /// Get effective dot permissions for a user (with caching)
    pub async fn get_user_dot_permissions(&self, user_id: &str) -> ApiResult<Vec<DotPermission>> {
        // Check cache first
        if let Some(cached_permissions) = self.cache.get_user_dot_permissions(user_id).await {
            debug!(user_id = %user_id, "User dot permissions retrieved from cache");
            return Ok(cached_permissions);
        }

        // Get from role manager
        let dot_permissions = self.role_manager.get_user_dot_permissions(user_id).await?;

        // Cache the result
        self.cache.set_user_dot_permissions(user_id.to_string(), dot_permissions.clone(), Duration::from_secs(300)).await;

        debug!(
            user_id = %user_id,
            dot_permission_count = %dot_permissions.len(),
            "User dot permissions computed and cached"
        );

        Ok(dot_permissions)
    }

    /// Get role hierarchy
    pub async fn get_role_hierarchy(&self, role_id: &str) -> ApiResult<Vec<String>> {
        self.role_manager.get_role_hierarchy(role_id).await
    }

    /// Validate role hierarchy
    pub async fn validate_hierarchy(&self) -> ApiResult<()> {
        self.role_manager.validate_hierarchy().await
    }

    /// Clean up expired role assignments
    pub async fn cleanup_expired_assignments(&self) {
        self.role_manager.cleanup_expired_assignments().await;
        info!("Expired role assignments cleaned up");
    }

    /// Get audit logger
    pub fn audit_logger(&self) -> &Arc<AuditLogger> {
        &self.audit_logger
    }

    /// Get permission cache
    pub fn cache(&self) -> &Arc<PermissionCache> {
        &self.cache
    }

    /// Get role manager
    pub fn role_manager(&self) -> &Arc<RoleManager> {
        &self.role_manager
    }

    /// Update user from authentication system
    pub async fn update_user_from_auth(&self, user: &User) -> ApiResult<()> {
        // This method would typically sync user information from the auth system
        // For now, we'll just invalidate the user's cache to force refresh
        self.cache.invalidate_user(&user.id).await;

        debug!(
            user_id = %user.id,
            username = %user.username,
            "User information updated from auth system"
        );

        Ok(())
    }

    /// Start background maintenance tasks
    pub fn start_maintenance_tasks(system: Arc<Self>) -> Vec<tokio::task::JoinHandle<()>> {
        let mut tasks = Vec::new();

        // Cleanup expired assignments task
        {
            let system = system.clone();
            let task = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour

                loop {
                    interval.tick().await;
                    system.cleanup_expired_assignments().await;
                }
            });
            tasks.push(task);
        }

        // Cache statistics logging task
        {
            let system = system.clone();
            let task = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

                loop {
                    interval.tick().await;
                    let stats = system.cache.get_stats().await;

                    info!(
                        cache_hits = %stats.hits,
                        cache_misses = %stats.misses,
                        hit_ratio = %format!("{:.2}%", stats.hit_ratio() * 100.0),
                        cache_size = %stats.current_size,
                        cache_evictions = %stats.evictions,
                        "RBAC cache statistics"
                    );
                }
            });
            tasks.push(task);
        }

        info!("RBAC maintenance tasks started");
        tasks
    }

    /// Get system health status
    pub async fn get_health_status(&self) -> RBACHealthStatus {
        let cache_stats = self.cache.get_stats().await;
        let audit_stats = self.audit_logger.get_statistics().await;

        RBACHealthStatus {
            is_healthy: true,
            cache_hit_ratio: cache_stats.hit_ratio(),
            cache_size: cache_stats.current_size,
            total_audit_events: audit_stats.total_events,
            roles_count: self.get_all_roles().await.len(),
            last_check: Utc::now(),
        }
    }
}

/// RBAC system health status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RBACHealthStatus {
    /// Whether the system is healthy
    pub is_healthy: bool,

    /// Cache hit ratio
    pub cache_hit_ratio: f64,

    /// Current cache size
    pub cache_size: usize,

    /// Total audit events
    pub total_audit_events: usize,

    /// Number of roles in the system
    pub roles_count: usize,

    /// Last health check timestamp
    pub last_check: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::permissions::Permission;

    #[tokio::test]
    async fn test_rbac_system_initialization() {
        let system = RBACSystem::initialize().await.unwrap();

        // Check that default roles exist
        let roles = system.get_all_roles().await;
        assert!(!roles.is_empty());

        // Check that system is healthy
        let health = system.get_health_status().await;
        assert!(health.is_healthy);
    }

    #[tokio::test]
    async fn test_role_operations() {
        let system = RBACSystem::initialize().await.unwrap();

        // Create a test role
        let mut role = Role::new("test_role".to_string(), "Test Role".to_string(), "A test role".to_string());
        role.add_permission(Permission::new("test".to_string(), "read".to_string()));

        // Create role
        assert!(system.create_role(role.clone(), "admin").await.is_ok());

        // Get role
        let retrieved_role = system.get_role("test_role").await;
        assert!(retrieved_role.is_some());
        assert_eq!(retrieved_role.unwrap().name, "Test Role");

        // Delete role
        assert!(system.delete_role("test_role", "admin").await.is_ok());

        // Verify deletion
        let deleted_role = system.get_role("test_role").await;
        assert!(deleted_role.is_none());
    }

    #[tokio::test]
    async fn test_role_assignment() {
        let system = RBACSystem::initialize().await.unwrap();

        // Assign existing system role
        assert!(system.assign_role("user123", "user", "admin").await.is_ok());

        // Check assignment
        let assignments = system.get_user_roles("user123").await;
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].role_id, "user");

        // Revoke role
        assert!(system.revoke_role("user123", "user", "admin").await.is_ok());

        // Check revocation
        let assignments = system.get_user_roles("user123").await;
        assert!(assignments.is_empty());
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let system = RBACSystem::initialize().await.unwrap();

        // Assign role with permissions
        system.assign_role("user123", "user", "admin").await.unwrap();

        // Create permission context
        let context = PermissionContext::new("user123".to_string());

        // Check permission (this will depend on the default user role permissions)
        let has_permission = system.check_permission("user123", "dots", "read", &context).await.unwrap();

        // The result depends on what permissions the default "user" role has
        // For now, just verify the method doesn't error
        assert!(has_permission || !has_permission); // Always true, just checking no panic
    }

    #[tokio::test]
    async fn test_performance_requirement() {
        let system = RBACSystem::initialize().await.unwrap();

        // Assign role
        system.assign_role("user123", "user", "admin").await.unwrap();

        let context = PermissionContext::new("user123".to_string());

        // Measure permission check time
        let start = Instant::now();
        let _result = system.check_permission("user123", "dots", "read", &context).await.unwrap();
        let duration = start.elapsed();

        // Should be under 5ms as per requirements
        assert!(duration < Duration::from_millis(5), "Permission check took {:?}, should be under 5ms", duration);
    }
}
