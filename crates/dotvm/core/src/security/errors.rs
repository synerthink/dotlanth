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

//! Security Error Types
//!
//! Defines comprehensive error types for all security operations
//! including capability violations, resource exhaustion, and policy failures.

use crate::security::types::{OpcodeType, SecurityLevel};
use std::fmt;
use std::time::SystemTime;

/// Main security error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Capability check failed
    CapabilityDenied { opcode_type: OpcodeType, reason: String },
    /// Required capability not found
    CapabilityNotFound { opcode_type: OpcodeType },
    /// Capability has expired
    CapabilityExpired { opcode_type: OpcodeType, expired_at: SystemTime },
    /// Invalid capability format
    InvalidCapability { reason: String },
    /// Resource limit exceeded
    ResourceLimitExceeded(ResourceError),
    /// Permission denied
    PermissionDenied(PermissionError),
    /// Isolation violation
    IsolationViolation(IsolationError),
    /// Security policy violation
    PolicyViolation(PolicyError),
    /// Audit logging failed
    AuditFailure(AuditError),
    /// Internal security system error
    InternalError { message: String },
}

/// Resource-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    /// Memory limit exceeded
    MemoryLimitExceeded { current: u64, limit: u64 },
    /// CPU time limit exceeded
    CpuTimeExceeded { current_ms: u64, limit_ms: u64 },
    /// Instruction count limit exceeded
    InstructionCountExceeded { current: u64, limit: u64 },
    /// File descriptor limit exceeded
    FileDescriptorLimitExceeded { current: u32, limit: u32 },
    /// Network bandwidth limit exceeded
    NetworkBandwidthExceeded { current_bytes: u64, limit_bytes: u64 },
    /// Storage quota exceeded
    StorageQuotaExceeded { current_bytes: u64, quota_bytes: u64 },
    /// Call stack depth exceeded
    CallStackDepthExceeded { current: u32, limit: u32 },
    /// Resource allocation failed
    AllocationFailed { resource_type: String, reason: String },
}

/// Audit logging errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    /// Log write failed
    LogWriteFailed { reason: String },
    /// Log rotation failed
    LogRotationFailed { reason: String },
    /// Log format invalid
    InvalidLogFormat { reason: String },
    /// Audit buffer full
    BufferFull,
    /// Audit sink unavailable
    SinkUnavailable { sink_name: String },
    /// Audit configuration error
    ConfigurationError { reason: String },
}

/// Permission checking errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionError {
    /// Insufficient permissions
    InsufficientPermissions { required: Vec<String>, available: Vec<String> },
    /// Invalid permission format
    InvalidPermission { permission: String, reason: String },
    /// Permission context not found
    ContextNotFound { context_id: String },
    /// Permission evaluation failed
    EvaluationFailed { reason: String },
    /// Permission database error
    DatabaseError { reason: String },
}

/// Isolation management errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolationError {
    /// Cross-dot interference detected
    CrossDotInterference { source_dot: String, target_dot: String, interference_type: String },
    /// Isolation boundary violation
    BoundaryViolation { dot_id: String, violation_type: String },
    /// Isolation context creation failed
    ContextCreationFailed { dot_id: String, reason: String },
    /// Isolation level not supported
    UnsupportedIsolationLevel { requested: SecurityLevel, supported: Vec<SecurityLevel> },
    /// Resource sharing violation
    ResourceSharingViolation { resource_type: String, reason: String },
}

/// Security policy errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// Policy not found
    PolicyNotFound { policy_id: String },
    /// Policy evaluation failed
    EvaluationFailed { policy_id: String, reason: String },
    /// Policy configuration invalid
    InvalidConfiguration { policy_id: String, reason: String },
    /// Policy conflict detected
    PolicyConflict { conflicting_policies: Vec<String>, reason: String },
    /// Policy enforcement failed
    EnforcementFailed { policy_id: String, reason: String },
    /// Policy version mismatch
    VersionMismatch { policy_id: String, expected: String, actual: String },
}

// Error Display implementations
impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::CapabilityDenied { opcode_type, reason } => {
                write!(f, "Capability denied for opcode {:?}: {}", opcode_type, reason)
            }
            SecurityError::CapabilityNotFound { opcode_type } => {
                write!(f, "Capability not found for opcode {:?}", opcode_type)
            }
            SecurityError::CapabilityExpired { opcode_type, expired_at } => {
                write!(f, "Capability for opcode {:?} expired at {:?}", opcode_type, expired_at)
            }
            SecurityError::InvalidCapability { reason } => {
                write!(f, "Invalid capability: {}", reason)
            }
            SecurityError::ResourceLimitExceeded(err) => {
                write!(f, "Resource limit exceeded: {}", err)
            }
            SecurityError::PermissionDenied(err) => {
                write!(f, "Permission denied: {}", err)
            }
            SecurityError::IsolationViolation(err) => {
                write!(f, "Isolation violation: {}", err)
            }
            SecurityError::PolicyViolation(err) => {
                write!(f, "Security policy violation: {}", err)
            }
            SecurityError::AuditFailure(err) => {
                write!(f, "Audit logging failed: {}", err)
            }
            SecurityError::InternalError { message } => {
                write!(f, "Internal security error: {}", message)
            }
        }
    }
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::MemoryLimitExceeded { current, limit } => {
                write!(f, "Memory limit exceeded: {} bytes (limit: {} bytes)", current, limit)
            }
            ResourceError::CpuTimeExceeded { current_ms, limit_ms } => {
                write!(f, "CPU time exceeded: {} ms (limit: {} ms)", current_ms, limit_ms)
            }
            ResourceError::InstructionCountExceeded { current, limit } => {
                write!(f, "Instruction count exceeded: {} (limit: {})", current, limit)
            }
            ResourceError::FileDescriptorLimitExceeded { current, limit } => {
                write!(f, "File descriptor limit exceeded: {} (limit: {})", current, limit)
            }
            ResourceError::NetworkBandwidthExceeded { current_bytes, limit_bytes } => {
                write!(f, "Network bandwidth exceeded: {} bytes (limit: {} bytes)", current_bytes, limit_bytes)
            }
            ResourceError::StorageQuotaExceeded { current_bytes, quota_bytes } => {
                write!(f, "Storage quota exceeded: {} bytes (quota: {} bytes)", current_bytes, quota_bytes)
            }
            ResourceError::CallStackDepthExceeded { current, limit } => {
                write!(f, "Call stack depth exceeded: {} (limit: {})", current, limit)
            }
            ResourceError::AllocationFailed { resource_type, reason } => {
                write!(f, "Resource allocation failed for {}: {}", resource_type, reason)
            }
        }
    }
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditError::LogWriteFailed { reason } => {
                write!(f, "Audit log write failed: {}", reason)
            }
            AuditError::LogRotationFailed { reason } => {
                write!(f, "Audit log rotation failed: {}", reason)
            }
            AuditError::InvalidLogFormat { reason } => {
                write!(f, "Invalid audit log format: {}", reason)
            }
            AuditError::BufferFull => {
                write!(f, "Audit buffer is full")
            }
            AuditError::SinkUnavailable { sink_name } => {
                write!(f, "Audit sink '{}' is unavailable", sink_name)
            }
            AuditError::ConfigurationError { reason } => {
                write!(f, "Audit configuration error: {}", reason)
            }
        }
    }
}

impl fmt::Display for PermissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionError::InsufficientPermissions { required, available } => {
                write!(f, "Insufficient permissions. Required: {:?}, Available: {:?}", required, available)
            }
            PermissionError::InvalidPermission { permission, reason } => {
                write!(f, "Invalid permission '{}': {}", permission, reason)
            }
            PermissionError::ContextNotFound { context_id } => {
                write!(f, "Permission context '{}' not found", context_id)
            }
            PermissionError::EvaluationFailed { reason } => {
                write!(f, "Permission evaluation failed: {}", reason)
            }
            PermissionError::DatabaseError { reason } => {
                write!(f, "Permission database error: {}", reason)
            }
        }
    }
}

impl fmt::Display for IsolationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IsolationError::CrossDotInterference {
                source_dot,
                target_dot,
                interference_type,
            } => {
                write!(f, "Cross-dot interference: {} -> {} (type: {})", source_dot, target_dot, interference_type)
            }
            IsolationError::BoundaryViolation { dot_id, violation_type } => {
                write!(f, "Isolation boundary violation in dot '{}': {}", dot_id, violation_type)
            }
            IsolationError::ContextCreationFailed { dot_id, reason } => {
                write!(f, "Failed to create isolation context for dot '{}': {}", dot_id, reason)
            }
            IsolationError::UnsupportedIsolationLevel { requested, supported } => {
                write!(f, "Unsupported isolation level {:?}, supported: {:?}", requested, supported)
            }
            IsolationError::ResourceSharingViolation { resource_type, reason } => {
                write!(f, "Resource sharing violation for {}: {}", resource_type, reason)
            }
        }
    }
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyError::PolicyNotFound { policy_id } => {
                write!(f, "Security policy '{}' not found", policy_id)
            }
            PolicyError::EvaluationFailed { policy_id, reason } => {
                write!(f, "Policy evaluation failed for '{}': {}", policy_id, reason)
            }
            PolicyError::InvalidConfiguration { policy_id, reason } => {
                write!(f, "Invalid configuration for policy '{}': {}", policy_id, reason)
            }
            PolicyError::PolicyConflict { conflicting_policies, reason } => {
                write!(f, "Policy conflict between {:?}: {}", conflicting_policies, reason)
            }
            PolicyError::EnforcementFailed { policy_id, reason } => {
                write!(f, "Policy enforcement failed for '{}': {}", policy_id, reason)
            }
            PolicyError::VersionMismatch { policy_id, expected, actual } => {
                write!(f, "Policy version mismatch for '{}': expected {}, got {}", policy_id, expected, actual)
            }
        }
    }
}

// Standard Error trait implementations
impl std::error::Error for SecurityError {}
impl std::error::Error for ResourceError {}
impl std::error::Error for AuditError {}
impl std::error::Error for PermissionError {}
impl std::error::Error for IsolationError {}
impl std::error::Error for PolicyError {}

// Result type aliases for convenience
pub type SecurityResult<T> = Result<T, SecurityError>;
pub type ResourceResult<T> = Result<T, ResourceError>;
pub type AuditResult<T> = Result<T, AuditError>;
pub type PermissionResult<T> = Result<T, PermissionError>;
pub type IsolationResult<T> = Result<T, IsolationError>;
pub type PolicyResult<T> = Result<T, PolicyError>;
