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

//! Audit Logger
//!
//! Comprehensive audit logging system for all security-relevant events
//! including opcode calls, permission checks, and policy violations.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::security::errors::{AuditError, AuditResult};
use crate::security::types::{CustomOpcode, DotVMContext, OpcodeResult};

/// Audit logger for security events
#[derive(Debug)]
pub struct AuditLogger {
    /// Event buffer for in-memory storage
    event_buffer: Arc<RwLock<VecDeque<AuditEvent>>>,
    /// Audit sinks for event output
    sinks: Arc<RwLock<HashMap<String, Box<dyn AuditSink + Send + Sync>>>>,
    /// Logger configuration
    config: AuditConfig,
    /// Event statistics
    statistics: Arc<RwLock<AuditStatistics>>,
}

/// Individual audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier
    pub id: String,
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event type and severity
    pub event_type: AuditEventType,
    /// Dot that triggered the event
    pub dot_id: String,
    /// Session identifier
    pub session_id: String,
    /// Event details
    pub details: AuditEventDetails,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Event severity level
    pub severity: AuditSeverity,
}

/// Types of audit events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Opcode execution event
    OpcodeExecution,
    /// Capability check event
    CapabilityCheck,
    /// Permission check event
    PermissionCheck,
    /// Resource limit enforcement
    ResourceLimit,
    /// Isolation violation
    IsolationViolation,
    /// Policy violation
    PolicyViolation,
    /// Security configuration change
    ConfigurationChange,
    /// Authentication event
    Authentication,
    /// Authorization event
    Authorization,
    /// System event
    SystemEvent,
}

/// Audit event severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Detailed audit event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventDetails {
    /// Opcode execution details
    OpcodeExecution {
        opcode_type: String,
        parameters_hash: String,
        result: OpcodeAuditResult,
        resource_consumed: ResourceConsumptionAudit,
        execution_time_ms: u64,
    },
    /// Capability check details
    CapabilityCheck {
        opcode_type: String,
        capability_id: Option<String>,
        check_result: bool,
        failure_reason: Option<String>,
    },
    /// Permission check details
    PermissionCheck {
        requested_permissions: Vec<String>,
        granted_permissions: Vec<String>,
        check_result: bool,
        failure_reason: Option<String>,
    },
    /// Resource limit details
    ResourceLimit {
        resource_type: String,
        current_usage: u64,
        limit: u64,
        enforcement_action: String,
    },
    /// Isolation violation details
    IsolationViolation {
        violation_type: String,
        source_dot: String,
        target_dot: String,
        violation_details: String,
    },
    /// Policy violation details
    PolicyViolation { policy_id: String, violation_type: String, violation_details: String },
    /// Configuration change details
    ConfigurationChange {
        component: String,
        change_type: String,
        old_value: Option<String>,
        new_value: String,
        changed_by: String,
    },
    /// Authentication details
    Authentication {
        user_id: String,
        auth_method: String,
        success: bool,
        failure_reason: Option<String>,
    },
    /// Authorization details
    Authorization {
        user_id: String,
        resource: String,
        action: String,
        success: bool,
        failure_reason: Option<String>,
    },
    /// System event details
    SystemEvent { event_type: String, description: String, system_component: String },
}

/// Opcode execution result for auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcodeAuditResult {
    pub success: bool,
    pub return_value_hash: Option<String>,
    pub errors: Vec<String>,
    pub side_effects: Vec<String>,
}

/// Resource consumption audit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConsumptionAudit {
    pub memory_bytes: u64,
    pub cpu_cycles: u64,
    pub storage_bytes: u64,
    pub network_bytes: u64,
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Maximum events in buffer
    pub max_buffer_size: usize,
    /// Minimum severity to log
    pub min_severity: AuditSeverity,
    /// Enable async logging
    pub async_logging: bool,
    /// Buffer flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Enable event compression
    pub enable_compression: bool,
    /// Log file rotation size in bytes
    pub log_rotation_size: u64,
    /// Maximum log files to keep
    pub max_log_files: u32,
    /// Enable structured logging (JSON)
    pub structured_logging: bool,
}

/// Audit sink trait for different output destinations
pub trait AuditSink: std::fmt::Debug {
    /// Write an audit event to the sink
    fn write_event(&mut self, event: &AuditEvent) -> AuditResult<()>;

    /// Flush any buffered events
    fn flush(&mut self) -> AuditResult<()>;

    /// Get sink configuration
    fn get_config(&self) -> HashMap<String, String>;

    /// Check if sink is healthy
    fn is_healthy(&self) -> bool;
}

/// File-based audit sink
#[derive(Debug)]
pub struct FileAuditSink {
    file_path: PathBuf,
    current_size: u64,
    max_size: u64,
    file_count: u32,
    max_files: u32,
}

/// In-memory audit sink for testing
#[derive(Debug)]
pub struct MemoryAuditSink {
    events: Vec<AuditEvent>,
    max_events: usize,
}

/// Syslog audit sink
#[derive(Debug)]
pub struct SyslogAuditSink {
    facility: String,
    tag: String,
}

/// Audit statistics
#[derive(Debug, Clone, Default)]
pub struct AuditStatistics {
    pub total_events: u64,
    pub events_by_type: HashMap<AuditEventType, u64>,
    pub events_by_severity: HashMap<AuditSeverity, u64>,
    pub events_by_dot: HashMap<String, u64>,
    pub buffer_overflows: u64,
    pub sink_errors: u64,
    pub last_event_time: Option<SystemTime>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_buffer_size: 10000,
            min_severity: AuditSeverity::Info,
            async_logging: true,
            flush_interval_ms: 1000,
            enable_compression: false,
            log_rotation_size: 100 * 1024 * 1024, // 100MB
            max_log_files: 10,
            structured_logging: true,
        }
    }
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new() -> Self {
        Self::with_config(AuditConfig::default())
    }

    /// Create a new audit logger with custom configuration
    pub fn with_config(config: AuditConfig) -> Self {
        Self {
            event_buffer: Arc::new(RwLock::new(VecDeque::new())),
            sinks: Arc::new(RwLock::new(HashMap::new())),
            config,
            statistics: Arc::new(RwLock::new(AuditStatistics::default())),
        }
    }

    /// Log an opcode execution event
    pub fn audit_opcode_call(&self, context: &DotVMContext, opcode: &CustomOpcode, result: &OpcodeResult) -> AuditResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let event = AuditEvent {
            id: self.generate_event_id(),
            timestamp: SystemTime::now(),
            event_type: AuditEventType::OpcodeExecution,
            dot_id: context.dot_id.clone(),
            session_id: context.session_id.clone(),
            details: AuditEventDetails::OpcodeExecution {
                opcode_type: format!("{:?}", opcode.opcode_type),
                parameters_hash: self.hash_parameters(&opcode.parameters),
                result: OpcodeAuditResult {
                    success: result.success,
                    return_value_hash: result.return_value.as_ref().map(|v| self.hash_data(v)),
                    errors: result.errors.clone(),
                    side_effects: result.side_effects.iter().map(|se| format!("{:?}", se)).collect(),
                },
                resource_consumed: ResourceConsumptionAudit {
                    memory_bytes: result.resource_consumed.memory_bytes,
                    cpu_cycles: result.resource_consumed.cpu_cycles,
                    storage_bytes: result.resource_consumed.storage_bytes,
                    network_bytes: result.resource_consumed.network_bytes,
                },
                execution_time_ms: result.execution_time.as_millis() as u64,
            },
            metadata: HashMap::new(),
            severity: if result.success { AuditSeverity::Info } else { AuditSeverity::Warning },
        };

        self.log_event(event)
    }

    /// Log a capability check event
    pub fn audit_capability_check(&self, context: &DotVMContext, opcode_type: &str, capability_id: Option<&str>, success: bool, failure_reason: Option<&str>) -> AuditResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let event = AuditEvent {
            id: self.generate_event_id(),
            timestamp: SystemTime::now(),
            event_type: AuditEventType::CapabilityCheck,
            dot_id: context.dot_id.clone(),
            session_id: context.session_id.clone(),
            details: AuditEventDetails::CapabilityCheck {
                opcode_type: opcode_type.to_string(),
                capability_id: capability_id.map(|s| s.to_string()),
                check_result: success,
                failure_reason: failure_reason.map(|s| s.to_string()),
            },
            metadata: HashMap::new(),
            severity: if success { AuditSeverity::Debug } else { AuditSeverity::Warning },
        };

        self.log_event(event)
    }

    /// Log a permission check event
    pub fn audit_permission_check(&self, context: &DotVMContext, requested_permissions: &[String], granted_permissions: &[String], success: bool, failure_reason: Option<&str>) -> AuditResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let event = AuditEvent {
            id: self.generate_event_id(),
            timestamp: SystemTime::now(),
            event_type: AuditEventType::PermissionCheck,
            dot_id: context.dot_id.clone(),
            session_id: context.session_id.clone(),
            details: AuditEventDetails::PermissionCheck {
                requested_permissions: requested_permissions.to_vec(),
                granted_permissions: granted_permissions.to_vec(),
                check_result: success,
                failure_reason: failure_reason.map(|s| s.to_string()),
            },
            metadata: HashMap::new(),
            severity: if success { AuditSeverity::Debug } else { AuditSeverity::Error },
        };

        self.log_event(event)
    }

    /// Log a resource limit event
    pub fn audit_resource_limit(&self, context: &DotVMContext, resource_type: &str, current_usage: u64, limit: u64, enforcement_action: &str) -> AuditResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let event = AuditEvent {
            id: self.generate_event_id(),
            timestamp: SystemTime::now(),
            event_type: AuditEventType::ResourceLimit,
            dot_id: context.dot_id.clone(),
            session_id: context.session_id.clone(),
            details: AuditEventDetails::ResourceLimit {
                resource_type: resource_type.to_string(),
                current_usage,
                limit,
                enforcement_action: enforcement_action.to_string(),
            },
            metadata: HashMap::new(),
            severity: match enforcement_action {
                "allow" => AuditSeverity::Info,
                "warn" => AuditSeverity::Warning,
                "throttle" => AuditSeverity::Warning,
                "deny" => AuditSeverity::Error,
                "terminate" => AuditSeverity::Critical,
                _ => AuditSeverity::Info,
            },
        };

        self.log_event(event)
    }

    /// Log a security violation event
    pub fn audit_security_violation(&self, context: &DotVMContext, violation_type: &str, details: &str, severity: AuditSeverity) -> AuditResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let (event_type, event_details) = match violation_type {
            "isolation" => (
                AuditEventType::IsolationViolation,
                AuditEventDetails::IsolationViolation {
                    violation_type: violation_type.to_string(),
                    source_dot: context.dot_id.clone(),
                    target_dot: "unknown".to_string(),
                    violation_details: details.to_string(),
                },
            ),
            "policy" => (
                AuditEventType::PolicyViolation,
                AuditEventDetails::PolicyViolation {
                    policy_id: "unknown".to_string(),
                    violation_type: violation_type.to_string(),
                    violation_details: details.to_string(),
                },
            ),
            _ => (
                AuditEventType::SystemEvent,
                AuditEventDetails::SystemEvent {
                    event_type: violation_type.to_string(),
                    description: details.to_string(),
                    system_component: "security".to_string(),
                },
            ),
        };

        let event = AuditEvent {
            id: self.generate_event_id(),
            timestamp: SystemTime::now(),
            event_type,
            dot_id: context.dot_id.clone(),
            session_id: context.session_id.clone(),
            details: event_details,
            metadata: HashMap::new(),
            severity,
        };

        self.log_event(event)
    }

    /// Add an audit sink
    pub fn add_sink(&self, name: String, sink: Box<dyn AuditSink + Send + Sync>) -> AuditResult<()> {
        let mut sinks = self.sinks.write().map_err(|e| AuditError::ConfigurationError {
            reason: format!("Failed to acquire sinks lock: {}", e),
        })?;

        sinks.insert(name, sink);
        Ok(())
    }

    /// Remove an audit sink
    pub fn remove_sink(&self, name: &str) -> AuditResult<()> {
        let mut sinks = self.sinks.write().map_err(|e| AuditError::ConfigurationError {
            reason: format!("Failed to acquire sinks lock: {}", e),
        })?;

        sinks.remove(name);
        Ok(())
    }

    /// Flush all audit sinks
    pub fn flush(&self) -> AuditResult<()> {
        let mut sinks = self.sinks.write().map_err(|e| AuditError::ConfigurationError {
            reason: format!("Failed to acquire sinks lock: {}", e),
        })?;

        for sink in sinks.values_mut() {
            sink.flush()?;
        }

        Ok(())
    }

    /// Get audit statistics
    pub fn get_statistics(&self) -> AuditResult<AuditStatistics> {
        let stats = self.statistics.read().map_err(|e| AuditError::ConfigurationError {
            reason: format!("Failed to acquire statistics lock: {}", e),
        })?;

        Ok(stats.clone())
    }

    /// Clear audit buffer
    pub fn clear_buffer(&self) -> AuditResult<usize> {
        let mut buffer = self.event_buffer.write().map_err(|e| AuditError::LogWriteFailed {
            reason: format!("Failed to acquire buffer lock: {}", e),
        })?;

        let count = buffer.len();
        buffer.clear();
        Ok(count)
    }

    // Private helper methods
    fn log_event(&self, event: AuditEvent) -> AuditResult<()> {
        // Check severity threshold
        if event.severity < self.config.min_severity {
            return Ok(());
        }

        // Update statistics
        self.update_statistics(&event)?;

        // Add to buffer
        self.add_to_buffer(event.clone())?;

        // Write to sinks
        self.write_to_sinks(&event)?;

        Ok(())
    }

    fn add_to_buffer(&self, event: AuditEvent) -> AuditResult<()> {
        let mut buffer = self.event_buffer.write().map_err(|e| AuditError::LogWriteFailed {
            reason: format!("Failed to acquire buffer lock: {}", e),
        })?;

        if buffer.len() >= self.config.max_buffer_size {
            buffer.pop_front(); // Remove oldest event

            // Update overflow counter
            if let Ok(mut stats) = self.statistics.write() {
                stats.buffer_overflows += 1;
            }
        }

        buffer.push_back(event);
        Ok(())
    }

    fn write_to_sinks(&self, event: &AuditEvent) -> AuditResult<()> {
        let mut sinks = self.sinks.write().map_err(|e| AuditError::LogWriteFailed {
            reason: format!("Failed to acquire sinks lock: {}", e),
        })?;

        for (sink_name, sink) in sinks.iter_mut() {
            if let Err(e) = sink.write_event(event) {
                // Log sink error but continue with other sinks
                if let Ok(mut stats) = self.statistics.write() {
                    stats.sink_errors += 1;
                }

                eprintln!("Audit sink '{}' error: {}", sink_name, e);
            }
        }

        Ok(())
    }

    fn update_statistics(&self, event: &AuditEvent) -> AuditResult<()> {
        let mut stats = self.statistics.write().map_err(|e| AuditError::LogWriteFailed {
            reason: format!("Failed to acquire statistics lock: {}", e),
        })?;

        stats.total_events += 1;
        *stats.events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        *stats.events_by_severity.entry(event.severity.clone()).or_insert(0) += 1;
        *stats.events_by_dot.entry(event.dot_id.clone()).or_insert(0) += 1;
        stats.last_event_time = Some(event.timestamp);

        Ok(())
    }

    fn generate_event_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        format!("audit_{:x}", now.as_nanos())
    }

    fn hash_parameters(&self, params: &[u8]) -> String {
        // Simple hash implementation - in production, use a proper hash function
        format!("hash_{:x}", params.iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64)))
    }

    fn hash_data(&self, data: &[u8]) -> String {
        // Simple hash implementation - in production, use a proper hash function
        format!("hash_{:x}", data.iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64)))
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

// Audit sink implementations
impl FileAuditSink {
    pub fn new(file_path: PathBuf, max_size: u64, max_files: u32) -> Self {
        Self {
            file_path,
            current_size: 0,
            max_size,
            file_count: 0,
            max_files,
        }
    }
}

impl AuditSink for FileAuditSink {
    fn write_event(&mut self, event: &AuditEvent) -> AuditResult<()> {
        // In a real implementation, this would write to file
        // For now, just simulate the operation
        let serialized = serde_json::to_string(event).map_err(|e| AuditError::LogWriteFailed {
            reason: format!("Failed to serialize event: {}", e),
        })?;

        self.current_size += serialized.len() as u64;

        // Check if rotation is needed
        if self.current_size > self.max_size {
            self.rotate_log()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> AuditResult<()> {
        // In a real implementation, this would flush file buffers
        Ok(())
    }

    fn get_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert("type".to_string(), "file".to_string());
        config.insert("path".to_string(), self.file_path.to_string_lossy().to_string());
        config.insert("max_size".to_string(), self.max_size.to_string());
        config.insert("max_files".to_string(), self.max_files.to_string());
        config
    }

    fn is_healthy(&self) -> bool {
        true // In a real implementation, check if file is writable
    }
}

impl FileAuditSink {
    fn rotate_log(&mut self) -> AuditResult<()> {
        // In a real implementation, this would rotate log files
        self.current_size = 0;
        self.file_count += 1;

        if self.file_count > self.max_files {
            // Remove oldest log file
            self.file_count = self.max_files;
        }

        Ok(())
    }
}

impl MemoryAuditSink {
    pub fn new(max_events: usize) -> Self {
        Self { events: Vec::new(), max_events }
    }

    pub fn get_events(&self) -> &[AuditEvent] {
        &self.events
    }
}

impl AuditSink for MemoryAuditSink {
    fn write_event(&mut self, event: &AuditEvent) -> AuditResult<()> {
        if self.events.len() >= self.max_events {
            self.events.remove(0); // Remove oldest event
        }
        self.events.push(event.clone());
        Ok(())
    }

    fn flush(&mut self) -> AuditResult<()> {
        // Nothing to flush for memory sink
        Ok(())
    }

    fn get_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert("type".to_string(), "memory".to_string());
        config.insert("max_events".to_string(), self.max_events.to_string());
        config
    }

    fn is_healthy(&self) -> bool {
        true
    }
}

impl SyslogAuditSink {
    pub fn new(facility: String, tag: String) -> Self {
        Self { facility, tag }
    }
}

impl AuditSink for SyslogAuditSink {
    fn write_event(&mut self, event: &AuditEvent) -> AuditResult<()> {
        // In a real implementation, this would write to syslog
        let _message = format!(
            "[{}] {} - {:?}: {}",
            self.tag,
            event.dot_id,
            event.event_type,
            serde_json::to_string(&event.details).unwrap_or_default()
        );
        Ok(())
    }

    fn flush(&mut self) -> AuditResult<()> {
        // Syslog usually doesn't need explicit flushing
        Ok(())
    }

    fn get_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert("type".to_string(), "syslog".to_string());
        config.insert("facility".to_string(), self.facility.clone());
        config.insert("tag".to_string(), self.tag.clone());
        config
    }

    fn is_healthy(&self) -> bool {
        true // In a real implementation, check syslog connection
    }
}
