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

//! Isolation Manager
//!
//! Ensures secure isolation between different dots to prevent
//! cross-dot interference and maintain execution boundaries.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::security::errors::{IsolationError, IsolationResult};
use crate::security::types::{CustomOpcode, DotVMContext, SecurityLevel};

/// Isolation manager for enforcing execution boundaries
#[derive(Debug)]
pub struct IsolationManager {
    /// Isolation contexts by dot ID
    contexts: Arc<RwLock<HashMap<String, IsolationContext>>>,
    /// Resource allocation tracking
    resource_allocations: Arc<RwLock<HashMap<String, ResourceAllocationContext>>>,
    /// Isolation violations log
    violations_log: Arc<RwLock<Vec<IsolationViolation>>>,
    /// Isolation boundaries configuration
    boundaries: Arc<RwLock<IsolationBoundaries>>,
    /// Manager configuration
    config: IsolationConfig,
}

/// Isolation context for a specific dot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationContext {
    /// Dot identifier
    pub dot_id: String,
    /// Isolation level for this dot
    pub isolation_level: SecurityLevel,
    /// Allowed resource access patterns
    pub allowed_resources: Vec<ResourceAccess>,
    /// Forbidden resource access patterns
    pub forbidden_resources: Vec<ResourceAccess>,
    /// Memory isolation configuration
    pub memory_isolation: MemoryIsolation,
    /// Network isolation configuration
    pub network_isolation: NetworkIsolation,
    /// File system isolation configuration
    pub filesystem_isolation: FilesystemIsolation,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Isolation metadata
    pub metadata: HashMap<String, String>,
}

/// Resource access specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceAccess {
    /// Resource type (memory, network, filesystem, etc.)
    pub resource_type: String,
    /// Access pattern (read, write, execute, etc.)
    pub access_pattern: String,
    /// Resource identifier or pattern
    pub resource_id: String,
    /// Additional access constraints
    pub constraints: HashMap<String, String>,
}

/// Memory isolation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryIsolation {
    /// Enable memory isolation
    pub enabled: bool,
    /// Allocated memory ranges
    pub allocated_ranges: Vec<MemoryRange>,
    /// Memory protection level
    pub protection_level: MemoryProtectionLevel,
    /// Enable stack protection
    pub stack_protection: bool,
    /// Enable heap protection
    pub heap_protection: bool,
}

/// Memory range specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRange {
    /// Start address (virtual)
    pub start_address: u64,
    /// End address (virtual)
    pub end_address: u64,
    /// Access permissions
    pub permissions: MemoryPermissions,
    /// Range metadata
    pub metadata: HashMap<String, String>,
}

/// Memory protection levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryProtectionLevel {
    None,
    Basic,
    Enhanced,
    Maximum,
}

/// Memory permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

/// Network isolation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIsolation {
    /// Enable network isolation
    pub enabled: bool,
    /// Allowed network destinations
    pub allowed_destinations: Vec<NetworkDestination>,
    /// Blocked network destinations
    pub blocked_destinations: Vec<NetworkDestination>,
    /// Network bandwidth limits
    pub bandwidth_limits: NetworkBandwidthLimits,
    /// Enable outbound connections
    pub allow_outbound: bool,
    /// Enable inbound connections
    pub allow_inbound: bool,
}

/// Network destination specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDestination {
    /// Destination type (IP, hostname, service, etc.)
    pub destination_type: String,
    /// Destination address or pattern
    pub address: String,
    /// Port range
    pub port_range: Option<(u16, u16)>,
    /// Protocol restrictions
    pub protocols: Vec<String>,
}

/// Network bandwidth limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkBandwidthLimits {
    /// Maximum upload bandwidth in bytes/second
    pub max_upload_bps: u64,
    /// Maximum download bandwidth in bytes/second
    pub max_download_bps: u64,
    /// Maximum concurrent connections
    pub max_connections: u32,
}

/// Filesystem isolation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemIsolation {
    /// Enable filesystem isolation
    pub enabled: bool,
    /// Allowed file paths
    pub allowed_paths: Vec<FilesystemPath>,
    /// Blocked file paths
    pub blocked_paths: Vec<FilesystemPath>,
    /// Filesystem operation limits
    pub operation_limits: FilesystemLimits,
    /// Enable chroot-like isolation
    pub chroot_enabled: bool,
    /// Root directory for chroot
    pub chroot_root: Option<String>,
}

/// Filesystem path specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPath {
    /// Path pattern
    pub path: String,
    /// Access permissions
    pub permissions: FilesystemPermissions,
    /// Whether path is recursive
    pub recursive: bool,
}

/// Filesystem permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub delete: bool,
}

/// Filesystem operation limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemLimits {
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Maximum number of open files
    pub max_open_files: u32,
    /// Maximum disk usage in bytes
    pub max_disk_usage: u64,
}

/// Resource allocation context for tracking
#[derive(Debug, Clone)]
pub struct ResourceAllocationContext {
    /// Dot identifier
    pub dot_id: String,
    /// Currently allocated resources
    pub allocated_resources: HashMap<String, AllocatedResource>,
    /// Resource usage statistics
    pub usage_statistics: ResourceUsageStatistics,
    /// Allocation history
    pub allocation_history: Vec<ResourceAllocationEvent>,
}

/// Individual allocated resource
#[derive(Debug, Clone)]
pub struct AllocatedResource {
    /// Resource identifier
    pub resource_id: String,
    /// Resource type
    pub resource_type: String,
    /// Allocation size or quantity
    pub allocation_size: u64,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Resource metadata
    pub metadata: HashMap<String, String>,
}

/// Resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceUsageStatistics {
    /// Total memory allocated
    pub total_memory_allocated: u64,
    /// Total network bytes transferred
    pub total_network_bytes: u64,
    /// Total filesystem bytes used
    pub total_filesystem_bytes: u64,
    /// Number of active allocations
    pub active_allocations: u32,
    /// Peak memory usage
    pub peak_memory_usage: u64,
}

/// Resource allocation event for auditing
#[derive(Debug, Clone)]
pub struct ResourceAllocationEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event type (allocate, deallocate, etc.)
    pub event_type: String,
    /// Resource type
    pub resource_type: String,
    /// Resource identifier
    pub resource_id: String,
    /// Allocation size
    pub size: u64,
    /// Event metadata
    pub metadata: HashMap<String, String>,
}

/// Isolation violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationViolation {
    /// Violation identifier
    pub id: String,
    /// Violation timestamp
    pub timestamp: SystemTime,
    /// Source dot that caused the violation
    pub source_dot: String,
    /// Target dot affected by the violation
    pub target_dot: Option<String>,
    /// Violation type
    pub violation_type: IsolationViolationType,
    /// Violation details
    pub details: String,
    /// Severity level
    pub severity: ViolationSeverity,
    /// Resolution status
    pub resolved: bool,
}

/// Types of isolation violations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationViolationType {
    MemoryAccess,
    NetworkAccess,
    FilesystemAccess,
    ResourceSharing,
    CrossDotCommunication,
    PrivilegeEscalation,
    BoundaryViolation,
}

/// Violation severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Isolation boundaries configuration
#[derive(Debug, Clone, Default)]
pub struct IsolationBoundaries {
    /// Memory boundaries between dots
    pub memory_boundaries: HashMap<String, MemoryBoundary>,
    /// Network boundaries
    pub network_boundaries: NetworkBoundaries,
    /// Filesystem boundaries
    pub filesystem_boundaries: FilesystemBoundaries,
    /// Resource sharing rules
    pub resource_sharing_rules: Vec<ResourceSharingRule>,
}

/// Memory boundary specification
#[derive(Debug, Clone)]
pub struct MemoryBoundary {
    /// Dot identifier
    pub dot_id: String,
    /// Allocated memory regions
    pub regions: Vec<MemoryRange>,
    /// Boundary enforcement level
    pub enforcement_level: MemoryProtectionLevel,
}

/// Network boundaries configuration
#[derive(Debug, Clone, Default)]
pub struct NetworkBoundaries {
    /// Network namespace isolation
    pub namespace_isolation: bool,
    /// Port allocation ranges per dot
    pub port_allocations: HashMap<String, (u16, u16)>,
    /// Network interface restrictions
    pub interface_restrictions: HashMap<String, Vec<String>>,
}

/// Filesystem boundaries configuration
#[derive(Debug, Clone, Default)]
pub struct FilesystemBoundaries {
    /// Filesystem namespace isolation
    pub namespace_isolation: bool,
    /// Directory boundaries per dot
    pub directory_boundaries: HashMap<String, Vec<String>>,
    /// Temporary directory allocations
    pub temp_directories: HashMap<String, String>,
}

/// Resource sharing rule
#[derive(Debug, Clone)]
pub struct ResourceSharingRule {
    /// Rule identifier
    pub id: String,
    /// Resource type this rule applies to
    pub resource_type: String,
    /// Dots that can share this resource
    pub allowed_dots: HashSet<String>,
    /// Sharing constraints
    pub constraints: HashMap<String, String>,
    /// Rule priority
    pub priority: u32,
}

/// Isolation manager configuration
#[derive(Debug, Clone)]
pub struct IsolationConfig {
    /// Enable isolation enforcement
    pub enabled: bool,
    /// Default isolation level
    pub default_isolation_level: SecurityLevel,
    /// Enable memory isolation
    pub enable_memory_isolation: bool,
    /// Enable network isolation
    pub enable_network_isolation: bool,
    /// Enable filesystem isolation
    pub enable_filesystem_isolation: bool,
    /// Enable resource tracking
    pub enable_resource_tracking: bool,
    /// Violation detection threshold
    pub violation_detection_threshold: ViolationSeverity,
    /// Enable automatic violation response
    pub enable_auto_response: bool,
    /// Maximum violation history to keep
    pub max_violation_history: usize,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_isolation_level: SecurityLevel::Standard,
            enable_memory_isolation: true,
            enable_network_isolation: true,
            enable_filesystem_isolation: true,
            enable_resource_tracking: true,
            violation_detection_threshold: ViolationSeverity::Medium,
            enable_auto_response: true,
            max_violation_history: 1000,
        }
    }
}

impl IsolationManager {
    /// Create a new isolation manager
    pub fn new() -> Self {
        Self::with_config(IsolationConfig::default())
    }

    /// Create a new isolation manager with custom configuration
    pub fn with_config(config: IsolationConfig) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            resource_allocations: Arc::new(RwLock::new(HashMap::new())),
            violations_log: Arc::new(RwLock::new(Vec::new())),
            boundaries: Arc::new(RwLock::new(IsolationBoundaries::default())),
            config,
        }
    }

    /// Create an isolation context for a dot
    pub fn create_isolation_context(&self, dot_id: String, isolation_level: SecurityLevel) -> IsolationResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let context = IsolationContext {
            dot_id: dot_id.clone(),
            isolation_level: isolation_level.clone(),
            allowed_resources: self.create_default_allowed_resources(&isolation_level),
            forbidden_resources: self.create_default_forbidden_resources(&isolation_level),
            memory_isolation: self.create_memory_isolation_config(&isolation_level),
            network_isolation: self.create_network_isolation_config(&isolation_level),
            filesystem_isolation: self.create_filesystem_isolation_config(&isolation_level),
            created_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let mut contexts = self.contexts.write().map_err(|e| IsolationError::ContextCreationFailed {
            dot_id: dot_id.clone(),
            reason: format!("Failed to acquire contexts lock: {}", e),
        })?;

        contexts.insert(dot_id.clone(), context);

        // Initialize resource allocation context
        let allocation_context = ResourceAllocationContext {
            dot_id: dot_id.clone(),
            allocated_resources: HashMap::new(),
            usage_statistics: ResourceUsageStatistics::default(),
            allocation_history: Vec::new(),
        };

        let mut allocations = self.resource_allocations.write().map_err(|e| IsolationError::ContextCreationFailed {
            dot_id: dot_id.clone(),
            reason: format!("Failed to acquire allocations lock: {}", e),
        })?;

        allocations.insert(dot_id, allocation_context);

        Ok(())
    }

    /// Check if a resource access is allowed
    pub fn check_resource_access(&self, context: &DotVMContext, resource_type: &str, resource_id: &str, access_pattern: &str) -> IsolationResult<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        let contexts = self.contexts.read().map_err(|e| IsolationError::BoundaryViolation {
            dot_id: context.dot_id.clone(),
            violation_type: format!("Failed to acquire contexts lock: {}", e),
        })?;

        let isolation_context = contexts.get(&context.dot_id).ok_or_else(|| IsolationError::ContextCreationFailed {
            dot_id: context.dot_id.clone(),
            reason: "Isolation context not found".to_string(),
        })?;

        // Check if access is explicitly forbidden
        for forbidden in &isolation_context.forbidden_resources {
            if self.resource_matches(forbidden, resource_type, resource_id, access_pattern) {
                self.log_violation(
                    &context.dot_id,
                    None,
                    IsolationViolationType::ResourceSharing,
                    &format!("Forbidden resource access: {} {} {}", resource_type, resource_id, access_pattern),
                    ViolationSeverity::High,
                )?;
                return Ok(false);
            }
        }

        // Check if access is explicitly allowed
        for allowed in &isolation_context.allowed_resources {
            if self.resource_matches(allowed, resource_type, resource_id, access_pattern) {
                return Ok(true);
            }
        }

        // Default behavior based on isolation level
        match isolation_context.isolation_level {
            SecurityLevel::Development => Ok(true), // Permissive for development
            SecurityLevel::Standard => Ok(self.is_safe_resource_access(resource_type, access_pattern)),
            SecurityLevel::High | SecurityLevel::Maximum => Ok(false), // Strict isolation
            SecurityLevel::Custom { .. } => Ok(true),                  // Custom configuration handles this
        }
    }

    /// Check for cross-dot interference
    pub fn check_cross_dot_interference(&self, source_context: &DotVMContext, target_dot_id: &str, operation: &str) -> IsolationResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if source_context.dot_id == target_dot_id {
            return Ok(()); // Same dot, no interference
        }

        // Check if cross-dot communication is allowed
        if !self.is_cross_dot_communication_allowed(&source_context.dot_id, target_dot_id, operation)? {
            self.log_violation(
                &source_context.dot_id,
                Some(target_dot_id.to_string()),
                IsolationViolationType::CrossDotCommunication,
                &format!("Unauthorized cross-dot operation: {}", operation),
                ViolationSeverity::High,
            )?;

            return Err(IsolationError::CrossDotInterference {
                source_dot: source_context.dot_id.clone(),
                target_dot: target_dot_id.to_string(),
                interference_type: operation.to_string(),
            });
        }

        Ok(())
    }

    /// Validate isolation boundaries
    pub fn validate_isolation_boundaries(&self, context: &DotVMContext, opcode: &CustomOpcode) -> IsolationResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check memory boundaries
        if self.config.enable_memory_isolation {
            self.validate_memory_boundaries(context, opcode)?;
        }

        // Check network boundaries
        if self.config.enable_network_isolation {
            self.validate_network_boundaries(context, opcode)?;
        }

        // Check filesystem boundaries
        if self.config.enable_filesystem_isolation {
            self.validate_filesystem_boundaries(context, opcode)?;
        }

        Ok(())
    }

    /// Allocate a resource to a dot
    pub fn allocate_resource(&self, dot_id: &str, resource_type: &str, resource_id: &str, size: u64) -> IsolationResult<()> {
        if !self.config.enable_resource_tracking {
            return Ok(());
        }

        let mut allocations = self.resource_allocations.write().map_err(|e| IsolationError::ResourceSharingViolation {
            resource_type: resource_type.to_string(),
            reason: format!("Failed to acquire allocations lock: {}", e),
        })?;

        let allocation_context = allocations.entry(dot_id.to_string()).or_insert_with(|| ResourceAllocationContext {
            dot_id: dot_id.to_string(),
            allocated_resources: HashMap::new(),
            usage_statistics: ResourceUsageStatistics::default(),
            allocation_history: Vec::new(),
        });

        let resource = AllocatedResource {
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            allocation_size: size,
            allocated_at: SystemTime::now(),
            metadata: HashMap::new(),
        };

        allocation_context.allocated_resources.insert(resource_id.to_string(), resource);

        // Update usage statistics
        match resource_type {
            "memory" => allocation_context.usage_statistics.total_memory_allocated += size,
            "network" => allocation_context.usage_statistics.total_network_bytes += size,
            "filesystem" => allocation_context.usage_statistics.total_filesystem_bytes += size,
            _ => {}
        }

        allocation_context.usage_statistics.active_allocations += 1;

        // Record allocation event
        let event = ResourceAllocationEvent {
            timestamp: SystemTime::now(),
            event_type: "allocate".to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            size,
            metadata: HashMap::new(),
        };

        allocation_context.allocation_history.push(event);

        Ok(())
    }

    /// Deallocate a resource from a dot
    pub fn deallocate_resource(&self, dot_id: &str, resource_id: &str) -> IsolationResult<()> {
        if !self.config.enable_resource_tracking {
            return Ok(());
        }

        let mut allocations = self.resource_allocations.write().map_err(|e| IsolationError::ResourceSharingViolation {
            resource_type: "unknown".to_string(),
            reason: format!("Failed to acquire allocations lock: {}", e),
        })?;

        if let Some(allocation_context) = allocations.get_mut(dot_id) {
            if let Some(resource) = allocation_context.allocated_resources.remove(resource_id) {
                // Update usage statistics
                match resource.resource_type.as_str() {
                    "memory" => allocation_context.usage_statistics.total_memory_allocated -= resource.allocation_size,
                    "network" => allocation_context.usage_statistics.total_network_bytes -= resource.allocation_size,
                    "filesystem" => allocation_context.usage_statistics.total_filesystem_bytes -= resource.allocation_size,
                    _ => {}
                }

                allocation_context.usage_statistics.active_allocations -= 1;

                // Record deallocation event
                let event = ResourceAllocationEvent {
                    timestamp: SystemTime::now(),
                    event_type: "deallocate".to_string(),
                    resource_type: resource.resource_type,
                    resource_id: resource_id.to_string(),
                    size: resource.allocation_size,
                    metadata: HashMap::new(),
                };

                allocation_context.allocation_history.push(event);
            }
        }

        Ok(())
    }

    /// Get isolation statistics
    pub fn get_isolation_statistics(&self, dot_id: &str) -> IsolationResult<IsolationStatistics> {
        let contexts = self.contexts.read().map_err(|e| IsolationError::BoundaryViolation {
            dot_id: dot_id.to_string(),
            violation_type: format!("Failed to acquire contexts lock: {}", e),
        })?;

        let allocations = self.resource_allocations.read().map_err(|e| IsolationError::ResourceSharingViolation {
            resource_type: "unknown".to_string(),
            reason: format!("Failed to acquire allocations lock: {}", e),
        })?;

        let violations = self.violations_log.read().map_err(|e| IsolationError::BoundaryViolation {
            dot_id: dot_id.to_string(),
            violation_type: format!("Failed to acquire violations lock: {}", e),
        })?;

        let context = contexts.get(dot_id);
        let allocation_context = allocations.get(dot_id);

        let violation_count = violations.iter().filter(|v| v.source_dot == dot_id || v.target_dot.as_ref() == Some(&dot_id.to_string())).count();

        Ok(IsolationStatistics {
            dot_id: dot_id.to_string(),
            isolation_level: context.map(|c| c.isolation_level.clone()).unwrap_or_default(),
            total_violations: violation_count,
            active_allocations: allocation_context.map(|a| a.usage_statistics.active_allocations).unwrap_or(0),
            total_memory_allocated: allocation_context.map(|a| a.usage_statistics.total_memory_allocated).unwrap_or(0),
            creation_time: context.map(|c| c.created_at),
            last_activity: context.map(|c| c.last_activity),
        })
    }

    // Private helper methods
    fn create_default_allowed_resources(&self, level: &SecurityLevel) -> Vec<ResourceAccess> {
        match level {
            SecurityLevel::Development => vec![
                ResourceAccess {
                    resource_type: "memory".to_string(),
                    access_pattern: "read,write".to_string(),
                    resource_id: "*".to_string(),
                    constraints: HashMap::new(),
                },
                ResourceAccess {
                    resource_type: "network".to_string(),
                    access_pattern: "outbound".to_string(),
                    resource_id: "localhost".to_string(),
                    constraints: HashMap::new(),
                },
            ],
            SecurityLevel::Standard => vec![ResourceAccess {
                resource_type: "memory".to_string(),
                access_pattern: "read,write".to_string(),
                resource_id: "allocated".to_string(),
                constraints: HashMap::new(),
            }],
            _ => Vec::new(),
        }
    }

    fn create_default_forbidden_resources(&self, level: &SecurityLevel) -> Vec<ResourceAccess> {
        match level {
            SecurityLevel::High | SecurityLevel::Maximum => vec![
                ResourceAccess {
                    resource_type: "network".to_string(),
                    access_pattern: "*".to_string(),
                    resource_id: "*".to_string(),
                    constraints: HashMap::new(),
                },
                ResourceAccess {
                    resource_type: "filesystem".to_string(),
                    access_pattern: "write,delete".to_string(),
                    resource_id: "/system/*".to_string(),
                    constraints: HashMap::new(),
                },
            ],
            _ => Vec::new(),
        }
    }

    fn create_memory_isolation_config(&self, level: &SecurityLevel) -> MemoryIsolation {
        MemoryIsolation {
            enabled: self.config.enable_memory_isolation,
            allocated_ranges: Vec::new(),
            protection_level: match level {
                SecurityLevel::Development => MemoryProtectionLevel::Basic,
                SecurityLevel::Standard => MemoryProtectionLevel::Enhanced,
                SecurityLevel::High | SecurityLevel::Maximum => MemoryProtectionLevel::Maximum,
                SecurityLevel::Custom { .. } => MemoryProtectionLevel::Enhanced,
            },
            stack_protection: matches!(level, SecurityLevel::High | SecurityLevel::Maximum),
            heap_protection: matches!(level, SecurityLevel::High | SecurityLevel::Maximum),
        }
    }

    fn create_network_isolation_config(&self, level: &SecurityLevel) -> NetworkIsolation {
        NetworkIsolation {
            enabled: self.config.enable_network_isolation,
            allowed_destinations: Vec::new(),
            blocked_destinations: Vec::new(),
            bandwidth_limits: NetworkBandwidthLimits {
                max_upload_bps: match level {
                    SecurityLevel::Development => 10 * 1024 * 1024, // 10 MB/s
                    SecurityLevel::Standard => 5 * 1024 * 1024,     // 5 MB/s
                    SecurityLevel::High => 1024 * 1024,             // 1 MB/s
                    SecurityLevel::Maximum => 512 * 1024,           // 512 KB/s
                    SecurityLevel::Custom { .. } => 5 * 1024 * 1024,
                },
                max_download_bps: match level {
                    SecurityLevel::Development => 50 * 1024 * 1024, // 50 MB/s
                    SecurityLevel::Standard => 20 * 1024 * 1024,    // 20 MB/s
                    SecurityLevel::High => 5 * 1024 * 1024,         // 5 MB/s
                    SecurityLevel::Maximum => 1024 * 1024,          // 1 MB/s
                    SecurityLevel::Custom { .. } => 20 * 1024 * 1024,
                },
                max_connections: match level {
                    SecurityLevel::Development => 100,
                    SecurityLevel::Standard => 50,
                    SecurityLevel::High => 10,
                    SecurityLevel::Maximum => 5,
                    SecurityLevel::Custom { .. } => 50,
                },
            },
            allow_outbound: !matches!(level, SecurityLevel::Maximum),
            allow_inbound: matches!(level, SecurityLevel::Development),
        }
    }

    fn create_filesystem_isolation_config(&self, level: &SecurityLevel) -> FilesystemIsolation {
        FilesystemIsolation {
            enabled: self.config.enable_filesystem_isolation,
            allowed_paths: Vec::new(),
            blocked_paths: Vec::new(),
            operation_limits: FilesystemLimits {
                max_file_size: match level {
                    SecurityLevel::Development => 100 * 1024 * 1024, // 100 MB
                    SecurityLevel::Standard => 50 * 1024 * 1024,     // 50 MB
                    SecurityLevel::High => 10 * 1024 * 1024,         // 10 MB
                    SecurityLevel::Maximum => 1024 * 1024,           // 1 MB
                    SecurityLevel::Custom { .. } => 50 * 1024 * 1024,
                },
                max_open_files: match level {
                    SecurityLevel::Development => 1000,
                    SecurityLevel::Standard => 500,
                    SecurityLevel::High => 100,
                    SecurityLevel::Maximum => 50,
                    SecurityLevel::Custom { .. } => 500,
                },
                max_disk_usage: match level {
                    SecurityLevel::Development => 1024 * 1024 * 1024, // 1 GB
                    SecurityLevel::Standard => 500 * 1024 * 1024,     // 500 MB
                    SecurityLevel::High => 100 * 1024 * 1024,         // 100 MB
                    SecurityLevel::Maximum => 50 * 1024 * 1024,       // 50 MB
                    SecurityLevel::Custom { .. } => 500 * 1024 * 1024,
                },
            },
            chroot_enabled: matches!(level, SecurityLevel::High | SecurityLevel::Maximum),
            chroot_root: None,
        }
    }

    fn resource_matches(&self, resource_access: &ResourceAccess, resource_type: &str, resource_id: &str, access_pattern: &str) -> bool {
        // Check resource type
        if resource_access.resource_type != "*" && resource_access.resource_type != resource_type {
            return false;
        }

        // Check resource ID (with glob pattern support)
        if !self.pattern_matches(&resource_access.resource_id, resource_id) {
            return false;
        }

        // Check access pattern
        if resource_access.access_pattern == "*" {
            return true;
        }

        let allowed_patterns: HashSet<&str> = resource_access.access_pattern.split(',').collect();
        let requested_patterns: HashSet<&str> = access_pattern.split(',').collect();

        requested_patterns.iter().all(|pattern| allowed_patterns.contains(pattern))
    }

    fn pattern_matches(&self, pattern: &str, text: &str) -> bool {
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

    fn is_safe_resource_access(&self, resource_type: &str, access_pattern: &str) -> bool {
        match resource_type {
            "memory" => true,                         // Memory access is generally safe if bounded
            "network" => access_pattern == "read",    // Only allow read access
            "filesystem" => access_pattern == "read", // Only allow read access
            _ => false,
        }
    }

    fn is_cross_dot_communication_allowed(&self, _source_dot: &str, _target_dot: &str, _operation: &str) -> IsolationResult<bool> {
        // For now, disallow all cross-dot communication for security
        // In a real implementation, this would check sharing rules
        Ok(false)
    }

    fn validate_memory_boundaries(&self, _context: &DotVMContext, _opcode: &CustomOpcode) -> IsolationResult<()> {
        // Memory boundary validation would be implemented here
        // For now, just return success
        Ok(())
    }

    fn validate_network_boundaries(&self, _context: &DotVMContext, _opcode: &CustomOpcode) -> IsolationResult<()> {
        // Network boundary validation would be implemented here
        // For now, just return success
        Ok(())
    }

    fn validate_filesystem_boundaries(&self, _context: &DotVMContext, _opcode: &CustomOpcode) -> IsolationResult<()> {
        // Filesystem boundary validation would be implemented here
        // For now, just return success
        Ok(())
    }

    fn log_violation(&self, source_dot: &str, target_dot: Option<String>, violation_type: IsolationViolationType, details: &str, severity: ViolationSeverity) -> IsolationResult<()> {
        let violation = IsolationViolation {
            id: self.generate_violation_id(),
            timestamp: SystemTime::now(),
            source_dot: source_dot.to_string(),
            target_dot,
            violation_type,
            details: details.to_string(),
            severity,
            resolved: false,
        };

        let mut violations = self.violations_log.write().map_err(|e| IsolationError::BoundaryViolation {
            dot_id: source_dot.to_string(),
            violation_type: format!("Failed to acquire violations lock: {}", e),
        })?;

        violations.push(violation);

        // Limit violation history size
        if violations.len() > self.config.max_violation_history {
            violations.remove(0);
        }

        Ok(())
    }

    fn generate_violation_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        format!("violation_{:x}", now.as_nanos())
    }
}

impl Default for IsolationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Isolation statistics
#[derive(Debug, Clone)]
pub struct IsolationStatistics {
    pub dot_id: String,
    pub isolation_level: SecurityLevel,
    pub total_violations: usize,
    pub active_allocations: u32,
    pub total_memory_allocated: u64,
    pub creation_time: Option<SystemTime>,
    pub last_activity: Option<SystemTime>,
}
