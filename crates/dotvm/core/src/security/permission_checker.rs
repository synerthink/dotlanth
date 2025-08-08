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

//! Permission Checker
//!
//! Implements execution context-based permission checking to ensure
//! opcodes are only executed with appropriate authorization.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::security::errors::{PermissionError, PermissionResult};
use crate::security::types::{CustomOpcode, DotVMContext, OpcodeType, SecurityLevel};

/// Permission checker for authorization
#[derive(Debug)]
pub struct PermissionChecker {
    /// Permission policies by context
    policies: Arc<RwLock<HashMap<String, PermissionPolicy>>>,
    /// Permission grants by dot
    grants: Arc<RwLock<HashMap<String, Vec<PermissionGrant>>>>,
    /// Permission templates
    templates: Arc<RwLock<HashMap<String, PermissionTemplate>>>,
    /// Permission cache for performance
    cache: Arc<RwLock<PermissionCache>>,
    /// Checker configuration
    config: PermissionConfig,
}

/// Individual permission definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Execute permission for a resource
    Execute { resource: String },
    /// Read permission for a resource
    Read { resource: String },
    /// Write permission for a resource
    Write { resource: String },
    /// Delete permission for a resource
    Delete { resource: String },
    /// Admin permission for a scope
    Admin { scope: String },
    /// Custom permission with arbitrary attributes
    Custom { name: String, attributes: Vec<(String, String)> }, // Using Vec instead of HashMap for Hash trait
}

/// Permission policy defining access rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    /// Policy identifier
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Permission rules
    pub rules: Vec<PermissionRule>,
    /// Policy metadata
    pub metadata: HashMap<String, String>,
    /// Policy validity period
    pub valid_from: Option<SystemTime>,
    pub valid_until: Option<SystemTime>,
    /// Policy priority (higher = more important)
    pub priority: u32,
}

/// Individual permission rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Rule identifier
    pub id: String,
    /// Conditions that must be met
    pub conditions: Vec<PermissionCondition>,
    /// Permissions granted if conditions are met
    pub granted_permissions: Vec<Permission>,
    /// Permissions denied if conditions are met
    pub denied_permissions: Vec<Permission>,
    /// Rule effect (allow or deny)
    pub effect: PermissionEffect,
    /// Rule priority within policy
    pub priority: u32,
}

/// Permission rule condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionCondition {
    /// Dot ID matches pattern
    DotIdMatches { pattern: String },
    /// Security level is at least the specified level
    SecurityLevelAtLeast { level: SecurityLevel },
    /// Opcode type matches
    OpcodeTypeMatches { opcode_type: OpcodeType },
    /// Time is within specified range
    TimeInRange { start: SystemTime, end: SystemTime },
    /// Custom condition with arbitrary logic
    Custom { condition_type: String, parameters: HashMap<String, String> },
}

/// Permission rule effect
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionEffect {
    Allow,
    Deny,
}

/// Permission grant for a specific dot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    /// Grant identifier
    pub id: String,
    /// Granted permissions
    pub permissions: Vec<Permission>,
    /// Grant metadata
    pub metadata: HashMap<String, String>,
    /// Who granted the permissions
    pub granted_by: String,
    /// When permissions were granted
    pub granted_at: SystemTime,
    /// Optional expiration time
    pub expires_at: Option<SystemTime>,
    /// Whether grant is currently active
    pub active: bool,
}

/// Permission template for easy grant creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Default permissions in template
    pub permissions: Vec<Permission>,
    /// Default metadata
    pub metadata: HashMap<String, String>,
    /// Default validity duration
    pub default_duration: Option<std::time::Duration>,
}

/// Permission cache for performance optimization
#[derive(Debug, Default)]
pub struct PermissionCache {
    /// Cached permission check results
    check_cache: HashMap<PermissionCacheKey, PermissionCheckResult>,
    /// Cache statistics
    hits: u64,
    misses: u64,
    /// Cache expiration times
    expiration_times: HashMap<PermissionCacheKey, SystemTime>,
}

/// Cache key for permission checks
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionCacheKey {
    dot_id: String,
    opcode_type: String,
    security_level: String,
    timestamp_minute: u64, // To provide some time-based invalidation
}

/// Cached permission check result
#[derive(Debug, Clone)]
pub struct PermissionCheckResult {
    granted_permissions: Vec<Permission>,
    denied_permissions: Vec<Permission>,
    check_result: bool,
    cached_at: SystemTime,
}

/// Permission checker configuration
#[derive(Debug, Clone)]
pub struct PermissionConfig {
    /// Enable permission caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Enable policy evaluation
    pub enable_policy_evaluation: bool,
    /// Default permission behavior (allow or deny)
    pub default_behavior: PermissionEffect,
    /// Enable permission inheritance
    pub enable_inheritance: bool,
    /// Enable time-based permissions
    pub enable_time_based_permissions: bool,
    /// Maximum policy evaluation depth
    pub max_evaluation_depth: u32,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl_seconds: 300, // 5 minutes
            enable_policy_evaluation: true,
            default_behavior: PermissionEffect::Deny, // Secure by default
            enable_inheritance: true,
            enable_time_based_permissions: true,
            max_evaluation_depth: 10,
        }
    }
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new() -> Self {
        Self::with_config(PermissionConfig::default())
    }

    /// Create a new permission checker with custom configuration
    pub fn with_config(config: PermissionConfig) -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            grants: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(PermissionCache::default())),
            config,
        }
    }

    /// Check permissions for an opcode execution
    pub fn check_permission(&self, context: &DotVMContext, opcode: &CustomOpcode, required_permissions: &[Permission]) -> PermissionResult<Vec<Permission>> {
        // Check cache first if enabled
        if self.config.enable_caching {
            if let Some(cached_result) = self.check_cache(context, opcode)? {
                return if cached_result.check_result {
                    Ok(cached_result.granted_permissions)
                } else {
                    Err(PermissionError::InsufficientPermissions {
                        required: required_permissions.iter().map(|p| format!("{:?}", p)).collect(),
                        available: cached_result.granted_permissions.iter().map(|p| format!("{:?}", p)).collect(),
                    })
                };
            }
        }

        // Get granted permissions for the dot
        let granted_permissions = self.get_granted_permissions(&context.dot_id)?;

        // Evaluate policies if enabled
        let policy_permissions = if self.config.enable_policy_evaluation {
            self.evaluate_policies(context, opcode)?
        } else {
            Vec::new()
        };

        // Combine all permissions
        let mut all_permissions = granted_permissions;
        all_permissions.extend(policy_permissions);

        // Check if required permissions are satisfied
        let check_result = self.permissions_satisfied(required_permissions, &all_permissions);

        // Cache the result if caching is enabled
        if self.config.enable_caching {
            self.cache_result(context, opcode, &all_permissions, check_result)?;
        }

        if check_result {
            Ok(all_permissions)
        } else {
            Err(PermissionError::InsufficientPermissions {
                required: required_permissions.iter().map(|p| format!("{:?}", p)).collect(),
                available: all_permissions.iter().map(|p| format!("{:?}", p)).collect(),
            })
        }
    }

    /// Grant permissions to a dot
    pub fn grant_permissions(&self, dot_id: String, permissions: Vec<Permission>, granted_by: String, expires_at: Option<SystemTime>) -> PermissionResult<String> {
        let grant = PermissionGrant {
            id: self.generate_grant_id(),
            permissions,
            metadata: HashMap::new(),
            granted_by,
            granted_at: SystemTime::now(),
            expires_at,
            active: true,
        };

        let grant_id = grant.id.clone();

        let mut grants = self.grants.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire grants lock: {}", e),
        })?;

        grants.entry(dot_id.clone()).or_insert_with(Vec::new).push(grant);

        // Clear cache for this dot
        self.clear_cache_for_dot(&dot_id)?;

        Ok(grant_id)
    }

    /// Revoke permissions from a dot
    pub fn revoke_permissions(&self, dot_id: &str, grant_id: &str) -> PermissionResult<()> {
        let mut grants = self.grants.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire grants lock: {}", e),
        })?;

        if let Some(dot_grants) = grants.get_mut(dot_id) {
            for grant in dot_grants.iter_mut() {
                if grant.id == grant_id {
                    grant.active = false;
                    break;
                }
            }
        }

        // Clear cache for this dot
        self.clear_cache_for_dot(dot_id)?;

        Ok(())
    }

    /// Add a permission policy
    pub fn add_policy(&self, policy: PermissionPolicy) -> PermissionResult<()> {
        let mut policies = self.policies.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        policies.insert(policy.id.clone(), policy);

        // Clear entire cache since policies affect all permissions
        self.clear_cache()?;

        Ok(())
    }

    /// Remove a permission policy
    pub fn remove_policy(&self, policy_id: &str) -> PermissionResult<()> {
        let mut policies = self.policies.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        policies.remove(policy_id);

        // Clear entire cache since policies affect all permissions
        self.clear_cache()?;

        Ok(())
    }

    /// Add a permission template
    pub fn add_template(&self, template: PermissionTemplate) -> PermissionResult<()> {
        let mut templates = self.templates.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire templates lock: {}", e),
        })?;

        templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Create permissions from template
    pub fn create_from_template(&self, template_name: &str, dot_id: String, granted_by: String) -> PermissionResult<String> {
        let templates = self.templates.read().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire templates lock: {}", e),
        })?;

        let template = templates.get(template_name).ok_or_else(|| PermissionError::EvaluationFailed {
            reason: format!("Template '{}' not found", template_name),
        })?;

        let expires_at = template.default_duration.map(|duration| SystemTime::now() + duration);
        let permissions = template.permissions.clone();

        drop(templates); // Release lock before calling grant_permissions

        self.grant_permissions(dot_id, permissions, granted_by, expires_at)
    }

    /// Get permission statistics
    pub fn get_statistics(&self) -> PermissionResult<PermissionStatistics> {
        let grants = self.grants.read().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire grants lock: {}", e),
        })?;

        let policies = self.policies.read().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        let cache = self.cache.read().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire cache lock: {}", e),
        })?;

        let total_grants = grants.values().map(|g| g.len()).sum();
        let active_grants = grants
            .values()
            .flat_map(|g| g.iter())
            .filter(|grant| grant.active && grant.expires_at.map_or(true, |exp| exp > SystemTime::now()))
            .count();

        Ok(PermissionStatistics {
            total_grants,
            active_grants,
            total_policies: policies.len(),
            cache_hits: cache.hits,
            cache_misses: cache.misses,
            cache_hit_rate: if cache.hits + cache.misses > 0 {
                cache.hits as f64 / (cache.hits + cache.misses) as f64
            } else {
                0.0
            },
        })
    }

    // Private helper methods
    fn get_granted_permissions(&self, dot_id: &str) -> PermissionResult<Vec<Permission>> {
        let grants = self.grants.read().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire grants lock: {}", e),
        })?;

        let now = SystemTime::now();
        let mut permissions = Vec::new();

        if let Some(dot_grants) = grants.get(dot_id) {
            for grant in dot_grants {
                if grant.active && grant.expires_at.map_or(true, |exp| exp > now) {
                    permissions.extend(grant.permissions.clone());
                }
            }
        }

        Ok(permissions)
    }

    fn evaluate_policies(&self, context: &DotVMContext, opcode: &CustomOpcode) -> PermissionResult<Vec<Permission>> {
        let policies = self.policies.read().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        let mut granted_permissions = Vec::new();
        let mut denied_permissions = HashSet::new();

        // Sort policies by priority (higher priority first)
        let mut sorted_policies: Vec<_> = policies.values().collect();
        sorted_policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        for policy in sorted_policies {
            // Check if policy is currently valid
            if !self.is_policy_valid(policy) {
                continue;
            }

            // Sort rules by priority within policy
            let mut sorted_rules = policy.rules.clone();
            sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

            for rule in sorted_rules {
                if self.evaluate_conditions(&rule.conditions, context, opcode)? {
                    match rule.effect {
                        PermissionEffect::Allow => {
                            granted_permissions.extend(rule.granted_permissions);
                        }
                        PermissionEffect::Deny => {
                            for perm in &rule.denied_permissions {
                                denied_permissions.insert(perm.clone());
                            }
                        }
                    }
                }
            }
        }

        // Remove denied permissions from granted permissions
        granted_permissions.retain(|perm| !denied_permissions.contains(perm));

        Ok(granted_permissions)
    }

    fn evaluate_conditions(&self, conditions: &[PermissionCondition], context: &DotVMContext, opcode: &CustomOpcode) -> PermissionResult<bool> {
        for condition in conditions {
            if !self.evaluate_condition(condition, context, opcode)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn evaluate_condition(&self, condition: &PermissionCondition, context: &DotVMContext, opcode: &CustomOpcode) -> PermissionResult<bool> {
        match condition {
            PermissionCondition::DotIdMatches { pattern } => Ok(self.pattern_matches(pattern, &context.dot_id)),
            PermissionCondition::SecurityLevelAtLeast { level } => Ok(self.security_level_sufficient(&context.security_level, level)),
            PermissionCondition::OpcodeTypeMatches { opcode_type } => Ok(opcode.opcode_type == *opcode_type),
            PermissionCondition::TimeInRange { start, end } => {
                let now = SystemTime::now();
                Ok(now >= *start && now <= *end)
            }
            PermissionCondition::Custom { condition_type, parameters: _ } => {
                // Custom condition evaluation would be implemented based on specific needs
                match condition_type.as_str() {
                    "always_true" => Ok(true),
                    "always_false" => Ok(false),
                    _ => Ok(false),
                }
            }
        }
    }

    fn permissions_satisfied(&self, required: &[Permission], available: &[Permission]) -> bool {
        for required_perm in required {
            if !self.permission_granted(required_perm, available) {
                return false;
            }
        }
        true
    }

    fn permission_granted(&self, required: &Permission, available: &[Permission]) -> bool {
        available.iter().any(|perm| self.permissions_match(required, perm))
    }

    fn permissions_match(&self, required: &Permission, available: &Permission) -> bool {
        match (required, available) {
            (Permission::Execute { resource: r1 }, Permission::Execute { resource: r2 }) => r1 == r2,
            (Permission::Read { resource: r1 }, Permission::Read { resource: r2 }) => r1 == r2,
            (Permission::Write { resource: r1 }, Permission::Write { resource: r2 }) => r1 == r2,
            (Permission::Delete { resource: r1 }, Permission::Delete { resource: r2 }) => r1 == r2,
            (Permission::Admin { scope: s1 }, Permission::Admin { scope: s2 }) => s1 == s2,
            (Permission::Custom { name: n1, .. }, Permission::Custom { name: n2, .. }) => n1 == n2,
            // Admin permissions grant all other permissions within their scope
            (_, Permission::Admin { scope }) => self.permission_in_scope(required, scope),
            _ => false,
        }
    }

    fn permission_in_scope(&self, permission: &Permission, scope: &str) -> bool {
        match permission {
            Permission::Execute { resource } | Permission::Read { resource } | Permission::Write { resource } | Permission::Delete { resource } => resource.starts_with(scope),
            Permission::Admin { scope: perm_scope } => perm_scope.starts_with(scope),
            Permission::Custom { attributes, .. } => attributes.iter().find(|(k, _)| k == "scope").map_or(false, |(_, v)| v.starts_with(scope)),
        }
    }

    fn pattern_matches(&self, pattern: &str, text: &str) -> bool {
        // Simple glob-like pattern matching
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return text.starts_with(prefix);
        }
        if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            return text.ends_with(suffix);
        }
        pattern == text
    }

    fn security_level_sufficient(&self, current: &SecurityLevel, required: &SecurityLevel) -> bool {
        use SecurityLevel::*;
        match (current, required) {
            (Maximum, _) => true,
            (High, Development | Standard | High) => true,
            (Standard, Development | Standard) => true,
            (Development, Development) => true,
            (Custom { .. }, _) => true, // Custom levels assumed to be properly configured
            _ => false,
        }
    }

    fn is_policy_valid(&self, policy: &PermissionPolicy) -> bool {
        let now = SystemTime::now();

        if let Some(valid_from) = policy.valid_from {
            if now < valid_from {
                return false;
            }
        }

        if let Some(valid_until) = policy.valid_until {
            if now > valid_until {
                return false;
            }
        }

        true
    }

    fn check_cache(&self, context: &DotVMContext, opcode: &CustomOpcode) -> PermissionResult<Option<PermissionCheckResult>> {
        let cache = self.cache.read().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire cache lock: {}", e),
        })?;

        let cache_key = PermissionCacheKey {
            dot_id: context.dot_id.clone(),
            opcode_type: format!("{:?}", opcode.opcode_type),
            security_level: format!("{:?}", context.security_level),
            timestamp_minute: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs() / 60,
        };

        if let Some(result) = cache.check_cache.get(&cache_key) {
            // Check if result has expired
            if let Some(expiration) = cache.expiration_times.get(&cache_key) {
                if SystemTime::now() < *expiration {
                    return Ok(Some(result.clone()));
                }
            }
        }

        Ok(None)
    }

    fn cache_result(&self, context: &DotVMContext, opcode: &CustomOpcode, permissions: &[Permission], check_result: bool) -> PermissionResult<()> {
        let now = SystemTime::now(); // Single time call
        let mut cache = self.cache.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire cache lock: {}", e),
        })?;

        let cache_key = PermissionCacheKey {
            dot_id: context.dot_id.clone(),
            opcode_type: format!("{:?}", opcode.opcode_type),
            security_level: format!("{:?}", context.security_level),
            timestamp_minute: now.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs() / 60,
        };

        let result = PermissionCheckResult {
            granted_permissions: permissions.to_vec(),
            denied_permissions: Vec::new(),
            check_result,
            cached_at: now,
        };

        let expiration = now + std::time::Duration::from_secs(self.config.cache_ttl_seconds);

        cache.check_cache.insert(cache_key.clone(), result);
        cache.expiration_times.insert(cache_key, expiration);

        if check_result {
            cache.hits += 1;
        } else {
            cache.misses += 1;
        }

        Ok(())
    }

    fn clear_cache(&self) -> PermissionResult<()> {
        let mut cache = self.cache.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire cache lock: {}", e),
        })?;

        cache.check_cache.clear();
        cache.expiration_times.clear();
        Ok(())
    }

    fn clear_cache_for_dot(&self, dot_id: &str) -> PermissionResult<()> {
        let mut cache = self.cache.write().map_err(|e| PermissionError::DatabaseError {
            reason: format!("Failed to acquire cache lock: {}", e),
        })?;

        cache.check_cache.retain(|key, _| key.dot_id != dot_id);
        cache.expiration_times.retain(|key, _| key.dot_id != dot_id);
        Ok(())
    }

    fn generate_grant_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        format!("grant_{:x}", now.as_nanos())
    }
}

impl Default for PermissionChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Permission statistics
#[derive(Debug, Clone)]
pub struct PermissionStatistics {
    pub total_grants: usize,
    pub active_grants: usize,
    pub total_policies: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
}

/// Helper functions for creating common permission templates
pub fn create_default_permission_templates() -> Vec<PermissionTemplate> {
    vec![
        PermissionTemplate {
            name: "basic_execution".to_string(),
            description: "Basic execution permissions for arithmetic and stack operations".to_string(),
            permissions: vec![Permission::Execute { resource: "arithmetic".to_string() }, Permission::Execute { resource: "stack".to_string() }],
            metadata: HashMap::new(),
            default_duration: Some(std::time::Duration::from_secs(3600)), // 1 hour
        },
        PermissionTemplate {
            name: "database_reader".to_string(),
            description: "Read-only database access permissions".to_string(),
            permissions: vec![Permission::Read { resource: "database".to_string() }, Permission::Execute { resource: "query".to_string() }],
            metadata: HashMap::new(),
            default_duration: Some(std::time::Duration::from_secs(7200)), // 2 hours
        },
        PermissionTemplate {
            name: "database_writer".to_string(),
            description: "Read-write database access permissions".to_string(),
            permissions: vec![
                Permission::Read { resource: "database".to_string() },
                Permission::Write { resource: "database".to_string() },
                Permission::Execute { resource: "query".to_string() },
                Permission::Execute { resource: "transaction".to_string() },
            ],
            metadata: HashMap::new(),
            default_duration: Some(std::time::Duration::from_secs(1800)), // 30 minutes
        },
        PermissionTemplate {
            name: "system_admin".to_string(),
            description: "System administration permissions".to_string(),
            permissions: vec![Permission::Admin { scope: "system".to_string() }],
            metadata: HashMap::new(),
            default_duration: Some(std::time::Duration::from_secs(900)), // 15 minutes
        },
    ]
}
