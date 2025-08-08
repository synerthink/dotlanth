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

//! Security Sandbox
//!
//! Main orchestrator for the capability-based security system.
//! Integrates all security components to provide comprehensive
//! protection for custom opcode execution.

use std::sync::Arc;
use std::time::SystemTime;

use crate::security::audit_logger::AuditSeverity;
use crate::security::errors::{SecurityError, SecurityResult};
use crate::security::permission_checker::Permission;
use crate::security::policy_enforcer::PolicyEnforcementResult;
use crate::security::resource_limiter::{ResourceEnforcementAction, ResourceUsage};
use crate::security::types::{CustomOpcode, DotVMContext, OpcodeResult, SecurityLevel};
use crate::security::{AuditLogger, CapabilityManager, IsolationManager, PermissionChecker, PolicyEnforcer, ResourceLimiter};

/// Main security sandbox orchestrating all security components
#[derive(Debug)]
pub struct SecuritySandbox {
    /// Capability-based security manager
    pub capability_manager: Arc<CapabilityManager>,
    /// Resource limits enforcer
    pub resource_limiter: Arc<ResourceLimiter>,
    /// Security audit logger
    pub audit_logger: Arc<AuditLogger>,
    /// Permission checker
    pub permission_checker: Arc<PermissionChecker>,
    /// Isolation manager
    pub isolation_manager: Arc<IsolationManager>,
    /// Security policy enforcer
    pub policy_enforcer: Arc<PolicyEnforcer>,
    /// Sandbox configuration
    config: SecuritySandboxConfig,
}

/// Security sandbox configuration
#[derive(Debug, Clone)]
pub struct SecuritySandboxConfig {
    /// Enable the entire security sandbox
    pub enabled: bool,
    /// Enable capability checking
    pub enable_capability_checking: bool,
    /// Enable resource limiting
    pub enable_resource_limiting: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Enable permission checking
    pub enable_permission_checking: bool,
    /// Enable isolation enforcement
    pub enable_isolation: bool,
    /// Enable policy enforcement
    pub enable_policy_enforcement: bool,
    /// Fail-safe mode (allow or deny when checks fail)
    pub fail_safe_mode: FailSafeMode,
    /// Default security level for new contexts
    pub default_security_level: SecurityLevel,
    /// Enable comprehensive security checks
    pub comprehensive_checks: bool,
    /// Security check timeout in milliseconds
    pub check_timeout_ms: u64,
}

/// Fail-safe behavior when security checks fail
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailSafeMode {
    /// Allow execution when checks fail (less secure but more available)
    AllowOnFailure,
    /// Deny execution when checks fail (more secure but less available)
    DenyOnFailure,
}

/// Comprehensive security check result
#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    /// Overall result (allowed or denied)
    pub allowed: bool,
    /// Capability check result
    pub capability_result: Option<SecurityResult<()>>,
    /// Resource limit check result
    pub resource_result: Option<SecurityResult<ResourceEnforcementAction>>,
    /// Permission check result
    pub permission_result: Option<SecurityResult<Vec<Permission>>>,
    /// Isolation check result
    pub isolation_result: Option<SecurityResult<()>>,
    /// Policy enforcement result
    pub policy_result: Option<SecurityResult<PolicyEnforcementResult>>,
    /// Security violations detected
    pub violations: Vec<SecurityViolation>,
    /// Security warnings issued
    pub warnings: Vec<SecurityWarning>,
    /// Total check duration
    pub check_duration_ms: u64,
}

/// Security violation details
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    pub violation_type: String,
    pub severity: SecurityViolationSeverity,
    pub description: String,
    pub timestamp: SystemTime,
    pub context: String,
}

/// Security warning details
#[derive(Debug, Clone)]
pub struct SecurityWarning {
    pub warning_type: String,
    pub description: String,
    pub timestamp: SystemTime,
    pub context: String,
}

/// Security violation severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for SecuritySandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_capability_checking: true,
            enable_resource_limiting: true,
            enable_audit_logging: true,
            enable_permission_checking: true,
            enable_isolation: true,
            enable_policy_enforcement: true,
            fail_safe_mode: FailSafeMode::DenyOnFailure, // Secure by default
            default_security_level: SecurityLevel::Standard,
            comprehensive_checks: true,
            check_timeout_ms: 100, // 100ms - much faster for normal operations
        }
    }
}

impl SecuritySandbox {
    /// Create a new security sandbox with default configuration
    pub fn new() -> Self {
        Self::with_config(SecuritySandboxConfig::default())
    }

    /// Create a new security sandbox with custom configuration
    pub fn with_config(config: SecuritySandboxConfig) -> Self {
        Self {
            capability_manager: Arc::new(CapabilityManager::new()),
            resource_limiter: Arc::new(ResourceLimiter::new()),
            audit_logger: Arc::new(AuditLogger::new()),
            permission_checker: Arc::new(PermissionChecker::new()),
            isolation_manager: Arc::new(IsolationManager::new()),
            policy_enforcer: Arc::new(PolicyEnforcer::new()),
            config,
        }
    }

    /// Create a security sandbox with custom components
    pub fn with_components(
        capability_manager: Arc<CapabilityManager>,
        resource_limiter: Arc<ResourceLimiter>,
        audit_logger: Arc<AuditLogger>,
        permission_checker: Arc<PermissionChecker>,
        isolation_manager: Arc<IsolationManager>,
        policy_enforcer: Arc<PolicyEnforcer>,
        config: SecuritySandboxConfig,
    ) -> Self {
        Self {
            capability_manager,
            resource_limiter,
            audit_logger,
            permission_checker,
            isolation_manager,
            policy_enforcer,
            config,
        }
    }

    /// Check capability for an opcode execution
    pub fn check_capability(&self, context: &DotVMContext, opcode: &CustomOpcode) -> SecurityResult<()> {
        if !self.config.enabled || !self.config.enable_capability_checking {
            return Ok(());
        }

        // Perform capability check
        let result = self.capability_manager.check_capability(context, opcode);

        // Log the capability check
        if self.config.enable_audit_logging {
            let success = result.is_ok();
            let failure_reason = result.as_ref().err().map(|e| e.to_string());

            if let Err(audit_err) = self.audit_logger.audit_capability_check(
                context,
                &format!("{:?}", opcode.opcode_type),
                None, // capability_id would be determined from successful check
                success,
                failure_reason.as_deref(),
            ) {
                // Log audit failure but don't fail the operation
                eprintln!("Audit logging failed: {}", audit_err);
            }
        }

        result
    }

    /// Enforce resource limits for an operation
    pub fn enforce_resource_limits(&self, context: &DotVMContext, resource_usage: &ResourceUsage) -> SecurityResult<ResourceEnforcementAction> {
        if !self.config.enabled || !self.config.enable_resource_limiting {
            return Ok(ResourceEnforcementAction::Allow);
        }

        // Perform resource limit enforcement
        let result = self.resource_limiter.enforce_resource_limits(context, resource_usage).map_err(SecurityError::ResourceLimitExceeded);

        // Log the resource enforcement
        if self.config.enable_audit_logging {
            let default_action = ResourceEnforcementAction::Deny { reason: "Unknown error".to_string() };
            let action = result.as_ref().unwrap_or(&default_action);

            if let Err(audit_err) = self.audit_logger.audit_resource_limit(
                context,
                "general", // resource type would be more specific in practice
                resource_usage.memory_bytes,
                100 * 1024 * 1024, // example limit
                &format!("{:?}", action),
            ) {
                eprintln!("Audit logging failed: {}", audit_err);
            }
        }

        result
    }

    /// Audit an opcode call
    pub fn audit_opcode_call(&self, context: &DotVMContext, opcode: &CustomOpcode, result: &OpcodeResult) {
        if !self.config.enabled || !self.config.enable_audit_logging {
            return;
        }

        if let Err(audit_err) = self.audit_logger.audit_opcode_call(context, opcode, result) {
            eprintln!("Audit logging failed: {}", audit_err);
        }
    }

    /// Perform comprehensive security check for an opcode execution
    pub fn comprehensive_security_check(
        &self,
        context: &DotVMContext,
        opcode: &CustomOpcode,
        required_permissions: &[Permission],
        resource_usage: &ResourceUsage,
    ) -> SecurityResult<SecurityCheckResult> {
        if !self.config.enabled {
            return Ok(SecurityCheckResult {
                allowed: true,
                capability_result: None,
                resource_result: None,
                permission_result: None,
                isolation_result: None,
                policy_result: None,
                violations: Vec::new(),
                warnings: Vec::new(),
                check_duration_ms: 0,
            });
        }

        let start_time = std::time::Instant::now(); // Use Instant for performance measurement
        let current_time = SystemTime::now(); // Single SystemTime call for all checks
        let mut result = SecurityCheckResult {
            allowed: true,
            capability_result: None,
            resource_result: None,
            permission_result: None,
            isolation_result: None,
            policy_result: None,
            violations: Vec::new(),
            warnings: Vec::new(),
            check_duration_ms: 0,
        };

        // 1. Check capabilities (most critical, fail fast)
        if self.config.enable_capability_checking {
            let capability_result = self.check_capability(context, opcode);
            if capability_result.is_err() {
                result.allowed = false;
                result.violations.push(SecurityViolation {
                    violation_type: "capability".to_string(),
                    severity: SecurityViolationSeverity::High,
                    description: format!("Capability check failed: {:?}", capability_result),
                    timestamp: current_time,
                    context: context.dot_id.clone(),
                });
                // Early exit for capability failure - most critical check
                result.capability_result = Some(capability_result);
                result.check_duration_ms = start_time.elapsed().as_millis() as u64;
                return Ok(result);
            }
            result.capability_result = Some(capability_result);
        }

        // 2. Check resource limits
        if self.config.enable_resource_limiting {
            let resource_result = self.enforce_resource_limits(context, resource_usage);
            match &resource_result {
                Ok(ResourceEnforcementAction::Deny { reason }) => {
                    result.allowed = false;
                    result.violations.push(SecurityViolation {
                        violation_type: "resource_limit".to_string(),
                        severity: SecurityViolationSeverity::High,
                        description: reason.clone(),
                        timestamp: current_time,
                        context: context.dot_id.clone(),
                    });
                }
                Ok(ResourceEnforcementAction::Terminate { reason }) => {
                    result.allowed = false;
                    result.violations.push(SecurityViolation {
                        violation_type: "resource_limit".to_string(),
                        severity: SecurityViolationSeverity::Critical,
                        description: reason.clone(),
                        timestamp: current_time,
                        context: context.dot_id.clone(),
                    });
                    // Early exit for terminate action - critical violation
                    result.resource_result = Some(resource_result);
                    result.check_duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
                Ok(ResourceEnforcementAction::Warn { message }) => {
                    result.warnings.push(SecurityWarning {
                        warning_type: "resource_usage".to_string(),
                        description: message.clone(),
                        timestamp: current_time,
                        context: context.dot_id.clone(),
                    });
                }
                Ok(ResourceEnforcementAction::Throttle { .. }) => {
                    result.warnings.push(SecurityWarning {
                        warning_type: "resource_throttling".to_string(),
                        description: "Operation is being throttled due to resource usage".to_string(),
                        timestamp: current_time,
                        context: context.dot_id.clone(),
                    });
                }
                Err(_) => {
                    result.allowed = false;
                    result.violations.push(SecurityViolation {
                        violation_type: "resource_limit_error".to_string(),
                        severity: SecurityViolationSeverity::Medium,
                        description: "Resource limit check failed".to_string(),
                        timestamp: current_time,
                        context: context.dot_id.clone(),
                    });
                }
                _ => {}
            }
            result.resource_result = Some(resource_result);
        }

        // 3. Check permissions
        if self.config.enable_permission_checking {
            let permission_result = self.permission_checker.check_permission(context, opcode, required_permissions).map_err(SecurityError::PermissionDenied);

            if permission_result.is_err() {
                result.allowed = false;
                result.violations.push(SecurityViolation {
                    violation_type: "permission".to_string(),
                    severity: SecurityViolationSeverity::High,
                    description: "Permission check failed".to_string(),
                    timestamp: current_time,
                    context: context.dot_id.clone(),
                });
            }
            result.permission_result = Some(permission_result);
        }

        // 4. Check isolation boundaries
        if self.config.enable_isolation {
            let isolation_result = self.isolation_manager.validate_isolation_boundaries(context, opcode).map_err(SecurityError::IsolationViolation);

            if isolation_result.is_err() {
                result.allowed = false;
                result.violations.push(SecurityViolation {
                    violation_type: "isolation".to_string(),
                    severity: SecurityViolationSeverity::Critical,
                    description: "Isolation boundary violation".to_string(),
                    timestamp: current_time,
                    context: context.dot_id.clone(),
                });
                // Early exit for isolation violation - critical security issue
                result.isolation_result = Some(isolation_result);
                result.check_duration_ms = start_time.elapsed().as_millis() as u64;
                return Ok(result);
            }
            result.isolation_result = Some(isolation_result);
        }

        // 5. Enforce security policies
        if self.config.enable_policy_enforcement {
            let policy_result = self.policy_enforcer.enforce_policies(context, opcode, None).map_err(SecurityError::PolicyViolation);

            match &policy_result {
                Ok(policy_enforcement) => {
                    if !policy_enforcement.allowed {
                        result.allowed = false;
                        for violated_policy in &policy_enforcement.violated_policies {
                            result.violations.push(SecurityViolation {
                                violation_type: "policy".to_string(),
                                severity: SecurityViolationSeverity::High,
                                description: format!("Policy violation: {}", violated_policy),
                                timestamp: current_time,
                                context: context.dot_id.clone(),
                            });
                        }
                    }
                }
                Err(_) => {
                    result.allowed = false;
                    result.violations.push(SecurityViolation {
                        violation_type: "policy_error".to_string(),
                        severity: SecurityViolationSeverity::Medium,
                        description: "Policy enforcement check failed".to_string(),
                        timestamp: current_time,
                        context: context.dot_id.clone(),
                    });
                }
            }
            result.policy_result = Some(policy_result);
        }

        // Calculate check duration using Instant for accuracy
        result.check_duration_ms = start_time.elapsed().as_millis() as u64;

        // Apply fail-safe mode if there were any violations
        if !result.violations.is_empty() && self.config.fail_safe_mode == FailSafeMode::AllowOnFailure {
            result.allowed = true;
            result.warnings.push(SecurityWarning {
                warning_type: "fail_safe".to_string(),
                description: "Operation allowed due to fail-safe mode despite violations".to_string(),
                timestamp: current_time,
                context: context.dot_id.clone(),
            });
        }

        // Log security violations
        if self.config.enable_audit_logging && !result.violations.is_empty() {
            for violation in &result.violations {
                let severity = match violation.severity {
                    SecurityViolationSeverity::Low => AuditSeverity::Info,
                    SecurityViolationSeverity::Medium => AuditSeverity::Warning,
                    SecurityViolationSeverity::High => AuditSeverity::Error,
                    SecurityViolationSeverity::Critical => AuditSeverity::Critical,
                };

                if let Err(audit_err) = self.audit_logger.audit_security_violation(context, &violation.violation_type, &violation.description, severity) {
                    eprintln!("Failed to audit security violation: {}", audit_err);
                }
            }
        }

        Ok(result)
    }

    /// Initialize security context for a new dot
    pub fn initialize_dot_security_context(&self, dot_id: String, security_level: SecurityLevel) -> SecurityResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Initialize isolation context
        if self.config.enable_isolation {
            self.isolation_manager
                .create_isolation_context(dot_id.clone(), security_level.clone())
                .map_err(SecurityError::IsolationViolation)?;
        }

        // Set up default resource limits
        if self.config.enable_resource_limiting {
            let default_limits = crate::security::resource_limiter::ResourceLimits::default();
            self.resource_limiter.set_limits(dot_id.clone(), default_limits).map_err(SecurityError::ResourceLimitExceeded)?;
        }

        // Grant default capabilities based on security level
        if self.config.enable_capability_checking {
            let default_templates = crate::security::capability_manager::create_default_templates();
            for template in default_templates {
                if self.should_grant_template_for_security_level(&template.name, &security_level) {
                    for opcode_type in &template.opcode_types {
                        if let Ok(capability) = self.capability_manager.create_from_template(&template.name, opcode_type.clone(), "system".to_string()) {
                            if let Err(err) = self.capability_manager.grant_capability(dot_id.clone(), capability, "system".to_string()) {
                                eprintln!("Failed to grant default capability: {}", err);
                            }
                        }
                    }
                }
            }
        }

        // Grant default permissions based on security level
        if self.config.enable_permission_checking {
            let default_templates = crate::security::permission_checker::create_default_permission_templates();
            for template in default_templates {
                if self.should_grant_template_for_security_level(&template.name, &security_level) {
                    if let Err(err) = self.permission_checker.create_from_template(&template.name, dot_id.clone(), "system".to_string()) {
                        eprintln!("Failed to grant default permissions: {}", err);
                    }
                }
            }
        }

        // Add default security policies
        if self.config.enable_policy_enforcement {
            let default_policies = crate::security::policy_enforcer::create_default_security_policies();
            for policy in default_policies {
                if let Err(err) = self.policy_enforcer.add_policy(policy) {
                    eprintln!("Failed to add default policy: {}", err);
                }
            }
        }

        Ok(())
    }

    /// Clean up security context for a dot
    pub fn cleanup_dot_security_context(&self, dot_id: &str) -> SecurityResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Reset resource usage
        if self.config.enable_resource_limiting {
            if let Err(err) = self.resource_limiter.reset_usage(dot_id) {
                eprintln!("Failed to reset resource usage: {}", err);
            }
        }

        // Note: Other cleanup would be implemented as needed
        // For now, we don't remove capabilities/permissions as they might be reused

        Ok(())
    }

    /// Get comprehensive security statistics
    pub fn get_security_statistics(&self, dot_id: &str) -> SecurityResult<SecurityStatistics> {
        let mut stats = SecurityStatistics {
            dot_id: dot_id.to_string(),
            capability_stats: None,
            resource_stats: None,
            permission_stats: None,
            isolation_stats: None,
            policy_stats: None,
            audit_stats: None,
        };

        if self.config.enable_capability_checking {
            stats.capability_stats = self.capability_manager.get_usage_statistics(dot_id).ok();
        }

        if self.config.enable_resource_limiting {
            stats.resource_stats = self.resource_limiter.get_usage_statistics(dot_id).ok().flatten();
        }

        if self.config.enable_permission_checking {
            stats.permission_stats = self.permission_checker.get_statistics().ok();
        }

        if self.config.enable_isolation {
            stats.isolation_stats = self.isolation_manager.get_isolation_statistics(dot_id).ok();
        }

        if self.config.enable_policy_enforcement {
            stats.policy_stats = self.policy_enforcer.get_enforcement_statistics().ok();
        }

        if self.config.enable_audit_logging {
            stats.audit_stats = self.audit_logger.get_statistics().ok();
        }

        Ok(stats)
    }

    // Private helper methods
    fn should_grant_template_for_security_level(&self, template_name: &str, security_level: &SecurityLevel) -> bool {
        match (template_name, security_level) {
            ("arithmetic_basic", _) => true, // All levels get basic arithmetic
            ("database_read", SecurityLevel::Standard | SecurityLevel::High | SecurityLevel::Maximum) => true,
            ("database_read", SecurityLevel::Development) => true,
            ("system_admin", SecurityLevel::Maximum) => true,
            ("system_admin", SecurityLevel::Custom { .. }) => true, // Let custom decide
            _ => false,
        }
    }
}

impl Default for SecuritySandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive security statistics
#[derive(Debug, Clone)]
pub struct SecurityStatistics {
    pub dot_id: String,
    pub capability_stats: Option<crate::security::capability_manager::CapabilityStatistics>,
    pub resource_stats: Option<crate::security::resource_limiter::ResourceUsageStatistics>,
    pub permission_stats: Option<crate::security::permission_checker::PermissionStatistics>,
    pub isolation_stats: Option<crate::security::isolation_manager::IsolationStatistics>,
    pub policy_stats: Option<crate::security::policy_enforcer::PolicyEnforcementStatistics>,
    pub audit_stats: Option<crate::security::audit_logger::AuditStatistics>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::types::{OpcodeArchitecture, OpcodeCategory, OpcodeMetadata, OpcodeType, ResourceCost};
    use crate::vm::executor::ExecutionContext;
    use std::time::Duration;

    fn create_test_context() -> DotVMContext {
        DotVMContext {
            execution_context: ExecutionContext::new(),
            dot_id: "test_dot".to_string(),
            session_id: "test_session".to_string(),
            security_level: SecurityLevel::Standard,
            caller_context: None,
            security_metadata: crate::security::types::SecurityMetadata {
                start_time: SystemTime::now(),
                permissions_checked: Vec::new(),
                capabilities_used: Vec::new(),
                resource_allocations: Vec::new(),
                audit_trail: Vec::new(),
            },
            resource_usage: Default::default(),
        }
    }

    fn create_test_opcode() -> CustomOpcode {
        CustomOpcode {
            opcode_type: OpcodeType::Standard {
                architecture: OpcodeArchitecture::Arch64,
                category: OpcodeCategory::Arithmetic,
            },
            parameters: vec![1, 2, 3],
            metadata: OpcodeMetadata {
                source_location: None,
                call_stack_depth: 1,
                execution_count: 1,
                estimated_cost: ResourceCost::default(),
            },
        }
    }

    fn create_test_opcode_result() -> OpcodeResult {
        crate::security::types::OpcodeResult {
            success: true,
            return_value: Some(vec![42]),
            resource_consumed: ResourceCost::default(),
            execution_time: Duration::from_millis(10),
            side_effects: Vec::new(),
            errors: Vec::new(),
        }
    }

    #[test]
    fn test_security_sandbox_creation() {
        let sandbox = SecuritySandbox::new();
        assert!(sandbox.config.enabled);
    }

    #[test]
    fn test_security_sandbox_disabled() {
        let mut config = SecuritySandboxConfig::default();
        config.enabled = false;

        let sandbox = SecuritySandbox::with_config(config);
        let context = create_test_context();
        let opcode = create_test_opcode();

        // All checks should pass when sandbox is disabled
        assert!(sandbox.check_capability(&context, &opcode).is_ok());
    }

    #[test]
    fn test_comprehensive_security_check() {
        let sandbox = SecuritySandbox::new();
        let context = create_test_context();
        let opcode = create_test_opcode();
        let permissions = vec![Permission::Execute { resource: "arithmetic".to_string() }];
        let resource_usage = crate::security::types::CurrentResourceUsage::default();

        let result = sandbox.comprehensive_security_check(&context, &opcode, &permissions, &resource_usage);

        // Should succeed but might have warnings due to missing setup
        assert!(result.is_ok());
    }

    #[test]
    fn test_initialize_dot_security_context() {
        let sandbox = SecuritySandbox::new();

        let result = sandbox.initialize_dot_security_context("test_dot".to_string(), SecurityLevel::Standard);

        assert!(result.is_ok());
    }

    #[test]
    fn test_audit_opcode_call() {
        let sandbox = SecuritySandbox::new();
        let context = create_test_context();
        let opcode = create_test_opcode();
        let opcode_result = create_test_opcode_result();

        // Should not panic or error
        sandbox.audit_opcode_call(&context, &opcode, &opcode_result);
    }

    #[test]
    fn test_get_security_statistics() {
        let sandbox = SecuritySandbox::new();

        let result = sandbox.get_security_statistics("test_dot");
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.dot_id, "test_dot");
    }

    #[test]
    fn test_cleanup_dot_security_context() {
        let sandbox = SecuritySandbox::new();

        let result = sandbox.cleanup_dot_security_context("test_dot");
        assert!(result.is_ok());
    }
}
