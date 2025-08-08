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

//! Security Type Definitions
//!
//! Core types used throughout the security system including opcodes,
//! contexts, security levels, and various capability types.

use crate::vm::executor::ExecutionContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
// Note: Opcode types would be used for more specific type matching in a full implementation

/// Unified opcode type for security operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpcodeType {
    /// Standard VM opcodes
    Standard { architecture: OpcodeArchitecture, category: OpcodeCategory },
    /// Custom application-defined opcodes
    Custom { id: u32, name: String, category: OpcodeCategory },
    /// System-level opcodes (privileged)
    System { operation: SystemOperation },
    /// Database-specific opcodes
    Database { operation: DatabaseOperation },
    /// Cryptographic opcodes
    Cryptographic { operation: CryptoOperation },
}

/// VM architecture types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpcodeArchitecture {
    Arch64,
    Arch128,
    Arch256,
    Arch512,
}

/// Opcode categories for permission grouping
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpcodeCategory {
    Arithmetic,
    ControlFlow,
    Memory,
    Stack,
    Io,
    Network,
    FileSystem,
    Cryptography,
    Database,
    State,
    Parallel,
    Vector,
    System,
    Custom,
}

/// System-level operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemOperation {
    ProcessManagement,
    MemoryAllocation,
    FileAccess,
    NetworkAccess,
    TimeAccess,
    EnvironmentAccess,
}

/// Database operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DatabaseOperation {
    Read,
    Write,
    Delete,
    Query,
    Transaction,
    Schema,
}

/// Cryptographic operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoOperation {
    Hash,
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    KeyGeneration,
    KeyDerivation,
}

/// Custom opcode wrapper for security system
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomOpcode {
    pub opcode_type: OpcodeType,
    pub parameters: Vec<u8>,
    pub metadata: OpcodeMetadata,
}

/// Metadata associated with opcode execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpcodeMetadata {
    pub source_location: Option<String>,
    pub call_stack_depth: u32,
    pub execution_count: u64,
    pub estimated_cost: ResourceCost,
}

/// Resource cost estimation for opcodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceCost {
    pub cpu_cycles: u64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bytes: u64,
    pub execution_time_ms: u64,
}

/// DotVM execution context extended with security information
#[derive(Debug, Clone)]
pub struct DotVMContext {
    /// Core VM execution context
    pub execution_context: ExecutionContext,
    /// Dot identifier executing the code
    pub dot_id: String,
    /// Execution session identifier
    pub session_id: String,
    /// Security level for this execution
    pub security_level: SecurityLevel,
    /// Caller information (for nested calls)
    pub caller_context: Option<Box<DotVMContext>>,
    /// Additional security metadata
    pub security_metadata: SecurityMetadata,
    /// Resource tracking
    pub resource_usage: CurrentResourceUsage,
}

/// Security levels for execution contexts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Minimal security (development only)
    Development,
    /// Standard security level
    Standard,
    /// High security level
    High,
    /// Maximum security level
    Maximum,
    /// Custom security level with specific configuration
    Custom { name: String, configuration: SecurityConfiguration },
}

/// Security configuration for custom levels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecurityConfiguration {
    pub enable_capability_checking: bool,
    pub enable_resource_limiting: bool,
    pub enable_audit_logging: bool,
    pub enable_permission_checking: bool,
    pub enable_isolation: bool,
    pub enable_policy_enforcement: bool,
    pub strict_mode: bool,
}

/// Security metadata for execution context
#[derive(Debug, Clone)]
pub struct SecurityMetadata {
    pub start_time: SystemTime,
    pub permissions_checked: Vec<String>,
    pub capabilities_used: Vec<String>,
    pub resource_allocations: Vec<ResourceAllocation>,
    pub audit_trail: Vec<SecurityEvent>,
}

/// Resource allocation tracking
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub resource_type: String,
    pub amount: u64,
    pub allocated_at: SystemTime,
    pub metadata: HashMap<String, String>,
}

/// Security events for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_type: SecurityEventType,
    pub timestamp: SystemTime,
    pub details: HashMap<String, String>,
}

/// Types of security events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventType {
    CapabilityCheck,
    PermissionCheck,
    ResourceLimit,
    IsolationViolation,
    PolicyViolation,
    AuditEvent,
}

/// Current resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct CurrentResourceUsage {
    pub memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub instruction_count: u64,
    pub file_descriptors: u32,
    pub network_bytes: u64,
    pub storage_bytes: u64,
    pub call_stack_depth: u32,
    pub last_updated: Option<SystemTime>,
}

/// Opcode execution result for security tracking
#[derive(Debug, Clone)]
pub struct OpcodeResult {
    pub success: bool,
    pub return_value: Option<Vec<u8>>,
    pub resource_consumed: ResourceCost,
    pub execution_time: Duration,
    pub side_effects: Vec<SideEffect>,
    pub errors: Vec<String>,
}

/// Side effects from opcode execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideEffect {
    MemoryAllocation { size: u64 },
    MemoryDeallocation { size: u64 },
    FileAccess { path: String, operation: String },
    NetworkAccess { destination: String, bytes: u64 },
    DatabaseAccess { operation: String, affected_records: u64 },
    StateModification { key: String, operation: String },
    CryptoOperation { operation: String, key_id: Option<String> },
}

impl OpcodeType {
    /// Get the security risk level of this opcode type
    pub fn risk_level(&self) -> SecurityRiskLevel {
        match self {
            OpcodeType::Standard { category, .. } => match category {
                OpcodeCategory::Arithmetic | OpcodeCategory::Stack => SecurityRiskLevel::Low,
                OpcodeCategory::Memory | OpcodeCategory::ControlFlow => SecurityRiskLevel::Medium,
                OpcodeCategory::Io | OpcodeCategory::Network | OpcodeCategory::FileSystem => SecurityRiskLevel::High,
                OpcodeCategory::System => SecurityRiskLevel::Critical,
                _ => SecurityRiskLevel::Medium,
            },
            OpcodeType::Custom { .. } => SecurityRiskLevel::High,
            OpcodeType::System { .. } => SecurityRiskLevel::Critical,
            OpcodeType::Database { .. } => SecurityRiskLevel::Medium,
            OpcodeType::Cryptographic { .. } => SecurityRiskLevel::High,
        }
    }

    /// Check if this opcode type requires elevated permissions
    pub fn requires_elevated_permissions(&self) -> bool {
        matches!(self.risk_level(), SecurityRiskLevel::High | SecurityRiskLevel::Critical)
    }

    /// Get the default resource cost for this opcode type
    pub fn default_resource_cost(&self) -> ResourceCost {
        match self {
            OpcodeType::Standard { category, .. } => match category {
                OpcodeCategory::Arithmetic => ResourceCost {
                    cpu_cycles: 10,
                    memory_bytes: 0,
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_time_ms: 1,
                },
                OpcodeCategory::Memory => ResourceCost {
                    cpu_cycles: 100,
                    memory_bytes: 1024,
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_time_ms: 5,
                },
                OpcodeCategory::Database => ResourceCost {
                    cpu_cycles: 1000,
                    memory_bytes: 4096,
                    storage_bytes: 1024,
                    network_bytes: 0,
                    execution_time_ms: 50,
                },
                _ => ResourceCost {
                    cpu_cycles: 50,
                    memory_bytes: 512,
                    storage_bytes: 0,
                    network_bytes: 0,
                    execution_time_ms: 2,
                },
            },
            OpcodeType::Custom { .. } => ResourceCost {
                cpu_cycles: 500,
                memory_bytes: 2048,
                storage_bytes: 0,
                network_bytes: 0,
                execution_time_ms: 10,
            },
            OpcodeType::System { .. } => ResourceCost {
                cpu_cycles: 2000,
                memory_bytes: 8192,
                storage_bytes: 0,
                network_bytes: 0,
                execution_time_ms: 100,
            },
            OpcodeType::Database { .. } => ResourceCost {
                cpu_cycles: 1500,
                memory_bytes: 4096,
                storage_bytes: 2048,
                network_bytes: 0,
                execution_time_ms: 75,
            },
            OpcodeType::Cryptographic { .. } => ResourceCost {
                cpu_cycles: 5000,
                memory_bytes: 2048,
                storage_bytes: 0,
                network_bytes: 0,
                execution_time_ms: 200,
            },
        }
    }
}

/// Security risk levels for opcode classification
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for SecurityConfiguration {
    fn default() -> Self {
        Self {
            enable_capability_checking: true,
            enable_resource_limiting: true,
            enable_audit_logging: true,
            enable_permission_checking: true,
            enable_isolation: true,
            enable_policy_enforcement: true,
            strict_mode: false,
        }
    }
}

impl Default for SecurityLevel {
    fn default() -> Self {
        SecurityLevel::Standard
    }
}

impl Default for ResourceCost {
    fn default() -> Self {
        Self {
            cpu_cycles: 1,
            memory_bytes: 0,
            storage_bytes: 0,
            network_bytes: 0,
            execution_time_ms: 1,
        }
    }
}

impl CurrentResourceUsage {
    /// Update resource usage with new consumption
    pub fn add_consumption(&mut self, cost: &ResourceCost) {
        self.memory_bytes += cost.memory_bytes;
        self.cpu_time_ms += cost.execution_time_ms;
        self.instruction_count += 1;
        self.last_updated = Some(SystemTime::now());
    }

    /// Check if usage exceeds any of the provided limits
    pub fn exceeds_limits(&self, limits: &super::resource_limiter::ResourceLimits) -> bool {
        self.memory_bytes > limits.max_memory_bytes
            || self.cpu_time_ms > limits.max_cpu_time_ms
            || self.instruction_count > limits.max_instruction_count
            || self.file_descriptors > limits.max_file_descriptors
            || self.network_bytes > limits.max_network_bytes
            || self.storage_bytes > limits.max_storage_bytes
            || self.call_stack_depth > limits.max_call_stack_depth
    }

    /// Reset all usage counters
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}
