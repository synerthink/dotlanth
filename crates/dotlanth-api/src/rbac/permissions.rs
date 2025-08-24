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

//! Permission definitions and management

use crate::error::{ApiError, ApiResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Permission structure defining access to resources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    /// Resource identifier (e.g., "dots", "collections", "users")
    pub resource: String,

    /// Action identifier (e.g., "read", "write", "delete", "execute")
    pub action: String,

    /// Conditions that must be met for this permission to apply
    pub conditions: Vec<PermissionCondition>,
}

impl Permission {
    /// Create a new permission
    pub fn new(resource: String, action: String) -> Self {
        Self {
            resource,
            action,
            conditions: Vec::new(),
        }
    }

    /// Create a permission with conditions
    pub fn with_conditions(resource: String, action: String, conditions: Vec<PermissionCondition>) -> Self {
        Self { resource, action, conditions }
    }

    /// Get the permission key for caching and comparison
    pub fn key(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }

    /// Check if this permission matches a resource and action
    pub fn matches(&self, resource: &str, action: &str) -> bool {
        (self.resource == "*" || self.resource == resource) && (self.action == "*" || self.action == action)
    }

    /// Evaluate conditions for this permission
    pub fn evaluate_conditions(&self, context: &PermissionContext) -> bool {
        if self.conditions.is_empty() {
            return true;
        }

        self.conditions.iter().all(|condition| condition.evaluate(context))
    }
}

/// Dot-specific permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DotPermission {
    /// Dot identifier
    pub dot_id: String,

    /// Allowed operations on this dot
    pub operations: Vec<String>,

    /// Conditions that must be met
    pub conditions: Vec<PermissionCondition>,
}

impl DotPermission {
    /// Create a new dot permission
    pub fn new(dot_id: String, operations: Vec<String>) -> Self {
        Self {
            dot_id,
            operations,
            conditions: Vec::new(),
        }
    }

    /// Create dot permission with conditions
    pub fn with_conditions(dot_id: String, operations: Vec<String>, conditions: Vec<PermissionCondition>) -> Self {
        Self { dot_id, operations, conditions }
    }

    /// Check if this permission allows an operation on the dot
    pub fn allows_operation(&self, dot_id: &str, operation: &str, context: &PermissionContext) -> bool {
        if self.dot_id != "*" && self.dot_id != dot_id {
            return false;
        }

        if !self.operations.contains(&operation.to_string()) && !self.operations.contains(&"*".to_string()) {
            return false;
        }

        self.evaluate_conditions(context)
    }

    /// Evaluate conditions for this dot permission
    pub fn evaluate_conditions(&self, context: &PermissionContext) -> bool {
        if self.conditions.is_empty() {
            return true;
        }

        self.conditions.iter().all(|condition| condition.evaluate(context))
    }
}

/// Permission condition that must be evaluated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionCondition {
    /// User must own the resource
    Ownership {
        /// Field name to check for ownership
        owner_field: String,
    },

    /// Time-based condition
    TimeWindow {
        /// Start time (optional)
        start: Option<DateTime<Utc>>,
        /// End time (optional)
        end: Option<DateTime<Utc>>,
    },

    /// IP address restriction
    IpRestriction {
        /// Allowed IP addresses or CIDR blocks
        allowed_ips: Vec<String>,
    },

    /// Custom condition with key-value pairs
    Custom {
        /// Condition type
        condition_type: String,
        /// Parameters for the condition
        parameters: HashMap<String, String>,
    },
}

impl PermissionCondition {
    /// Evaluate this condition against the given context
    pub fn evaluate(&self, context: &PermissionContext) -> bool {
        match self {
            PermissionCondition::Ownership { owner_field } => context.resource_metadata.get(owner_field).map(|owner| owner == &context.user_id).unwrap_or(false),

            PermissionCondition::TimeWindow { start, end } => {
                let now = Utc::now();

                if let Some(start_time) = start {
                    if now < *start_time {
                        return false;
                    }
                }

                if let Some(end_time) = end {
                    if now > *end_time {
                        return false;
                    }
                }

                true
            }

            PermissionCondition::IpRestriction { allowed_ips } => {
                if let Some(client_ip) = &context.client_ip {
                    allowed_ips.iter().any(|ip| self.ip_matches(client_ip, ip))
                } else {
                    false
                }
            }

            PermissionCondition::Custom { condition_type, parameters } => {
                // Custom condition evaluation can be extended
                match condition_type.as_str() {
                    "user_group" => {
                        if let Some(required_group) = parameters.get("group") {
                            context.user_groups.contains(required_group)
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
        }
    }

    /// Check if IP matches the pattern (supports CIDR notation)
    fn ip_matches(&self, client_ip: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('/') {
            // CIDR notation - simplified check for now
            // In production, use a proper CIDR library
            pattern.split('/').next().unwrap_or("") == client_ip
        } else {
            client_ip == pattern
        }
    }
}

impl Hash for PermissionCondition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PermissionCondition::Ownership { owner_field } => {
                0u8.hash(state);
                owner_field.hash(state);
            }
            PermissionCondition::TimeWindow { start, end } => {
                1u8.hash(state);
                start.hash(state);
                end.hash(state);
            }
            PermissionCondition::IpRestriction { allowed_ips } => {
                2u8.hash(state);
                allowed_ips.hash(state);
            }
            PermissionCondition::Custom { condition_type, parameters } => {
                3u8.hash(state);
                condition_type.hash(state);
                // Hash parameters as sorted key-value pairs for consistency
                let mut sorted_params: Vec<_> = parameters.iter().collect();
                sorted_params.sort_by_key(|(k, _)| *k);
                sorted_params.hash(state);
            }
        }
    }
}

/// Context for permission evaluation
#[derive(Debug, Clone)]
pub struct PermissionContext {
    /// User ID requesting access
    pub user_id: String,

    /// User groups
    pub user_groups: Vec<String>,

    /// Client IP address
    pub client_ip: Option<String>,

    /// Resource metadata for condition evaluation
    pub resource_metadata: HashMap<String, String>,

    /// Request timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional context data
    pub additional_data: HashMap<String, String>,
}

impl PermissionContext {
    /// Create a new permission context
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            user_groups: Vec::new(),
            client_ip: None,
            resource_metadata: HashMap::new(),
            timestamp: Utc::now(),
            additional_data: HashMap::new(),
        }
    }

    /// Set user groups
    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.user_groups = groups;
        self
    }

    /// Set client IP
    pub fn with_client_ip(mut self, ip: Option<String>) -> Self {
        self.client_ip = ip;
        self
    }

    /// Set resource metadata
    pub fn with_resource_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.resource_metadata = metadata;
        self
    }

    /// Add additional data
    pub fn with_additional_data(mut self, data: HashMap<String, String>) -> Self {
        self.additional_data = data;
        self
    }
}

/// Permission checker for evaluating access rights
#[derive(Debug, Clone)]
pub struct PermissionChecker {
    /// Cache for permission evaluation results
    cache: std::sync::Arc<crate::rbac::cache::PermissionCache>,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new(cache: std::sync::Arc<crate::rbac::cache::PermissionCache>) -> Self {
        Self { cache }
    }

    /// Check if permissions allow access to a resource
    pub async fn check_permission(&self, permissions: &[Permission], resource: &str, action: &str, context: &PermissionContext) -> ApiResult<bool> {
        // Check cache first
        let cache_key = format!("{}:{}:{}:{}", context.user_id, resource, action, context.timestamp.timestamp());

        if let Some(cached_result) = self.cache.get_permission(&cache_key).await {
            return Ok(cached_result);
        }

        // Evaluate permissions
        let result = permissions.iter().any(|permission| permission.matches(resource, action) && permission.evaluate_conditions(context));

        // Cache the result
        self.cache.set_permission(cache_key, result, std::time::Duration::from_secs(300)).await;

        Ok(result)
    }

    /// Check dot-specific permissions
    pub async fn check_dot_permission(&self, dot_permissions: &[DotPermission], dot_id: &str, operation: &str, context: &PermissionContext) -> ApiResult<bool> {
        // Check cache first
        let cache_key = format!("dot:{}:{}:{}:{}", context.user_id, dot_id, operation, context.timestamp.timestamp());

        if let Some(cached_result) = self.cache.get_permission(&cache_key).await {
            return Ok(cached_result);
        }

        // Evaluate dot permissions
        let result = dot_permissions.iter().any(|permission| permission.allows_operation(dot_id, operation, context));

        // Cache the result
        self.cache.set_permission(cache_key, result, std::time::Duration::from_secs(300)).await;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_matching() {
        let permission = Permission::new("dots".to_string(), "read".to_string());

        assert!(permission.matches("dots", "read"));
        assert!(!permission.matches("dots", "write"));
        assert!(!permission.matches("users", "read"));
    }

    #[test]
    fn test_wildcard_permission() {
        let permission = Permission::new("*".to_string(), "*".to_string());

        assert!(permission.matches("dots", "read"));
        assert!(permission.matches("users", "write"));
        assert!(permission.matches("collections", "delete"));
    }

    #[test]
    fn test_ownership_condition() {
        let condition = PermissionCondition::Ownership { owner_field: "owner_id".to_string() };

        let mut context = PermissionContext::new("user123".to_string());
        context.resource_metadata.insert("owner_id".to_string(), "user123".to_string());

        assert!(condition.evaluate(&context));

        context.resource_metadata.insert("owner_id".to_string(), "user456".to_string());
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_time_window_condition() {
        let start = Utc::now() - chrono::Duration::hours(1);
        let end = Utc::now() + chrono::Duration::hours(1);

        let condition = PermissionCondition::TimeWindow { start: Some(start), end: Some(end) };

        let context = PermissionContext::new("user123".to_string());
        assert!(condition.evaluate(&context));
    }

    #[test]
    fn test_dot_permission() {
        let dot_permission = DotPermission::new("dot123".to_string(), vec!["read".to_string(), "execute".to_string()]);

        let context = PermissionContext::new("user123".to_string());

        assert!(dot_permission.allows_operation("dot123", "read", &context));
        assert!(dot_permission.allows_operation("dot123", "execute", &context));
        assert!(!dot_permission.allows_operation("dot123", "write", &context));
        assert!(!dot_permission.allows_operation("dot456", "read", &context));
    }
}
