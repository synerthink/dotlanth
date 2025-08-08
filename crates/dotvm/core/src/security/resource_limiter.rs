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

//! Resource Limiter
//!
//! Enforces resource limits and quotas to prevent denial-of-service attacks
//! and ensure fair resource allocation across different execution contexts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::security::errors::{ResourceError, ResourceResult};
use crate::security::types::{CurrentResourceUsage, DotVMContext, ResourceCost};

/// Resource limiter for enforcing quotas and limits
#[derive(Debug)]
pub struct ResourceLimiter {
    /// Resource limits by context
    limits: Arc<RwLock<HashMap<String, ResourceLimits>>>,
    /// Current resource usage tracking
    usage_tracking: Arc<RwLock<HashMap<String, ResourceUsageTracker>>>,
    /// Global resource pool
    global_pool: Arc<RwLock<GlobalResourcePool>>,
    /// Limiter configuration
    config: ResourceLimiterConfig,
}

/// Resource limits configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Maximum CPU time in milliseconds
    pub max_cpu_time_ms: u64,
    /// Maximum number of instructions
    pub max_instruction_count: u64,
    /// Maximum number of file descriptors
    pub max_file_descriptors: u32,
    /// Maximum network bandwidth in bytes
    pub max_network_bytes: u64,
    /// Maximum storage usage in bytes
    pub max_storage_bytes: u64,
    /// Maximum call stack depth
    pub max_call_stack_depth: u32,
}

/// Current resource usage tracking with history
#[derive(Debug, Clone)]
pub struct ResourceUsageTracker {
    /// Current usage
    pub current: CurrentResourceUsage,
    /// Peak usage recorded
    pub peak: CurrentResourceUsage,
    /// Usage history for rate limiting
    pub history: Vec<ResourceUsageSnapshot>,
    /// Start time of tracking
    pub start_time: Instant,
    /// Last enforcement check
    pub last_check: Instant,
}

/// Resource usage snapshot for historical tracking
#[derive(Debug, Clone)]
pub struct ResourceUsageSnapshot {
    pub timestamp: Instant,
    pub memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub instruction_count: u64,
    pub network_bytes: u64,
    pub storage_bytes: u64,
}

/// Global resource pool for system-wide limits
#[derive(Debug, Clone)]
pub struct GlobalResourcePool {
    /// Total system memory limit
    pub total_memory_limit: u64,
    /// Current memory allocated
    pub current_memory_usage: u64,
    /// Total CPU time limit
    pub total_cpu_limit_ms: u64,
    /// Current CPU time used
    pub current_cpu_usage_ms: u64,
    /// Total network bandwidth limit
    pub total_network_limit: u64,
    /// Current network usage
    pub current_network_usage: u64,
    /// Active execution contexts
    pub active_contexts: u32,
    /// Maximum concurrent contexts
    pub max_concurrent_contexts: u32,
}

/// Resource limiter configuration
#[derive(Debug, Clone)]
pub struct ResourceLimiterConfig {
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limiting window in seconds
    pub rate_limit_window_seconds: u64,
    /// Enable burst allowance
    pub enable_burst_allowance: bool,
    /// Burst multiplier (e.g., 2.0 = allow 2x normal rate for short periods)
    pub burst_multiplier: f64,
    /// History retention duration
    pub history_retention_seconds: u64,
    /// Enforcement check interval in milliseconds
    pub enforcement_check_interval_ms: u64,
    /// Enable graceful degradation
    pub enable_graceful_degradation: bool,
    /// Warning threshold (percentage of limit)
    pub warning_threshold_percent: u8,
}

/// Resource enforcement action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceEnforcementAction {
    /// Allow operation to continue
    Allow,
    /// Issue warning but allow operation
    Warn { message: String },
    /// Throttle the operation
    Throttle { delay_ms: u64 },
    /// Deny the operation
    Deny { reason: String },
    /// Terminate the execution context
    Terminate { reason: String },
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            max_cpu_time_ms: 30 * 1000,          // 30 seconds
            max_instruction_count: 1_000_000,    // 1M instructions
            max_file_descriptors: 100,
            max_network_bytes: 10 * 1024 * 1024, // 10MB
            max_storage_bytes: 50 * 1024 * 1024, // 50MB
            max_call_stack_depth: 1000,
        }
    }
}

impl Default for ResourceLimiterConfig {
    fn default() -> Self {
        Self {
            enable_rate_limiting: true,
            rate_limit_window_seconds: 60, // 1 minute window
            enable_burst_allowance: true,
            burst_multiplier: 2.0,
            history_retention_seconds: 300,     // 5 minutes
            enforcement_check_interval_ms: 100, // Check every 100ms
            enable_graceful_degradation: true,
            warning_threshold_percent: 80, // Warn at 80%
        }
    }
}

impl ResourceLimiter {
    /// Create a new resource limiter
    pub fn new() -> Self {
        Self::with_config(ResourceLimiterConfig::default())
    }

    /// Create a new resource limiter with custom configuration
    pub fn with_config(config: ResourceLimiterConfig) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            usage_tracking: Arc::new(RwLock::new(HashMap::new())),
            global_pool: Arc::new(RwLock::new(GlobalResourcePool::default())),
            config,
        }
    }

    /// Set resource limits for a context
    pub fn set_limits(&self, context_id: String, limits: ResourceLimits) -> ResourceResult<()> {
        let mut limits_map = self.limits.write().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "limits_lock".to_string(),
            reason: format!("Failed to acquire limits lock: {}", e),
        })?;

        limits_map.insert(context_id, limits);
        Ok(())
    }

    /// Enforce resource limits for an operation
    pub fn enforce_resource_limits(&self, context: &DotVMContext, resource_usage: &CurrentResourceUsage) -> ResourceResult<ResourceEnforcementAction> {
        // Get limits for this context
        let limits = self.get_limits(&context.dot_id)?;

        // Check each resource limit
        if let Some(action) = self.check_memory_limit(resource_usage, &limits)? {
            return Ok(action);
        }

        if let Some(action) = self.check_cpu_limit(resource_usage, &limits)? {
            return Ok(action);
        }

        if let Some(action) = self.check_instruction_limit(resource_usage, &limits)? {
            return Ok(action);
        }

        if let Some(action) = self.check_file_descriptor_limit(resource_usage, &limits)? {
            return Ok(action);
        }

        if let Some(action) = self.check_network_limit(resource_usage, &limits)? {
            return Ok(action);
        }

        if let Some(action) = self.check_storage_limit(resource_usage, &limits)? {
            return Ok(action);
        }

        if let Some(action) = self.check_call_stack_limit(resource_usage, &limits)? {
            return Ok(action);
        }

        // Check global resource pool
        if let Some(action) = self.check_global_limits(resource_usage)? {
            return Ok(action);
        }

        // Check rate limits if enabled
        if self.config.enable_rate_limiting {
            if let Some(action) = self.check_rate_limits(&context.dot_id, resource_usage)? {
                return Ok(action);
            }
        }

        Ok(ResourceEnforcementAction::Allow)
    }

    /// Update resource usage for a context
    pub fn update_usage(&self, context_id: &str, resource_cost: &ResourceCost) -> ResourceResult<()> {
        let mut tracking = self.usage_tracking.write().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "tracking_lock".to_string(),
            reason: format!("Failed to acquire tracking lock: {}", e),
        })?;

        let tracker = tracking.entry(context_id.to_string()).or_insert_with(|| ResourceUsageTracker::new());

        // Update current usage
        tracker.current.add_consumption(resource_cost);

        // Update peak usage
        tracker.update_peak();

        // Add to history
        tracker.add_snapshot();

        // Update global pool
        self.update_global_pool(resource_cost)?;

        Ok(())
    }

    /// Get current resource usage for a context
    pub fn get_usage(&self, context_id: &str) -> ResourceResult<Option<CurrentResourceUsage>> {
        let tracking = self.usage_tracking.read().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "tracking_lock".to_string(),
            reason: format!("Failed to acquire tracking lock: {}", e),
        })?;

        Ok(tracking.get(context_id).map(|tracker| tracker.current.clone()))
    }

    /// Get resource usage statistics for a context
    pub fn get_usage_statistics(&self, context_id: &str) -> ResourceResult<Option<ResourceUsageStatistics>> {
        let tracking = self.usage_tracking.read().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "tracking_lock".to_string(),
            reason: format!("Failed to acquire tracking lock: {}", e),
        })?;

        if let Some(tracker) = tracking.get(context_id) {
            let stats = ResourceUsageStatistics {
                current: tracker.current.clone(),
                peak: tracker.peak.clone(),
                average: tracker.calculate_average(),
                runtime_seconds: tracker.start_time.elapsed().as_secs(),
                total_snapshots: tracker.history.len(),
            };
            Ok(Some(stats))
        } else {
            Ok(None)
        }
    }

    /// Reset resource usage for a context
    pub fn reset_usage(&self, context_id: &str) -> ResourceResult<()> {
        let mut tracking = self.usage_tracking.write().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "tracking_lock".to_string(),
            reason: format!("Failed to acquire tracking lock: {}", e),
        })?;

        if let Some(tracker) = tracking.get_mut(context_id) {
            tracker.current.reset();
        }

        Ok(())
    }

    /// Clean up expired usage history
    pub fn cleanup_expired_history(&self) -> ResourceResult<usize> {
        let mut tracking = self.usage_tracking.write().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "tracking_lock".to_string(),
            reason: format!("Failed to acquire tracking lock: {}", e),
        })?;

        let retention_duration = Duration::from_secs(self.config.history_retention_seconds);
        let cutoff = Instant::now() - retention_duration;
        let mut cleaned_count = 0;

        for tracker in tracking.values_mut() {
            let original_len = tracker.history.len();
            tracker.history.retain(|snapshot| snapshot.timestamp > cutoff);
            cleaned_count += original_len - tracker.history.len();
        }

        Ok(cleaned_count)
    }

    // Private helper methods
    fn get_limits(&self, context_id: &str) -> ResourceResult<ResourceLimits> {
        let limits = self.limits.read().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "limits_lock".to_string(),
            reason: format!("Failed to acquire limits lock: {}", e),
        })?;

        Ok(limits.get(context_id).cloned().unwrap_or_default())
    }

    fn check_memory_limit(&self, usage: &CurrentResourceUsage, limits: &ResourceLimits) -> ResourceResult<Option<ResourceEnforcementAction>> {
        if usage.memory_bytes > limits.max_memory_bytes {
            return Ok(Some(ResourceEnforcementAction::Deny {
                reason: format!("Memory limit exceeded: {} bytes (limit: {} bytes)", usage.memory_bytes, limits.max_memory_bytes),
            }));
        }

        if self.config.enable_graceful_degradation {
            let warning_threshold = (limits.max_memory_bytes * self.config.warning_threshold_percent as u64) / 100;
            if usage.memory_bytes > warning_threshold {
                return Ok(Some(ResourceEnforcementAction::Warn {
                    message: format!(
                        "Memory usage warning: {} bytes ({}% of limit)",
                        usage.memory_bytes,
                        (usage.memory_bytes * 100) / limits.max_memory_bytes
                    ),
                }));
            }
        }

        Ok(None)
    }

    fn check_cpu_limit(&self, usage: &CurrentResourceUsage, limits: &ResourceLimits) -> ResourceResult<Option<ResourceEnforcementAction>> {
        if usage.cpu_time_ms > limits.max_cpu_time_ms {
            return Ok(Some(ResourceEnforcementAction::Terminate {
                reason: format!("CPU time limit exceeded: {} ms (limit: {} ms)", usage.cpu_time_ms, limits.max_cpu_time_ms),
            }));
        }

        Ok(None)
    }

    fn check_instruction_limit(&self, usage: &CurrentResourceUsage, limits: &ResourceLimits) -> ResourceResult<Option<ResourceEnforcementAction>> {
        if usage.instruction_count > limits.max_instruction_count {
            return Ok(Some(ResourceEnforcementAction::Terminate {
                reason: format!("Instruction count limit exceeded: {} (limit: {})", usage.instruction_count, limits.max_instruction_count),
            }));
        }

        Ok(None)
    }

    fn check_file_descriptor_limit(&self, usage: &CurrentResourceUsage, limits: &ResourceLimits) -> ResourceResult<Option<ResourceEnforcementAction>> {
        if usage.file_descriptors > limits.max_file_descriptors {
            return Ok(Some(ResourceEnforcementAction::Deny {
                reason: format!("File descriptor limit exceeded: {} (limit: {})", usage.file_descriptors, limits.max_file_descriptors),
            }));
        }

        Ok(None)
    }

    fn check_network_limit(&self, usage: &CurrentResourceUsage, limits: &ResourceLimits) -> ResourceResult<Option<ResourceEnforcementAction>> {
        if usage.network_bytes > limits.max_network_bytes {
            return Ok(Some(ResourceEnforcementAction::Throttle {
                delay_ms: 100, // Throttle network operations
            }));
        }

        Ok(None)
    }

    fn check_storage_limit(&self, usage: &CurrentResourceUsage, limits: &ResourceLimits) -> ResourceResult<Option<ResourceEnforcementAction>> {
        if usage.storage_bytes > limits.max_storage_bytes {
            return Ok(Some(ResourceEnforcementAction::Deny {
                reason: format!("Storage limit exceeded: {} bytes (limit: {} bytes)", usage.storage_bytes, limits.max_storage_bytes),
            }));
        }

        Ok(None)
    }

    fn check_call_stack_limit(&self, usage: &CurrentResourceUsage, limits: &ResourceLimits) -> ResourceResult<Option<ResourceEnforcementAction>> {
        if usage.call_stack_depth > limits.max_call_stack_depth {
            return Ok(Some(ResourceEnforcementAction::Deny {
                reason: format!("Call stack depth limit exceeded: {} (limit: {})", usage.call_stack_depth, limits.max_call_stack_depth),
            }));
        }

        Ok(None)
    }

    fn check_global_limits(&self, _usage: &CurrentResourceUsage) -> ResourceResult<Option<ResourceEnforcementAction>> {
        let global_pool = self.global_pool.read().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "global_pool_lock".to_string(),
            reason: format!("Failed to acquire global pool lock: {}", e),
        })?;

        if global_pool.current_memory_usage > global_pool.total_memory_limit {
            return Ok(Some(ResourceEnforcementAction::Deny {
                reason: "Global memory limit exceeded".to_string(),
            }));
        }

        if global_pool.active_contexts >= global_pool.max_concurrent_contexts {
            return Ok(Some(ResourceEnforcementAction::Deny {
                reason: "Maximum concurrent contexts limit exceeded".to_string(),
            }));
        }

        Ok(None)
    }

    fn check_rate_limits(&self, _context_id: &str, _usage: &CurrentResourceUsage) -> ResourceResult<Option<ResourceEnforcementAction>> {
        // Rate limiting implementation would check usage patterns over time
        // For now, return None (no rate limiting action)
        Ok(None)
    }

    fn update_global_pool(&self, resource_cost: &ResourceCost) -> ResourceResult<()> {
        let mut global_pool = self.global_pool.write().map_err(|e| ResourceError::AllocationFailed {
            resource_type: "global_pool_lock".to_string(),
            reason: format!("Failed to acquire global pool lock: {}", e),
        })?;

        global_pool.current_memory_usage += resource_cost.memory_bytes;
        global_pool.current_cpu_usage_ms += resource_cost.execution_time_ms;
        global_pool.current_network_usage += resource_cost.network_bytes;

        Ok(())
    }
}

impl Default for ResourceLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for GlobalResourcePool {
    fn default() -> Self {
        Self {
            total_memory_limit: 1024 * 1024 * 1024, // 1GB
            current_memory_usage: 0,
            total_cpu_limit_ms: 60 * 60 * 1000, // 1 hour
            current_cpu_usage_ms: 0,
            total_network_limit: 100 * 1024 * 1024, // 100MB
            current_network_usage: 0,
            active_contexts: 0,
            max_concurrent_contexts: 1000,
        }
    }
}

impl ResourceUsageTracker {
    fn new() -> Self {
        Self {
            current: Default::default(),
            peak: Default::default(),
            history: Vec::new(),
            start_time: Instant::now(),
            last_check: Instant::now(),
        }
    }

    fn update_peak(&mut self) {
        if self.current.memory_bytes > self.peak.memory_bytes {
            self.peak.memory_bytes = self.current.memory_bytes;
        }
        if self.current.cpu_time_ms > self.peak.cpu_time_ms {
            self.peak.cpu_time_ms = self.current.cpu_time_ms;
        }
        if self.current.instruction_count > self.peak.instruction_count {
            self.peak.instruction_count = self.current.instruction_count;
        }
        if self.current.network_bytes > self.peak.network_bytes {
            self.peak.network_bytes = self.current.network_bytes;
        }
        if self.current.storage_bytes > self.peak.storage_bytes {
            self.peak.storage_bytes = self.current.storage_bytes;
        }
    }

    fn add_snapshot(&mut self) {
        let snapshot = ResourceUsageSnapshot {
            timestamp: Instant::now(),
            memory_bytes: self.current.memory_bytes,
            cpu_time_ms: self.current.cpu_time_ms,
            instruction_count: self.current.instruction_count,
            network_bytes: self.current.network_bytes,
            storage_bytes: self.current.storage_bytes,
        };
        self.history.push(snapshot);
    }

    fn calculate_average(&self) -> CurrentResourceUsage {
        if self.history.is_empty() {
            return Default::default();
        }

        let count = self.history.len() as u64;
        let mut avg = CurrentResourceUsage::default();

        for snapshot in &self.history {
            avg.memory_bytes += snapshot.memory_bytes;
            avg.cpu_time_ms += snapshot.cpu_time_ms;
            avg.instruction_count += snapshot.instruction_count;
            avg.network_bytes += snapshot.network_bytes;
            avg.storage_bytes += snapshot.storage_bytes;
        }

        avg.memory_bytes /= count;
        avg.cpu_time_ms /= count;
        avg.instruction_count /= count;
        avg.network_bytes /= count;
        avg.storage_bytes /= count;

        avg
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsageStatistics {
    pub current: CurrentResourceUsage,
    pub peak: CurrentResourceUsage,
    pub average: CurrentResourceUsage,
    pub runtime_seconds: u64,
    pub total_snapshots: usize,
}

/// Resource usage for external API compatibility
pub type ResourceUsage = CurrentResourceUsage;
