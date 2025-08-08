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

//! Capability Manager
//!
//! Implements capability-based security where each opcode execution requires
//! appropriate capabilities with permissions, resource limits, and expiration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::security::errors::{SecurityError, SecurityResult};
use crate::security::permission_checker::Permission;
use crate::security::resource_limiter::ResourceLimits;
use crate::security::types::{CustomOpcode, DotVMContext, OpcodeType, SecurityLevel};

/// Capability Manager for opcode authorization
#[derive(Debug)]
pub struct CapabilityManager {
    /// Capabilities by dot ID
    capabilities: Arc<RwLock<HashMap<String, Vec<Capability>>>>,
    /// Global capability templates
    templates: Arc<RwLock<HashMap<String, CapabilityTemplate>>>,
    /// Capability grants log
    grants_log: Arc<RwLock<Vec<CapabilityGrant>>>,
    /// Configuration
    config: CapabilityConfig,
}

/// Individual capability for specific opcode types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Unique capability identifier
    pub id: String,
    /// Opcode type this capability grants access to
    pub opcode_type: OpcodeType,
    /// Permissions granted by this capability
    pub permissions: Vec<Permission>,
    /// Resource limits enforced with this capability
    pub resource_limits: ResourceLimits,
    /// Optional expiration time
    pub expiration: Option<SystemTime>,
    /// Capability metadata
    pub metadata: CapabilityMetadata,
    /// Whether this capability can be delegated
    pub delegatable: bool,
    /// Security level required for this capability
    pub required_security_level: SecurityLevel,
}

/// Capability metadata for tracking and auditing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// When the capability was created
    pub created_at: SystemTime,
    /// Who granted this capability
    pub granted_by: String,
    /// Purpose or reason for the capability
    pub purpose: String,
    /// Usage statistics
    pub usage_count: u64,
    /// Last used timestamp
    pub last_used: Option<SystemTime>,
    /// Additional custom metadata
    pub custom_data: HashMap<String, String>,
}

/// Capability template for easy creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Opcode types covered by this template
    pub opcode_types: Vec<OpcodeType>,
    /// Default permissions
    pub default_permissions: Vec<Permission>,
    /// Default resource limits
    pub default_resource_limits: ResourceLimits,
    /// Default expiration duration
    pub default_expiration_duration: Option<std::time::Duration>,
    /// Whether capabilities from this template are delegatable
    pub delegatable: bool,
    /// Required security level
    pub required_security_level: SecurityLevel,
}

/// Capability grant record for auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// Grant identifier
    pub id: String,
    /// Capability that was granted
    pub capability_id: String,
    /// Dot that received the capability
    pub dot_id: String,
    /// Who granted the capability
    pub granted_by: String,
    /// When it was granted
    pub granted_at: SystemTime,
    /// Grant expiration
    pub expires_at: Option<SystemTime>,
    /// Grant revocation info
    pub revoked_at: Option<SystemTime>,
    /// Revocation reason
    pub revocation_reason: Option<String>,
}

/// Capability manager configuration
#[derive(Debug, Clone)]
pub struct CapabilityConfig {
    /// Enable capability caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum capabilities per dot
    pub max_capabilities_per_dot: usize,
    /// Enable capability delegation
    pub enable_delegation: bool,
    /// Enable automatic expiration checking
    pub enable_expiration_checking: bool,
    /// Audit all capability operations
    pub audit_all_operations: bool,
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl_seconds: 300, // 5 minutes
            max_capabilities_per_dot: 100,
            enable_delegation: false, // Disabled by default for security
            enable_expiration_checking: true,
            audit_all_operations: true,
        }
    }
}

impl CapabilityManager {
    /// Create a new capability manager
    pub fn new() -> Self {
        Self::with_config(CapabilityConfig::default())
    }

    /// Create a new capability manager with custom configuration
    pub fn with_config(config: CapabilityConfig) -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            grants_log: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Check if a dot has the required capability for an opcode
    pub fn check_capability(&self, context: &DotVMContext, opcode: &CustomOpcode) -> SecurityResult<()> {
        // Fast path: Clone the capabilities we need instead of holding the lock
        let dot_capabilities = {
            let capabilities = self.capabilities.read().map_err(|e| SecurityError::InternalError {
                message: format!("Failed to acquire capabilities lock: {}", e),
            })?;

            capabilities
                .get(&context.dot_id)
                .ok_or_else(|| SecurityError::CapabilityNotFound {
                    opcode_type: opcode.opcode_type.clone(),
                })?
                .clone() // Clone to release lock quickly
        }; // Lock is released here

        // Find matching capability without holding the lock
        let matching_capability = dot_capabilities
            .iter()
            .find(|cap| self.capability_matches(cap, &opcode.opcode_type, context))
            .ok_or_else(|| SecurityError::CapabilityNotFound {
                opcode_type: opcode.opcode_type.clone(),
            })?;

        // Check if capability has expired (using current system time)
        if self.config.enable_expiration_checking {
            if let Some(expiration) = matching_capability.expiration {
                if SystemTime::now() > expiration {
                    return Err(SecurityError::CapabilityExpired {
                        opcode_type: opcode.opcode_type.clone(),
                        expired_at: expiration,
                    });
                }
            }
        }

        // Check security level requirement
        if !self.security_level_sufficient(&context.security_level, &matching_capability.required_security_level) {
            return Err(SecurityError::CapabilityDenied {
                opcode_type: opcode.opcode_type.clone(),
                reason: format!(
                    "Insufficient security level. Required: {:?}, Current: {:?}",
                    matching_capability.required_security_level, context.security_level
                ),
            });
        }

        // Async update usage statistics (non-blocking)
        self.update_capability_usage_async(&context.dot_id, &matching_capability.id);

        Ok(())
    }

    /// Grant a capability to a dot
    pub fn grant_capability(&self, dot_id: String, capability: Capability, granted_by: String) -> SecurityResult<String> {
        // Check if dot already has maximum capabilities
        {
            let capabilities = self.capabilities.read().map_err(|e| SecurityError::InternalError {
                message: format!("Failed to acquire capabilities lock: {}", e),
            })?;

            if let Some(dot_caps) = capabilities.get(&dot_id) {
                if dot_caps.len() >= self.config.max_capabilities_per_dot {
                    return Err(SecurityError::InternalError {
                        message: format!("Dot {} already has maximum number of capabilities ({})", dot_id, self.config.max_capabilities_per_dot),
                    });
                }
            }
        }

        // Add capability
        let grant_id = format!("grant_{}", uuid::Uuid::new_v4());
        let grant = CapabilityGrant {
            id: grant_id.clone(),
            capability_id: capability.id.clone(),
            dot_id: dot_id.clone(),
            granted_by,
            granted_at: SystemTime::now(),
            expires_at: capability.expiration,
            revoked_at: None,
            revocation_reason: None,
        };

        {
            let mut capabilities = self.capabilities.write().map_err(|e| SecurityError::InternalError {
                message: format!("Failed to acquire capabilities write lock: {}", e),
            })?;

            capabilities.entry(dot_id).or_insert_with(Vec::new).push(capability);
        }

        // Log the grant
        {
            let mut grants_log = self.grants_log.write().map_err(|e| SecurityError::InternalError {
                message: format!("Failed to acquire grants log lock: {}", e),
            })?;

            grants_log.push(grant);
        }

        Ok(grant_id)
    }

    /// Revoke a capability from a dot
    pub fn revoke_capability(&self, dot_id: &str, capability_id: &str, reason: String) -> SecurityResult<()> {
        // Remove capability
        {
            let mut capabilities = self.capabilities.write().map_err(|e| SecurityError::InternalError {
                message: format!("Failed to acquire capabilities write lock: {}", e),
            })?;

            if let Some(dot_caps) = capabilities.get_mut(dot_id) {
                dot_caps.retain(|cap| cap.id != capability_id);
            }
        }

        // Update grants log
        {
            let mut grants_log = self.grants_log.write().map_err(|e| SecurityError::InternalError {
                message: format!("Failed to acquire grants log lock: {}", e),
            })?;

            for grant in grants_log.iter_mut() {
                if grant.capability_id == capability_id && grant.dot_id == dot_id {
                    grant.revoked_at = Some(SystemTime::now());
                    grant.revocation_reason = Some(reason.clone());
                    break;
                }
            }
        }

        Ok(())
    }

    /// List capabilities for a dot
    pub fn list_capabilities(&self, dot_id: &str) -> SecurityResult<Vec<Capability>> {
        let capabilities = self.capabilities.read().map_err(|e| SecurityError::InternalError {
            message: format!("Failed to acquire capabilities lock: {}", e),
        })?;

        Ok(capabilities.get(dot_id).cloned().unwrap_or_default())
    }

    /// Create a capability from a template
    pub fn create_from_template(&self, template_name: &str, opcode_type: OpcodeType, granted_by: String) -> SecurityResult<Capability> {
        let templates = self.templates.read().map_err(|e| SecurityError::InternalError {
            message: format!("Failed to acquire templates lock: {}", e),
        })?;

        let template = templates.get(template_name).ok_or_else(|| SecurityError::InvalidCapability {
            reason: format!("Template '{}' not found", template_name),
        })?;

        let expiration = template.default_expiration_duration.map(|duration| SystemTime::now() + duration);

        Ok(Capability {
            id: format!("cap_{}", uuid::Uuid::new_v4()),
            opcode_type,
            permissions: template.default_permissions.clone(),
            resource_limits: template.default_resource_limits.clone(),
            expiration,
            metadata: CapabilityMetadata {
                created_at: SystemTime::now(),
                granted_by,
                purpose: template.description.clone(),
                usage_count: 0,
                last_used: None,
                custom_data: HashMap::new(),
            },
            delegatable: template.delegatable,
            required_security_level: template.required_security_level.clone(),
        })
    }

    /// Register a capability template
    pub fn register_template(&self, template: CapabilityTemplate) -> SecurityResult<()> {
        let mut templates = self.templates.write().map_err(|e| SecurityError::InternalError {
            message: format!("Failed to acquire templates write lock: {}", e),
        })?;

        templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Clean up expired capabilities
    pub fn cleanup_expired_capabilities(&self) -> SecurityResult<usize> {
        let mut capabilities = self.capabilities.write().map_err(|e| SecurityError::InternalError {
            message: format!("Failed to acquire capabilities write lock: {}", e),
        })?;

        let now = SystemTime::now();
        let mut removed_count = 0;

        for dot_caps in capabilities.values_mut() {
            let original_len = dot_caps.len();
            dot_caps.retain(|cap| cap.expiration.map_or(true, |exp| exp > now));
            removed_count += original_len - dot_caps.len();
        }

        Ok(removed_count)
    }

    /// Get capability usage statistics
    pub fn get_usage_statistics(&self, dot_id: &str) -> SecurityResult<CapabilityStatistics> {
        let capabilities = self.capabilities.read().map_err(|e| SecurityError::InternalError {
            message: format!("Failed to acquire capabilities lock: {}", e),
        })?;

        let empty_vec = Vec::new();
        let dot_caps = capabilities.get(dot_id).unwrap_or(&empty_vec);

        let total_capabilities = dot_caps.len();
        let active_capabilities = dot_caps.iter().filter(|cap| cap.expiration.map_or(true, |exp| exp > SystemTime::now())).count();
        let total_usage = dot_caps.iter().map(|cap| cap.metadata.usage_count).sum();

        Ok(CapabilityStatistics {
            total_capabilities,
            active_capabilities,
            expired_capabilities: total_capabilities - active_capabilities,
            total_usage_count: total_usage,
            most_used_capability: dot_caps.iter().max_by_key(|cap| cap.metadata.usage_count).map(|cap| cap.id.clone()),
        })
    }

    // Private helper methods
    fn capability_matches(&self, capability: &Capability, opcode_type: &OpcodeType, _context: &DotVMContext) -> bool {
        capability.opcode_type == *opcode_type
    }

    fn security_level_sufficient(&self, current: &SecurityLevel, required: &SecurityLevel) -> bool {
        use SecurityLevel::*;
        match (current, required) {
            (Maximum, _) => true,
            (High, Development | Standard | High) => true,
            (Standard, Development | Standard) => true,
            (Development, Development) => true,
            (Custom { .. }, _) => true, // Custom levels are assumed to be properly configured
            _ => false,
        }
    }

    /// Update capability usage statistics asynchronously (non-blocking)
    fn update_capability_usage_async(&self, dot_id: &str, capability_id: &str) {
        // Try to acquire write lock without blocking
        if let Ok(mut caps) = self.capabilities.try_write() {
            if let Some(dot_caps) = caps.get_mut(dot_id) {
                if let Some(cap) = dot_caps.iter_mut().find(|c| c.id == capability_id) {
                    cap.metadata.usage_count += 1;
                    cap.metadata.last_used = Some(SystemTime::now());
                }
            }
        }
        // If we can't get the lock, skip the update - it's not critical for security
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability usage statistics
#[derive(Debug, Clone)]
pub struct CapabilityStatistics {
    pub total_capabilities: usize,
    pub active_capabilities: usize,
    pub expired_capabilities: usize,
    pub total_usage_count: u64,
    pub most_used_capability: Option<String>,
}

// Helper function to create default capability templates
pub fn create_default_templates() -> Vec<CapabilityTemplate> {
    vec![
        CapabilityTemplate {
            name: "arithmetic_basic".to_string(),
            description: "Basic arithmetic operations".to_string(),
            opcode_types: vec![OpcodeType::Standard {
                architecture: crate::security::types::OpcodeArchitecture::Arch64,
                category: crate::security::types::OpcodeCategory::Arithmetic,
            }],
            default_permissions: vec![Permission::Execute { resource: "arithmetic".to_string() }],
            default_resource_limits: ResourceLimits {
                max_memory_bytes: 1024 * 1024, // 1MB
                max_cpu_time_ms: 1000,         // 1 second
                max_instruction_count: 10000,
                max_file_descriptors: 0,
                max_network_bytes: 0,
                max_storage_bytes: 0,
                max_call_stack_depth: 10,
            },
            default_expiration_duration: Some(std::time::Duration::from_secs(3600)), // 1 hour
            delegatable: false,
            required_security_level: SecurityLevel::Development,
        },
        CapabilityTemplate {
            name: "database_read".to_string(),
            description: "Database read operations".to_string(),
            opcode_types: vec![
                OpcodeType::Database {
                    operation: crate::security::types::DatabaseOperation::Read,
                },
                OpcodeType::Database {
                    operation: crate::security::types::DatabaseOperation::Query,
                },
            ],
            default_permissions: vec![Permission::Read { resource: "database".to_string() }],
            default_resource_limits: ResourceLimits {
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 5000,              // 5 seconds
                max_instruction_count: 50000,
                max_file_descriptors: 5,
                max_network_bytes: 1024 * 1024, // 1MB
                max_storage_bytes: 0,
                max_call_stack_depth: 20,
            },
            default_expiration_duration: Some(std::time::Duration::from_secs(7200)), // 2 hours
            delegatable: false,
            required_security_level: SecurityLevel::Standard,
        },
        CapabilityTemplate {
            name: "system_admin".to_string(),
            description: "System administration operations".to_string(),
            opcode_types: vec![
                OpcodeType::System {
                    operation: crate::security::types::SystemOperation::ProcessManagement,
                },
                OpcodeType::System {
                    operation: crate::security::types::SystemOperation::MemoryAllocation,
                },
            ],
            default_permissions: vec![Permission::Admin { scope: "system".to_string() }],
            default_resource_limits: ResourceLimits {
                max_memory_bytes: 100 * 1024 * 1024, // 100MB
                max_cpu_time_ms: 30000,              // 30 seconds
                max_instruction_count: 1000000,      // 1M instructions
                max_file_descriptors: 100,
                max_network_bytes: 10 * 1024 * 1024, // 10MB
                max_storage_bytes: 50 * 1024 * 1024, // 50MB
                max_call_stack_depth: 100,
            },
            default_expiration_duration: Some(std::time::Duration::from_secs(1800)), // 30 minutes
            delegatable: false,
            required_security_level: SecurityLevel::Maximum,
        },
    ]
}

// Additional dependency for UUID generation
// Note: This would need to be added to Cargo.toml: uuid = { version = "1.0", features = ["v4"] }
// For now, I'll create a simple UUID-like function
mod uuid {
    pub struct Uuid;

    impl Uuid {
        pub fn new_v4() -> Self {
            Self
        }
    }

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
            write!(f, "{:x}", now.as_nanos())
        }
    }
}
