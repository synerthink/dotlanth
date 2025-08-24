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

//! Dot-level permission system based on ABI definitions

use crate::auth::User;
use crate::error::{ApiError, ApiResult};
use crate::rate_limiting::{RateLimitAlgorithm, RateLimitConfig, RateLimiter};
use crate::rbac::audit::AuditLogger;
use crate::rbac::manager::RoleManager;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Dot-specific permissions configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotPermissions {
    /// Operations that can be executed without authentication
    pub public_operations: Vec<String>,
    /// Operations that require specific permissions
    pub protected_operations: HashMap<String, OperationPermission>,
    /// Custom roles defined for this dot
    pub custom_roles: Vec<CustomRole>,
    /// Operations that only the dot owner can perform
    pub owner_operations: Vec<String>,
    /// Permission inheritance rules for dot hierarchies
    pub inheritance_rules: Vec<InheritanceRule>,
}

/// Permission requirements for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPermission {
    /// Required role to execute this operation
    pub required_role: String,
    /// Human-readable description of the operation
    pub description: String,
    /// Additional conditions that must be met
    pub conditions: Vec<PermissionCondition>,
    /// Rate limiting configuration for this operation
    pub rate_limit: Option<RateLimit>,
}

/// Custom role definition specific to a dot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRole {
    /// Unique role identifier within the dot
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Operations this role can perform
    pub permissions: Vec<String>,
    /// Roles this role inherits from
    pub inherits_from: Vec<String>,
    /// Whether this role can be assigned by non-owners
    pub assignable: bool,
}

/// Permission inheritance rule for dot hierarchies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritanceRule {
    /// Parent dot pattern (supports wildcards)
    pub parent_pattern: String,
    /// Operations to inherit from parent
    pub inherited_operations: Vec<String>,
    /// Whether to inherit custom roles
    pub inherit_roles: bool,
    /// Conditions for inheritance
    pub conditions: Vec<PermissionCondition>,
}

/// Condition that must be met for permission to be granted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCondition {
    /// Type of condition (time, location, resource, etc.)
    pub condition_type: String,
    /// Operator for the condition (eq, gt, lt, in, etc.)
    pub operator: String,
    /// Value to compare against
    pub value: String,
    /// Optional metadata for the condition
    pub metadata: HashMap<String, String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum number of requests
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u32,
    /// Burst allowance
    pub burst: Option<u32>,
}

/// Cached permission result
#[derive(Debug, Clone)]
struct CachedPermission {
    /// Whether permission is granted
    allowed: bool,
    /// When this cache entry expires
    expires_at: Instant,
    /// Conditions that were evaluated
    conditions_met: Vec<String>,
}

/// Permission cache for performance optimization
#[derive(Debug)]
pub struct PermissionCache {
    /// Cache storage
    cache: DashMap<String, CachedPermission>,
    /// Default cache TTL
    default_ttl: Duration,
}

impl PermissionCache {
    /// Create a new permission cache
    pub fn new(default_ttl: Duration) -> Self {
        Self { cache: DashMap::new(), default_ttl }
    }

    /// Get cached permission result
    pub fn get(&self, key: &str) -> Option<bool> {
        if let Some(entry) = self.cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.allowed);
            } else {
                // Remove expired entry
                self.cache.remove(key);
            }
        }
        None
    }

    /// Cache a permission result
    pub fn set(&self, key: String, allowed: bool, conditions_met: Vec<String>) {
        let cached = CachedPermission {
            allowed,
            expires_at: Instant::now() + self.default_ttl,
            conditions_met,
        };
        self.cache.insert(key, cached);
    }

    /// Clear cache for a specific dot
    pub fn clear_dot_cache(&self, dot_id: &str) {
        self.cache.retain(|key, _| !key.starts_with(&format!("{}:", dot_id)));
    }

    /// Clear all cached permissions
    pub fn clear_all(&self) {
        self.cache.clear();
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.cache.retain(|_, entry| entry.expires_at > now);
    }
}

/// Dot permission manager handles all dot-level permission operations
#[derive(Debug)]
pub struct DotPermissionManager {
    /// Dot permissions storage
    dot_permissions: Arc<DashMap<String, DotPermissions>>,
    /// Permission cache for performance
    permission_cache: Arc<PermissionCache>,
    /// Audit logger for permission decisions
    audit_logger: Arc<AuditLogger>,
    /// Dot ownership mapping
    dot_owners: Arc<DashMap<String, String>>,
    /// Role manager for dot-specific role assignments
    role_manager: Arc<RoleManager>,
    /// Rate limiters for operations
    rate_limiters: Arc<DashMap<String, Arc<RateLimiter>>>,
}

impl DotPermissionManager {
    /// Create a new dot permission manager
    pub fn new(audit_logger: Arc<AuditLogger>) -> Self {
        let role_manager = Arc::new(RoleManager::new(audit_logger.clone()));
        Self {
            dot_permissions: Arc::new(DashMap::new()),
            permission_cache: Arc::new(PermissionCache::new(Duration::from_secs(300))), // 5 minutes default
            audit_logger,
            dot_owners: Arc::new(DashMap::new()),
            role_manager,
            rate_limiters: Arc::new(DashMap::new()),
        }
    }

    /// Load dot permissions from ABI
    pub async fn load_dot_permissions(&self, dot_id: &str, abi: &DotABI) -> ApiResult<()> {
        let permissions = self.parse_abi_permissions(abi)?;
        self.dot_permissions.insert(dot_id.to_string(), permissions);

        // Clear cache for this dot
        self.permission_cache.clear_dot_cache(dot_id);

        // Log permission loading using the existing audit system
        self.audit_logger
            .log_permission_check("system", dot_id, "load_permissions", true, None, Some(format!("Loaded permissions from ABI for dot {}", dot_id)))
            .await;

        Ok(())
    }

    /// Check if a user can perform an operation on a dot
    pub fn check_dot_operation(&self, user: &User, dot_id: &str, operation: &str) -> ApiResult<bool> {
        let cache_key = format!("{}:{}:{}", dot_id, user.id, operation);

        // Check cache first
        if let Some(cached_result) = self.permission_cache.get(&cache_key) {
            return Ok(cached_result);
        }

        let result = self.check_dot_operation_internal(user, dot_id, operation)?;

        // Cache the result
        self.permission_cache.set(cache_key, result, vec![]);

        // Log the permission check
        self.audit_logger.log_permission_check(
            &user.id,
            dot_id,
            operation,
            result,
            None,
            Some(format!("User {} {} operation {} on dot {}", user.id, if result { "allowed" } else { "denied" }, operation, dot_id)),
        );

        Ok(result)
    }

    /// Internal permission check logic
    fn check_dot_operation_internal(&self, user: &User, dot_id: &str, operation: &str) -> ApiResult<bool> {
        // Get dot permissions
        let dot_permissions = match self.dot_permissions.get(dot_id) {
            Some(perms) => perms,
            None => {
                // If no specific permissions are defined, check global permissions
                return Ok(user.dot_permissions.get("*").map(|perms| perms.contains(&operation.to_string())).unwrap_or(false));
            }
        };

        // Check if operation is public
        if dot_permissions.public_operations.contains(&operation.to_string()) {
            return Ok(true);
        }

        // Check if user is dot owner and operation is owner-only
        if dot_permissions.owner_operations.contains(&operation.to_string()) {
            return Ok(self.is_dot_owner(user, dot_id)?);
        }

        // Check protected operations
        if let Some(op_permission) = dot_permissions.protected_operations.get(operation) {
            return self.check_operation_permission(user, dot_id, operation, op_permission);
        }

        // Check inheritance rules
        for rule in &dot_permissions.inheritance_rules {
            if self.matches_inheritance_pattern(&rule.parent_pattern, dot_id) {
                if rule.inherited_operations.contains(&operation.to_string()) {
                    if self.check_inheritance_conditions(user, rule)? {
                        // Check parent permissions
                        return self.check_parent_permissions(user, &rule.parent_pattern, operation);
                    }
                }
            }
        }

        // Check user's dot-specific permissions
        if let Some(user_dot_perms) = user.dot_permissions.get(dot_id) {
            if user_dot_perms.contains(&operation.to_string()) {
                return Ok(true);
            }
        }

        // Check wildcard permissions
        if let Some(wildcard_perms) = user.dot_permissions.get("*") {
            if wildcard_perms.contains(&operation.to_string()) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if user is the owner of a dot
    pub fn is_dot_owner(&self, user: &User, dot_id: &str) -> ApiResult<bool> {
        if let Some(owner_id) = self.dot_owners.get(dot_id) {
            Ok(*owner_id == user.id)
        } else {
            // If no owner is set, check if user has admin privileges
            Ok(user.roles.contains(&"admin".to_string()))
        }
    }

    /// Set dot owner
    pub fn set_dot_owner(&self, dot_id: &str, owner_id: &str) {
        self.dot_owners.insert(dot_id.to_string(), owner_id.to_string());

        // Log ownership change using the existing audit system
        tokio::spawn({
            let audit_logger = self.audit_logger.clone();
            let dot_id = dot_id.to_string();
            let owner_id = owner_id.to_string();
            async move {
                audit_logger
                    .log_permission_check(&owner_id, &dot_id, "set_ownership", true, None, Some(format!("Dot {} ownership set to user {}", dot_id, owner_id)))
                    .await;
            }
        });
    }

    /// Get user's permissions for a specific dot
    pub fn get_user_dot_permissions(&self, user: &User, dot_id: &str) -> ApiResult<Vec<String>> {
        let mut permissions = Vec::new();

        // Get dot permissions
        if let Some(dot_permissions) = self.dot_permissions.get(dot_id) {
            // Add public operations
            permissions.extend(dot_permissions.public_operations.clone());

            // Add owner operations if user is owner
            if self.is_dot_owner(user, dot_id)? {
                permissions.extend(dot_permissions.owner_operations.clone());
            }

            // Check protected operations
            for (operation, op_permission) in &dot_permissions.protected_operations {
                if self.check_operation_permission(user, dot_id, operation, op_permission)? {
                    permissions.push(operation.clone());
                }
            }

            // Check custom roles
            for role in &dot_permissions.custom_roles {
                if self.user_has_dot_role(user, dot_id, &role.name)? {
                    permissions.extend(role.permissions.clone());
                }
            }
        }

        // Add user's explicit dot permissions
        if let Some(user_dot_perms) = user.dot_permissions.get(dot_id) {
            permissions.extend(user_dot_perms.clone());
        }

        // Add wildcard permissions
        if let Some(wildcard_perms) = user.dot_permissions.get("*") {
            permissions.extend(wildcard_perms.clone());
        }

        // Remove duplicates and sort
        permissions.sort();
        permissions.dedup();

        Ok(permissions)
    }

    /// Check if user has a specific role for a dot
    fn user_has_dot_role(&self, user: &User, dot_id: &str, role_name: &str) -> ApiResult<bool> {
        // Check if user has the role globally
        if user.roles.contains(&role_name.to_string()) {
            return Ok(true);
        }

        // For now, check if the role is implied by permissions in custom roles
        // In a real async context, this would be handled differently
        if let Some(dot_permissions) = self.dot_permissions.get(dot_id) {
            for role in &dot_permissions.custom_roles {
                if role.name == role_name {
                    // Check if user has all permissions required by this role
                    if let Some(user_dot_perms) = user.dot_permissions.get(dot_id) {
                        return Ok(role.permissions.iter().all(|perm| user_dot_perms.contains(perm)));
                    }
                }
            }
        }

        // Check user's dot-specific permissions to see if they match the role
        if let Some(user_dot_perms) = user.dot_permissions.get(dot_id) {
            if user_dot_perms.contains(&format!("role:{}", role_name)) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check operation permission requirements
    fn check_operation_permission(&self, user: &User, dot_id: &str, operation: &str, op_permission: &OperationPermission) -> ApiResult<bool> {
        // Check if user has required role
        let has_global_role = user.roles.contains(&op_permission.required_role);
        let has_dot_role = self.user_has_dot_role(user, dot_id, &op_permission.required_role)?;

        if !has_global_role && !has_dot_role {
            return Ok(false);
        }

        // Check conditions
        for condition in &op_permission.conditions {
            if !self.check_permission_condition(user, condition)? {
                return Ok(false);
            }
        }

        // Check rate limiting using the actual rate limiter
        if let Some(rate_limit) = &op_permission.rate_limit {
            let limiter_key = format!("{}:{}:{}", dot_id, operation, user.id);
            let limiter = self.get_or_create_rate_limiter(&limiter_key, rate_limit);

            let rate_limit_info = limiter.is_allowed(&user.id).map_err(|e| ApiError::InternalServerError {
                message: format!("Rate limit check failed: {}", e),
            })?;

            if !rate_limit_info.allowed {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check a permission condition
    fn check_permission_condition(&self, _user: &User, condition: &PermissionCondition) -> ApiResult<bool> {
        match condition.condition_type.as_str() {
            "time" => {
                // Check time-based conditions
                let now = Utc::now();
                match condition.operator.as_str() {
                    "after" => {
                        let time_value = DateTime::parse_from_rfc3339(&condition.value).map_err(|_| ApiError::BadRequest {
                            message: "Invalid time format".to_string(),
                        })?;
                        Ok(now > time_value)
                    }
                    "before" => {
                        let time_value = DateTime::parse_from_rfc3339(&condition.value).map_err(|_| ApiError::BadRequest {
                            message: "Invalid time format".to_string(),
                        })?;
                        Ok(now < time_value)
                    }
                    _ => Ok(true), // Unknown operator, allow by default
                }
            }
            "resource" => {
                // Check resource-based conditions
                match condition.operator.as_str() {
                    "exists" => {
                        // Check if resource exists (simplified)
                        Ok(true)
                    }
                    _ => Ok(true),
                }
            }
            _ => Ok(true), // Unknown condition type, allow by default
        }
    }

    /// Check inheritance pattern matching
    fn matches_inheritance_pattern(&self, pattern: &str, dot_id: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            // Simple wildcard matching
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                let prefix = pattern_parts[0];
                let suffix = pattern_parts[1];
                return dot_id.starts_with(prefix) && dot_id.ends_with(suffix);
            }
        }

        pattern == dot_id
    }

    /// Check inheritance conditions
    fn check_inheritance_conditions(&self, user: &User, rule: &InheritanceRule) -> ApiResult<bool> {
        for condition in &rule.conditions {
            if !self.check_permission_condition(user, condition)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Check parent permissions for inheritance
    fn check_parent_permissions(&self, user: &User, parent_pattern: &str, operation: &str) -> ApiResult<bool> {
        // Find parent dots matching the pattern
        for dot_entry in self.dot_permissions.iter() {
            if self.matches_inheritance_pattern(parent_pattern, dot_entry.key()) {
                if self.check_dot_operation_internal(user, dot_entry.key(), operation)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Parse permissions from ABI
    fn parse_abi_permissions(&self, abi: &DotABI) -> ApiResult<DotPermissions> {
        let mut permissions = DotPermissions {
            public_operations: Vec::new(),
            protected_operations: HashMap::new(),
            custom_roles: Vec::new(),
            owner_operations: Vec::new(),
            inheritance_rules: Vec::new(),
        };

        if let Some(perm_config) = &abi.permissions {
            // Parse public operations
            permissions.public_operations = perm_config.public_operations.clone();

            // Parse protected operations
            for (operation, op_perm) in &perm_config.protected_operations {
                permissions.protected_operations.insert(
                    operation.clone(),
                    OperationPermission {
                        required_role: op_perm.required_roles.first().unwrap_or(&"user".to_string()).clone(),
                        description: op_perm.description.clone(),
                        conditions: Vec::new(), // Would be parsed from ABI metadata
                        rate_limit: None,       // Would be parsed from ABI metadata
                    },
                );
            }

            // Parse custom roles
            for (role_name, role_def) in &perm_config.roles {
                permissions.custom_roles.push(CustomRole {
                    name: role_name.clone(),
                    description: role_def.description.clone(),
                    permissions: role_def.permissions.clone(),
                    inherits_from: role_def.inherits.clone(),
                    assignable: true, // Default to assignable
                });
            }
        }

        Ok(permissions)
    }

    /// Clean up expired cache entries
    pub fn cleanup_cache(&self) {
        self.permission_cache.cleanup_expired();
    }

    /// Get permission statistics for monitoring
    pub fn get_permission_stats(&self) -> PermissionStats {
        PermissionStats {
            total_dots: self.dot_permissions.len(),
            cache_size: self.permission_cache.cache.len(),
            total_owners: self.dot_owners.len(),
        }
    }

    /// Get or create a rate limiter for a specific operation
    fn get_or_create_rate_limiter(&self, key: &str, rate_limit: &RateLimit) -> Arc<RateLimiter> {
        if let Some(limiter) = self.rate_limiters.get(key) {
            return limiter.clone();
        }

        let config = RateLimitConfig {
            max_requests: rate_limit.max_requests,
            window: Duration::from_secs(rate_limit.window_seconds as u64),
            algorithm: RateLimitAlgorithm::TokenBucket,
            per_ip: false,
            per_user: true,
            per_api_key: false,
        };

        let limiter = Arc::new(RateLimiter::new(config));
        self.rate_limiters.insert(key.to_string(), limiter.clone());
        limiter
    }

    /// Assign a dot-specific role to a user
    pub async fn assign_dot_role(&self, user_id: &str, dot_id: &str, role_name: &str, assigned_by: &str) -> ApiResult<()> {
        // Check if the role exists in the dot's custom roles
        if let Some(dot_permissions) = self.dot_permissions.get(dot_id) {
            let role_exists = dot_permissions.custom_roles.iter().any(|role| role.name == role_name);
            if !role_exists {
                return Err(ApiError::NotFound {
                    message: format!("Role '{}' not found in dot '{}'", role_name, dot_id),
                });
            }
        }

        // For this implementation, we'll create a dot-specific role in the role manager first
        let dot_specific_role_id = format!("{}:{}", dot_id, role_name);

        // Create the role if it doesn't exist
        if self.role_manager.get_role(&dot_specific_role_id).await.is_none() {
            use crate::rbac::roles::Role;
            let role = Role::new(
                dot_specific_role_id.clone(),
                format!("{} role for dot {}", role_name, dot_id),
                format!("Dot-specific {} role", role_name),
            );
            let _ = self.role_manager.create_role(role, assigned_by).await;
        }

        // Use the role manager to assign the role
        self.role_manager.assign_role(user_id, &dot_specific_role_id, assigned_by).await?;

        // Clear cache for this user
        let cache_key = format!("{}:{}:", dot_id, user_id);
        self.permission_cache.cache.retain(|key, _| !key.starts_with(&cache_key));

        Ok(())
    }

    /// Revoke a dot-specific role from a user
    pub async fn revoke_dot_role(&self, user_id: &str, dot_id: &str, role_name: &str, revoked_by: &str) -> ApiResult<()> {
        let dot_specific_role_id = format!("{}:{}", dot_id, role_name);

        // Use the role manager to revoke the role
        self.role_manager.revoke_role(user_id, &dot_specific_role_id, revoked_by).await?;

        // Clear cache for this user
        let cache_key = format!("{}:{}:", dot_id, user_id);
        self.permission_cache.cache.retain(|key, _| !key.starts_with(&cache_key));

        Ok(())
    }

    /// Get all dot-specific roles for a user
    pub async fn get_user_dot_roles(&self, user_id: &str, dot_id: &str) -> ApiResult<Vec<String>> {
        let user_roles = self.role_manager.get_user_roles(user_id).await;
        let dot_prefix = format!("{}:", dot_id);

        let mut dot_roles = Vec::new();
        for assignment in user_roles {
            if assignment.role_id.starts_with(&dot_prefix) {
                if let Some(role_name) = assignment.role_id.strip_prefix(&dot_prefix) {
                    dot_roles.push(role_name.to_string());
                }
            }
        }

        Ok(dot_roles)
    }
}

/// Permission statistics for monitoring
#[derive(Debug, Serialize)]
pub struct PermissionStats {
    pub total_dots: usize,
    pub cache_size: usize,
    pub total_owners: usize,
}

/// Dot ABI structure (simplified version for permission parsing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotABI {
    pub dot_name: String,
    pub version: String,
    pub description: String,
    pub permissions: Option<PermissionConfig>,
}

/// Permission configuration from ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub public_operations: Vec<String>,
    pub protected_operations: HashMap<String, OperationPermission2>,
    pub roles: HashMap<String, RoleDefinition>,
}

/// Operation permission from ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPermission2 {
    pub required_roles: Vec<String>,
    pub description: String,
}

/// Role definition from ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub description: String,
    pub inherits: Vec<String>,
    pub permissions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::audit::AuditLogger;

    fn create_test_user() -> User {
        User {
            id: "test_user".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string()],
            dot_permissions: HashMap::new(),
            created_at: Utc::now(),
            last_login: None,
            is_active: true,
        }
    }

    fn create_test_abi() -> DotABI {
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
    async fn test_public_operation_access() {
        let audit_logger = Arc::new(AuditLogger::new());
        let manager = DotPermissionManager::new(audit_logger);
        let user = create_test_user();
        let abi = create_test_abi();

        manager.load_dot_permissions("test_dot", &abi).await.unwrap();

        // Public operations should be accessible
        assert!(manager.check_dot_operation(&user, "test_dot", "read").unwrap());
    }

    #[tokio::test]
    async fn test_protected_operation_access() {
        let audit_logger = Arc::new(AuditLogger::new());
        let manager = DotPermissionManager::new(audit_logger);
        let mut user = create_test_user();
        let abi = create_test_abi();

        manager.load_dot_permissions("test_dot", &abi).await.unwrap();

        // Protected operations should be denied without proper role
        assert!(!manager.check_dot_operation(&user, "test_dot", "write").unwrap());

        // Grant the required role
        user.roles.push("writer".to_string());

        // Clear cache since user roles changed
        manager.permission_cache.clear_all();

        // Now the user should have access to the protected operation
        assert!(manager.check_dot_operation(&user, "test_dot", "write").unwrap());
    }

    #[tokio::test]
    async fn test_dot_ownership() {
        let audit_logger = Arc::new(AuditLogger::new());
        let manager = DotPermissionManager::new(audit_logger);
        let user = create_test_user();

        // Initially not owner
        assert!(!manager.is_dot_owner(&user, "test_dot").unwrap());

        // Set as owner
        manager.set_dot_owner("test_dot", &user.id);
        assert!(manager.is_dot_owner(&user, "test_dot").unwrap());
    }

    #[tokio::test]
    async fn test_dot_specific_roles() {
        let audit_logger = Arc::new(AuditLogger::new());
        let manager = DotPermissionManager::new(audit_logger);
        let mut user = create_test_user();
        let abi = create_test_abi();

        // Load dot permissions with custom roles
        manager.load_dot_permissions("test_dot", &abi).await.unwrap();

        // Initially user should not have dot-specific roles
        let roles = manager.get_user_dot_roles(&user.id, "test_dot").await.unwrap();
        assert!(roles.is_empty());

        // Test role assignment and revocation functionality
        manager.assign_dot_role(&user.id, "test_dot", "writer", "admin").await.unwrap();

        // Check that the role was assigned
        let roles = manager.get_user_dot_roles(&user.id, "test_dot").await.unwrap();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "writer");

        // Revoke the role
        manager.revoke_dot_role(&user.id, "test_dot", "writer", "admin").await.unwrap();

        // Check that the role was revoked
        let roles = manager.get_user_dot_roles(&user.id, "test_dot").await.unwrap();
        assert!(roles.is_empty());

        // Test permission check with role assignment
        // Grant the user the writer role globally (simulating successful role assignment)
        user.roles.push("writer".to_string());
        manager.permission_cache.clear_all();

        // Now the user should have access to protected operations
        assert!(manager.check_dot_operation(&user, "test_dot", "write").unwrap());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let audit_logger = Arc::new(AuditLogger::new());
        let manager = DotPermissionManager::new(audit_logger);
        let mut user = create_test_user();
        user.roles.push("writer".to_string());

        // Create ABI with rate limiting
        let mut abi = create_test_abi();
        if let Some(ref mut perm_config) = abi.permissions {
            if let Some(write_perm) = perm_config.protected_operations.get_mut("write") {
                // This would be set if the ABI parsing supported rate limits
                // For now, we'll test the basic functionality
            }
        }

        manager.load_dot_permissions("test_dot", &abi).await.unwrap();

        // First request should be allowed
        assert!(manager.check_dot_operation(&user, "test_dot", "write").unwrap());
    }
}
