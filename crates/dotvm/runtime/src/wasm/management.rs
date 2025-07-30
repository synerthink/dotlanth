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

//! WASM Runtime Management Module

use crate::wasm::{WasmError, WasmResult};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Resource limiter for WASM execution
#[derive(Debug, Clone)]
pub struct ResourceLimiter {
    /// Maximum memory in bytes
    pub max_memory: u64,
    /// Maximum execution time
    pub max_time: Duration,
    /// Maximum call depth
    pub max_call_depth: usize,
    /// Maximum instruction count
    pub max_instructions: u64,
}

impl ResourceLimiter {
    /// Create new resource limiter with default limits
    pub fn new() -> Self {
        Self::default()
    }

    /// Create resource limiter with custom limits
    pub fn with_limits(max_memory: u64, max_time: Duration, max_call_depth: usize, max_instructions: u64) -> Self {
        Self {
            max_memory,
            max_time,
            max_call_depth,
            max_instructions,
        }
    }

    /// Check if memory usage is within limits
    pub fn check_memory(&self, current: u64) -> WasmResult<()> {
        if current > self.max_memory {
            return Err(WasmError::ResourceLimitExceeded {
                resource: "memory".to_string(),
                current,
                limit: self.max_memory,
            });
        }
        Ok(())
    }


    /// Check if execution time is within limits
    pub fn check_time(&self, elapsed: Duration) -> WasmResult<()> {
        if elapsed > self.max_time {
            return Err(WasmError::Timeout {
                timeout_ms: self.max_time.as_millis() as u64,
            });
        }
        Ok(())
    }

    /// Check if call depth is within limits
    pub fn check_call_depth(&self, current: usize) -> WasmResult<()> {
        if current > self.max_call_depth {
            return Err(WasmError::StackOverflow { current, max: self.max_call_depth });
        }
        Ok(())
    }

    /// Check if instruction count is within limits
    pub fn check_instructions(&self, current: u64) -> WasmResult<()> {
        if current > self.max_instructions {
            return Err(WasmError::ResourceLimitExceeded {
                resource: "instructions".to_string(),
                current,
                limit: self.max_instructions,
            });
        }
        Ok(())
    }
}

impl Default for ResourceLimiter {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024, // 64MB
            max_time: Duration::from_secs(30),
            max_call_depth: 1000,
            max_instructions: 100_000_000,
        }
    }
}

/// WASM performance monitor
#[derive(Debug)]
pub struct WasmMonitor {
    /// Monitor configuration
    config: MonitorConfig,
    /// Performance metrics
    metrics: PerformanceMetrics,
    /// Resource usage tracking
    resources: ResourceUsage,
}

/// Monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Enable performance tracking
    pub enable_performance: bool,
    /// Enable resource tracking
    pub enable_resources: bool,
    /// Sampling interval for metrics
    pub sampling_interval: Duration,
    /// Maximum metrics history
    pub max_history: usize,
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Execution start time
    pub start_time: Option<Instant>,
    /// Total execution time
    pub execution_time: Duration,
    /// Instructions per second
    pub instructions_per_second: f64,
    /// Memory allocation rate
    pub memory_allocation_rate: f64,
    /// Function call frequency
    pub function_call_frequency: f64,
    /// Average function execution time
    pub avg_function_time: Duration,
}

/// Resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Current memory usage
    pub memory_used: u64,
    /// Peak memory usage
    pub peak_memory: u64,
    /// Current call depth
    pub call_depth: usize,
    /// Peak call depth
    pub peak_call_depth: usize,
    /// Instructions executed
    pub instructions_executed: u64,
    /// Function calls made
    pub function_calls: u64,
    /// Host function calls
    pub host_function_calls: u64,
}

impl WasmMonitor {
    /// Create new monitor with default configuration
    pub fn new() -> Self {
        Self::with_config(MonitorConfig::default())
    }

    /// Create monitor with custom configuration
    pub fn with_config(config: MonitorConfig) -> Self {
        Self {
            config,
            metrics: PerformanceMetrics::default(),
            resources: ResourceUsage::default(),
        }
    }

    /// Start monitoring
    pub fn start(&mut self) {
        if self.config.enable_performance {
            self.metrics.start_time = Some(Instant::now());
        }
    }

    /// Stop monitoring and finalize metrics
    pub fn stop(&mut self) {
        if let Some(start_time) = self.metrics.start_time {
            self.metrics.execution_time = start_time.elapsed();
            self.calculate_rates();
        }
    }

    /// Update resource usage
    pub fn update_resources(&mut self, resources: ResourceUsage) {
        self.resources = resources;

        // Update peak values
        if self.resources.memory_used > self.resources.peak_memory {
            self.resources.peak_memory = self.resources.memory_used;
        }

        if self.resources.call_depth > self.resources.peak_call_depth {
            self.resources.peak_call_depth = self.resources.call_depth;
        }
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// Get current resource usage
    pub fn get_resources(&self) -> &ResourceUsage {
        &self.resources
    }

    /// Calculate performance rates
    fn calculate_rates(&mut self) {
        let elapsed_secs = self.metrics.execution_time.as_secs_f64();

        if elapsed_secs > 0.0 {
            self.metrics.instructions_per_second = self.resources.instructions_executed as f64 / elapsed_secs;


            self.metrics.memory_allocation_rate = self.resources.memory_used as f64 / elapsed_secs;

            self.metrics.function_call_frequency = self.resources.function_calls as f64 / elapsed_secs;

            if self.resources.function_calls > 0 {
                self.metrics.avg_function_time = self.metrics.execution_time / self.resources.function_calls as u32;
            }
        }
    }
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enable_performance: true,
            enable_resources: true,
            sampling_interval: Duration::from_millis(100),
            max_history: 1000,
        }
    }
}

/// Security context for WASM execution
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Security level (0-10)
    pub security_level: u8,
    /// Allowed permissions
    pub permissions: Vec<String>,
    /// Blocked operations
    pub blocked_operations: Vec<String>,
    /// Sandbox enabled
    pub sandbox_enabled: bool,
    /// Resource limits enforced
    pub enforce_limits: bool,
}

/// Security policy
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Maximum security level allowed
    pub max_security_level: u8,
    /// Required permissions
    pub required_permissions: Vec<String>,
    /// Forbidden operations
    pub forbidden_operations: Vec<String>,
    /// Resource limits
    pub resource_limits: HashMap<String, u64>,
}

/// Security level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Development mode (minimal security)
    Development = 0,
    /// Testing mode (basic security)
    Testing = 3,
    /// Production mode (high security)
    Production = 7,
    /// Critical mode (maximum security)
    Critical = 10,
}

impl SecurityContext {
    /// Create new security context
    pub fn new(level: u8) -> Self {
        Self {
            security_level: level,
            permissions: Vec::new(),
            blocked_operations: Vec::new(),
            sandbox_enabled: true,
            enforce_limits: true,
        }
    }

    /// Create security context with level
    pub fn with_level(level: SecurityLevel) -> Self {
        Self::new(level as u8)
    }

    /// Check permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Add permission
    pub fn add_permission(&mut self, permission: String) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }

    /// Remove permission
    pub fn remove_permission(&mut self, permission: &str) {
        self.permissions.retain(|p| p != permission);
    }

    /// Check if operation is blocked
    pub fn is_operation_blocked(&self, operation: &str) -> bool {
        self.blocked_operations.contains(&operation.to_string())
    }

    /// Block operation
    pub fn block_operation(&mut self, operation: String) {
        if !self.blocked_operations.contains(&operation) {
            self.blocked_operations.push(operation);
        }
    }

    /// Unblock operation
    pub fn unblock_operation(&mut self, operation: &str) {
        self.blocked_operations.retain(|op| op != operation);
    }

    /// Validate against policy
    pub fn validate_policy(&self, policy: &SecurityPolicy) -> WasmResult<()> {
        if self.security_level > policy.max_security_level {
            return Err(WasmError::security_violation(format!(
                "Security level {} exceeds maximum {}",
                self.security_level, policy.max_security_level
            )));
        }

        // Check required permissions
        for required_perm in &policy.required_permissions {
            if !self.has_permission(required_perm) {
                return Err(WasmError::security_violation(format!("Missing required permission: {}", required_perm)));
            }
        }

        // Check forbidden operations
        for forbidden_op in &policy.forbidden_operations {
            if !self.is_operation_blocked(forbidden_op) {
                return Err(WasmError::security_violation(format!("Forbidden operation not blocked: {}", forbidden_op)));
            }
        }

        Ok(())
    }

    /// Check if operation is allowed
    pub fn is_operation_allowed(&self, operation: &str) -> bool {
        if self.is_operation_blocked(operation) {
            return false;
        }

        // If permissions list is not empty, operation must be explicitly allowed
        if !self.permissions.is_empty() {
            return self.has_permission(operation);
        }

        true
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self::new(SecurityLevel::Production as u8)
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            max_security_level: SecurityLevel::Critical as u8,
            required_permissions: Vec::new(),
            forbidden_operations: vec!["system.exit".to_string(), "file.write".to_string(), "network.connect".to_string()],
            resource_limits: HashMap::new(),
        }
    }
}

// ================================================================================================
// UNIFIED MANAGEMENT
// ================================================================================================

/// Unified runtime management
#[derive(Debug)]
pub struct RuntimeManager {
    /// Resource limiter
    pub limiter: ResourceLimiter,
    /// Performance monitor
    pub monitor: WasmMonitor,
    /// Security context
    pub security: SecurityContext,
}

impl RuntimeManager {
    /// Create new runtime manager
    pub fn new() -> Self {
        Self {
            limiter: ResourceLimiter::default(),
            monitor: WasmMonitor::new(),
            security: SecurityContext::default(),
        }
    }

    /// Create runtime manager with custom configuration
    pub fn with_config(limiter: ResourceLimiter, monitor_config: MonitorConfig, security: SecurityContext) -> Self {
        Self {
            limiter,
            monitor: WasmMonitor::with_config(monitor_config),
            security,
        }
    }

    /// Start monitoring
    pub fn start_monitoring(&mut self) {
        self.monitor.start();
    }

    /// Stop monitoring
    pub fn stop_monitoring(&mut self) {
        self.monitor.stop();
    }

    /// Check all resource limits
    pub fn check_limits(&self, resources: &ResourceUsage) -> WasmResult<()> {
        self.limiter.check_memory(resources.memory_used)?;
        self.limiter.check_call_depth(resources.call_depth)?;
        self.limiter.check_instructions(resources.instructions_executed)?;
        Ok(())
    }

    /// Update monitoring data
    pub fn update_monitoring(&mut self, resources: ResourceUsage) {
        self.monitor.update_resources(resources);
    }

    /// Check security policy
    pub fn check_security(&self, operation: &str) -> WasmResult<()> {
        if !self.security.is_operation_allowed(operation) {
            return Err(WasmError::security_violation(format!("Operation not allowed: {}", operation)));
        }
        Ok(())
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limiter() {
        let limiter = ResourceLimiter::default();

        // Test memory check
        assert!(limiter.check_memory(1024).is_ok());
        assert!(limiter.check_memory(limiter.max_memory + 1).is_err());

    }

    #[test]
    fn test_monitor() {
        let mut monitor = WasmMonitor::new();
        monitor.start();

        let resources = ResourceUsage {
            memory_used: 1024,
            instructions_executed: 1000,
            function_calls: 10,
            ..Default::default()
        };

        monitor.update_resources(resources);

        // Add small delay to ensure execution time is measurable
        std::thread::sleep(Duration::from_millis(1));

        monitor.stop();

        let metrics = monitor.get_metrics();
        assert!(metrics.execution_time > Duration::ZERO);
    }

    #[test]
    fn test_security_context() {
        let mut ctx = SecurityContext::new(3);
        assert_eq!(ctx.security_level, 3);
        assert!(!ctx.has_permission("test"));

        ctx.add_permission("test".to_string());
        assert!(ctx.has_permission("test"));

        ctx.block_operation("dangerous".to_string());
        assert!(!ctx.is_operation_allowed("dangerous"));
    }

    #[test]
    fn test_runtime_manager() {
        let mut manager = RuntimeManager::new();
        manager.start_monitoring();

        let resources = ResourceUsage {
            memory_used: 1024,
            call_depth: 5,
            instructions_executed: 1000,
            ..Default::default()
        };

        assert!(manager.check_limits(&resources).is_ok());
        manager.update_monitoring(resources);
        manager.stop_monitoring();
    }
}
