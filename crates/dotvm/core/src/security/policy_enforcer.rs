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

//! Policy Enforcer
//!
//! Implements runtime security policy enforcement to ensure compliance
//! with organizational security requirements and regulatory standards.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::security::errors::{PolicyError, PolicyResult};
use crate::security::types::{CustomOpcode, DotVMContext, OpcodeResult, SecurityLevel};

/// Policy enforcer for runtime security policies
#[derive(Debug)]
pub struct PolicyEnforcer {
    /// Active security policies
    policies: Arc<RwLock<HashMap<String, SecurityPolicy>>>,
    /// Policy evaluation engine
    evaluation_engine: Arc<RwLock<PolicyEvaluationEngine>>,
    /// Policy violation history
    violations: Arc<RwLock<Vec<PolicyViolation>>>,
    /// Policy compliance tracking
    compliance_tracker: Arc<RwLock<ComplianceTracker>>,
    /// Enforcer configuration
    config: PolicyEnforcerConfig,
}

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy identifier
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Policy version
    pub version: String,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
    /// Policy metadata
    pub metadata: HashMap<String, String>,
    /// Policy validity period
    pub valid_from: SystemTime,
    pub valid_until: Option<SystemTime>,
    /// Policy enforcement level
    pub enforcement_level: EnforcementLevel,
    /// Policy category
    pub category: PolicyCategory,
    /// Policy priority (higher = more important)
    pub priority: u32,
}

/// Individual policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule conditions
    pub conditions: Vec<PolicyCondition>,
    /// Rule actions
    pub actions: Vec<PolicyAction>,
    /// Rule enabled flag
    pub enabled: bool,
    /// Rule priority within policy
    pub priority: u32,
    /// Rule metadata
    pub metadata: HashMap<String, String>,
}

/// Policy condition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Security level requirement
    SecurityLevel { operator: ComparisonOperator, value: SecurityLevel },
    /// Opcode type restriction
    OpcodeType { operator: StringOperator, value: String },
    /// Resource usage limit
    ResourceUsage { resource_type: String, operator: ComparisonOperator, value: u64 },
    /// Time-based condition
    TimeRange { start_time: Option<SystemTime>, end_time: Option<SystemTime> },
    /// Dot-based condition
    DotIdentifier { operator: StringOperator, value: String },
    /// Execution context condition
    ExecutionContext { property: String, operator: StringOperator, value: String },
    /// Custom condition with arbitrary logic
    Custom { condition_type: String, parameters: HashMap<String, String> },
}

/// Policy action to take when rule is triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny { reason: String },
    /// Log the operation
    Log { level: LogLevel, message: String },
    /// Issue a warning
    Warn { message: String },
    /// Throttle the operation
    Throttle { delay_ms: u64 },
    /// Terminate the execution
    Terminate { reason: String },
    /// Execute custom action
    Custom { action_type: String, parameters: HashMap<String, String> },
}

/// Comparison operators for numeric conditions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

/// String operators for text conditions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringOperator {
    Equal,
    NotEqual,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Matches, // Regex pattern
    NotMatches,
}

/// Policy enforcement levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementLevel {
    /// Log violations but allow execution
    Advisory,
    /// Issue warnings but allow execution
    Warning,
    /// Enforce policies and block violations
    Enforcing,
    /// Strict enforcement with immediate termination
    Strict,
}

/// Policy categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyCategory {
    Security,
    Performance,
    Compliance,
    Audit,
    Resource,
    Access,
    Custom(String),
}

/// Log levels for policy actions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Policy evaluation engine
#[derive(Debug, Clone)]
pub struct PolicyEvaluationEngine {
    /// Evaluation cache for performance
    cache: HashMap<PolicyCacheKey, PolicyEvaluationResult>,
    /// Cache statistics
    cache_hits: u64,
    cache_misses: u64,
    /// Evaluation statistics
    evaluation_statistics: HashMap<String, PolicyEvaluationStats>,
}

/// Cache key for policy evaluations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyCacheKey {
    policy_id: String,
    context_hash: String,
    opcode_hash: String,
}

/// Policy evaluation result
#[derive(Debug, Clone)]
pub struct PolicyEvaluationResult {
    /// Whether policy allows the operation
    allowed: bool,
    /// Actions to take
    actions: Vec<PolicyAction>,
    /// Evaluation timestamp
    evaluated_at: SystemTime,
    /// Triggered rules
    triggered_rules: Vec<String>,
}

/// Policy evaluation statistics
#[derive(Debug, Clone, Default)]
pub struct PolicyEvaluationStats {
    pub total_evaluations: u64,
    pub successful_evaluations: u64,
    pub failed_evaluations: u64,
    pub average_evaluation_time_ms: f64,
    pub last_evaluation: Option<SystemTime>,
}

/// Policy violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    /// Violation identifier
    pub id: String,
    /// Violation timestamp
    pub timestamp: SystemTime,
    /// Policy that was violated
    pub policy_id: String,
    /// Rule that was violated
    pub rule_id: String,
    /// Dot that caused the violation
    pub dot_id: String,
    /// Violation details
    pub details: String,
    /// Violation severity
    pub severity: ViolationSeverity,
    /// Enforcement action taken
    pub enforcement_action: String,
    /// Resolution status
    pub resolved: bool,
    /// Resolution timestamp
    pub resolved_at: Option<SystemTime>,
}

/// Violation severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Compliance tracking information
#[derive(Debug, Clone, Default)]
pub struct ComplianceTracker {
    /// Compliance status by policy
    policy_compliance: HashMap<String, ComplianceStatus>,
    /// Overall compliance score
    overall_compliance_score: f64,
    /// Last compliance check
    last_check: Option<SystemTime>,
    /// Compliance history
    compliance_history: Vec<ComplianceSnapshot>,
}

/// Compliance status for a policy
#[derive(Debug, Clone)]
pub struct ComplianceStatus {
    pub policy_id: String,
    pub compliant: bool,
    pub compliance_score: f64,
    pub violations_count: u64,
    pub last_violation: Option<SystemTime>,
    pub check_count: u64,
}

/// Point-in-time compliance snapshot
#[derive(Debug, Clone)]
pub struct ComplianceSnapshot {
    pub timestamp: SystemTime,
    pub overall_score: f64,
    pub policy_scores: HashMap<String, f64>,
    pub total_violations: u64,
}

/// Policy enforcer configuration
#[derive(Debug, Clone)]
pub struct PolicyEnforcerConfig {
    /// Enable policy enforcement
    pub enabled: bool,
    /// Default enforcement level
    pub default_enforcement_level: EnforcementLevel,
    /// Enable policy caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum violations to track
    pub max_violations_history: usize,
    /// Enable compliance tracking
    pub enable_compliance_tracking: bool,
    /// Compliance check interval in seconds
    pub compliance_check_interval_seconds: u64,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Policy evaluation timeout in milliseconds
    pub evaluation_timeout_ms: u64,
}

impl Default for PolicyEnforcerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_enforcement_level: EnforcementLevel::Enforcing,
            enable_caching: true,
            cache_ttl_seconds: 300, // 5 minutes
            max_violations_history: 1000,
            enable_compliance_tracking: true,
            compliance_check_interval_seconds: 3600, // 1 hour
            enable_performance_monitoring: true,
            evaluation_timeout_ms: 1000, // 1 second
        }
    }
}

impl PolicyEnforcer {
    /// Create a new policy enforcer
    pub fn new() -> Self {
        Self::with_config(PolicyEnforcerConfig::default())
    }

    /// Create a new policy enforcer with custom configuration
    pub fn with_config(config: PolicyEnforcerConfig) -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            evaluation_engine: Arc::new(RwLock::new(PolicyEvaluationEngine::new())),
            violations: Arc::new(RwLock::new(Vec::new())),
            compliance_tracker: Arc::new(RwLock::new(ComplianceTracker::default())),
            config,
        }
    }

    /// Enforce policies for an opcode execution
    pub fn enforce_policies(&self, context: &DotVMContext, opcode: &CustomOpcode, opcode_result: Option<&OpcodeResult>) -> PolicyResult<PolicyEnforcementResult> {
        if !self.config.enabled {
            return Ok(PolicyEnforcementResult {
                allowed: true,
                actions_taken: Vec::new(),
                violated_policies: Vec::new(),
                enforcement_level: EnforcementLevel::Advisory,
            });
        }

        let policies = self.policies.read().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: "unknown".to_string(),
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        let mut enforcement_result = PolicyEnforcementResult {
            allowed: true,
            actions_taken: Vec::new(),
            violated_policies: Vec::new(),
            enforcement_level: self.config.default_enforcement_level.clone(),
        };

        // Sort policies by priority (higher priority first)
        let mut sorted_policies: Vec<_> = policies.values().collect();
        sorted_policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        for policy in sorted_policies {
            if !self.is_policy_active(policy) {
                continue;
            }

            let evaluation_result = self.evaluate_policy(policy, context, opcode, opcode_result)?;

            if !evaluation_result.allowed {
                enforcement_result.allowed = false;
                enforcement_result.violated_policies.push(policy.id.clone());

                // Log violation
                self.log_violation(policy, &evaluation_result.triggered_rules, context, opcode)?;

                // Apply enforcement level
                match policy.enforcement_level {
                    EnforcementLevel::Advisory => {
                        // Just log, don't block
                    }
                    EnforcementLevel::Warning => {
                        // Issue warning but allow
                        enforcement_result.actions_taken.push("warning_issued".to_string());
                    }
                    EnforcementLevel::Enforcing => {
                        // Block execution
                        return Ok(enforcement_result);
                    }
                    EnforcementLevel::Strict => {
                        // Immediate termination
                        enforcement_result.actions_taken.push("terminated".to_string());
                        return Ok(enforcement_result);
                    }
                }
            }

            // Execute policy actions
            for action in &evaluation_result.actions {
                self.execute_policy_action(action, context)?;
                enforcement_result.actions_taken.push(format!("{:?}", action));
            }
        }

        // Update compliance tracking
        if self.config.enable_compliance_tracking {
            self.update_compliance_tracking(&enforcement_result)?;
        }

        Ok(enforcement_result)
    }

    /// Add a security policy
    pub fn add_policy(&self, policy: SecurityPolicy) -> PolicyResult<()> {
        // Validate policy
        self.validate_policy(&policy)?;

        let mut policies = self.policies.write().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: policy.id.clone(),
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        policies.insert(policy.id.clone(), policy);

        // Clear evaluation cache
        if self.config.enable_caching {
            self.clear_evaluation_cache()?;
        }

        Ok(())
    }

    /// Remove a security policy
    pub fn remove_policy(&self, policy_id: &str) -> PolicyResult<()> {
        let mut policies = self.policies.write().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: policy_id.to_string(),
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        policies.remove(policy_id).ok_or_else(|| PolicyError::PolicyNotFound { policy_id: policy_id.to_string() })?;

        // Clear evaluation cache
        if self.config.enable_caching {
            self.clear_evaluation_cache()?;
        }

        Ok(())
    }

    /// Update a security policy
    pub fn update_policy(&self, policy: SecurityPolicy) -> PolicyResult<()> {
        // Validate policy
        self.validate_policy(&policy)?;

        let mut policies = self.policies.write().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: policy.id.clone(),
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        if !policies.contains_key(&policy.id) {
            return Err(PolicyError::PolicyNotFound { policy_id: policy.id });
        }

        policies.insert(policy.id.clone(), policy);

        // Clear evaluation cache
        if self.config.enable_caching {
            self.clear_evaluation_cache()?;
        }

        Ok(())
    }

    /// Get policy enforcement statistics
    pub fn get_enforcement_statistics(&self) -> PolicyResult<PolicyEnforcementStatistics> {
        let policies = self.policies.read().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: "unknown".to_string(),
            reason: format!("Failed to acquire policies lock: {}", e),
        })?;

        let violations = self.violations.read().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: "unknown".to_string(),
            reason: format!("Failed to acquire violations lock: {}", e),
        })?;

        let evaluation_engine = self.evaluation_engine.read().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: "unknown".to_string(),
            reason: format!("Failed to acquire evaluation engine lock: {}", e),
        })?;

        let compliance_tracker = self.compliance_tracker.read().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: "unknown".to_string(),
            reason: format!("Failed to acquire compliance tracker lock: {}", e),
        })?;

        Ok(PolicyEnforcementStatistics {
            total_policies: policies.len(),
            active_policies: policies.values().filter(|p| self.is_policy_active(p)).count(),
            total_violations: violations.len(),
            unresolved_violations: violations.iter().filter(|v| !v.resolved).count(),
            cache_hit_rate: if evaluation_engine.cache_hits + evaluation_engine.cache_misses > 0 {
                evaluation_engine.cache_hits as f64 / (evaluation_engine.cache_hits + evaluation_engine.cache_misses) as f64
            } else {
                0.0
            },
            overall_compliance_score: compliance_tracker.overall_compliance_score,
        })
    }

    // Private helper methods
    fn evaluate_policy(&self, policy: &SecurityPolicy, context: &DotVMContext, opcode: &CustomOpcode, opcode_result: Option<&OpcodeResult>) -> PolicyResult<PolicyEvaluationResult> {
        // Check cache first
        if self.config.enable_caching {
            if let Some(cached_result) = self.check_evaluation_cache(policy, context, opcode)? {
                return Ok(cached_result);
            }
        }

        let mut result = PolicyEvaluationResult {
            allowed: true,
            actions: Vec::new(),
            evaluated_at: SystemTime::now(),
            triggered_rules: Vec::new(),
        };

        // Sort rules by priority
        let mut sorted_rules = policy.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in &sorted_rules {
            if !rule.enabled {
                continue;
            }

            if self.evaluate_rule_conditions(rule, context, opcode, opcode_result)? {
                result.triggered_rules.push(rule.id.clone());

                for action in &rule.actions {
                    match action {
                        PolicyAction::Allow => {
                            // Explicitly allow
                        }
                        PolicyAction::Deny { .. } => {
                            result.allowed = false;
                        }
                        _ => {
                            result.actions.push(action.clone());
                        }
                    }
                }
            }
        }

        // Cache the result
        if self.config.enable_caching {
            self.cache_evaluation_result(policy, context, opcode, &result)?;
        }

        Ok(result)
    }

    fn evaluate_rule_conditions(&self, rule: &PolicyRule, context: &DotVMContext, opcode: &CustomOpcode, _opcode_result: Option<&OpcodeResult>) -> PolicyResult<bool> {
        for condition in &rule.conditions {
            if !self.evaluate_condition(condition, context, opcode)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn evaluate_condition(&self, condition: &PolicyCondition, context: &DotVMContext, opcode: &CustomOpcode) -> PolicyResult<bool> {
        match condition {
            PolicyCondition::SecurityLevel { operator, value } => Ok(self.compare_security_levels(&context.security_level, operator, value)),
            PolicyCondition::OpcodeType { operator, value } => {
                let opcode_type_str = format!("{:?}", opcode.opcode_type);
                Ok(self.compare_strings(&opcode_type_str, operator, value))
            }
            PolicyCondition::ResourceUsage { resource_type, operator, value } => {
                let current_usage = self.get_resource_usage(context, resource_type);
                Ok(self.compare_numbers(current_usage, operator, *value))
            }
            PolicyCondition::TimeRange { start_time, end_time } => {
                let now = SystemTime::now();
                let after_start = start_time.map_or(true, |start| now >= start);
                let before_end = end_time.map_or(true, |end| now <= end);
                Ok(after_start && before_end)
            }
            PolicyCondition::DotIdentifier { operator, value } => Ok(self.compare_strings(&context.dot_id, operator, value)),
            PolicyCondition::ExecutionContext { property, operator, value } => {
                let context_value = self.get_context_property(context, property);
                Ok(self.compare_strings(&context_value, operator, value))
            }
            PolicyCondition::Custom { condition_type, parameters: _ } => {
                // Custom condition evaluation
                match condition_type.as_str() {
                    "always_true" => Ok(true),
                    "always_false" => Ok(false),
                    _ => Ok(false),
                }
            }
        }
    }

    fn compare_security_levels(&self, current: &SecurityLevel, operator: &ComparisonOperator, target: &SecurityLevel) -> bool {
        let current_level = self.security_level_to_number(current);
        let target_level = self.security_level_to_number(target);

        match operator {
            ComparisonOperator::Equal => current_level == target_level,
            ComparisonOperator::NotEqual => current_level != target_level,
            ComparisonOperator::GreaterThan => current_level > target_level,
            ComparisonOperator::GreaterThanOrEqual => current_level >= target_level,
            ComparisonOperator::LessThan => current_level < target_level,
            ComparisonOperator::LessThanOrEqual => current_level <= target_level,
        }
    }

    fn compare_strings(&self, current: &str, operator: &StringOperator, target: &str) -> bool {
        match operator {
            StringOperator::Equal => current == target,
            StringOperator::NotEqual => current != target,
            StringOperator::Contains => current.contains(target),
            StringOperator::NotContains => !current.contains(target),
            StringOperator::StartsWith => current.starts_with(target),
            StringOperator::EndsWith => current.ends_with(target),
            StringOperator::Matches => {
                // Simple pattern matching - in production, use regex
                current == target || target == "*"
            }
            StringOperator::NotMatches => {
                // Simple pattern matching - in production, use regex
                current != target && target != "*"
            }
        }
    }

    fn compare_numbers(&self, current: u64, operator: &ComparisonOperator, target: u64) -> bool {
        match operator {
            ComparisonOperator::Equal => current == target,
            ComparisonOperator::NotEqual => current != target,
            ComparisonOperator::GreaterThan => current > target,
            ComparisonOperator::GreaterThanOrEqual => current >= target,
            ComparisonOperator::LessThan => current < target,
            ComparisonOperator::LessThanOrEqual => current <= target,
        }
    }

    fn security_level_to_number(&self, level: &SecurityLevel) -> u8 {
        match level {
            SecurityLevel::Development => 1,
            SecurityLevel::Standard => 2,
            SecurityLevel::High => 3,
            SecurityLevel::Maximum => 4,
            SecurityLevel::Custom { .. } => 2, // Treat as standard by default
        }
    }

    fn get_resource_usage(&self, context: &DotVMContext, resource_type: &str) -> u64 {
        match resource_type {
            "memory" => context.resource_usage.memory_bytes,
            "cpu" => context.resource_usage.cpu_time_ms,
            "instructions" => context.resource_usage.instruction_count,
            "network" => context.resource_usage.network_bytes,
            "storage" => context.resource_usage.storage_bytes,
            _ => 0,
        }
    }

    fn get_context_property(&self, context: &DotVMContext, property: &str) -> String {
        match property {
            "dot_id" => context.dot_id.clone(),
            "session_id" => context.session_id.clone(),
            "security_level" => format!("{:?}", context.security_level),
            _ => String::new(),
        }
    }

    fn execute_policy_action(&self, action: &PolicyAction, _context: &DotVMContext) -> PolicyResult<()> {
        match action {
            PolicyAction::Allow => {
                // No action needed
            }
            PolicyAction::Deny { reason: _ } => {
                // Denial is handled by the caller
            }
            PolicyAction::Log { level: _, message: _ } => {
                // Log action would be implemented here
            }
            PolicyAction::Warn { message: _ } => {
                // Warning action would be implemented here
            }
            PolicyAction::Throttle { delay_ms: _ } => {
                // Throttling would be implemented here
            }
            PolicyAction::Terminate { reason: _ } => {
                // Termination would be implemented here
            }
            PolicyAction::Custom { action_type: _, parameters: _ } => {
                // Custom actions would be implemented here
            }
        }
        Ok(())
    }

    fn log_violation(&self, policy: &SecurityPolicy, triggered_rules: &[String], context: &DotVMContext, _opcode: &CustomOpcode) -> PolicyResult<()> {
        let violation = PolicyViolation {
            id: self.generate_violation_id(),
            timestamp: SystemTime::now(),
            policy_id: policy.id.clone(),
            rule_id: triggered_rules.first().cloned().unwrap_or_default(),
            dot_id: context.dot_id.clone(),
            details: format!("Policy violation: {}", policy.name),
            severity: ViolationSeverity::Medium, // Default severity
            enforcement_action: format!("{:?}", policy.enforcement_level),
            resolved: false,
            resolved_at: None,
        };

        let mut violations = self.violations.write().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: policy.id.clone(),
            reason: format!("Failed to acquire violations lock: {}", e),
        })?;

        violations.push(violation);

        // Limit violations history
        if violations.len() > self.config.max_violations_history {
            violations.remove(0);
        }

        Ok(())
    }

    fn is_policy_active(&self, policy: &SecurityPolicy) -> bool {
        let now = SystemTime::now();
        now >= policy.valid_from && policy.valid_until.map_or(true, |until| now <= until)
    }

    fn validate_policy(&self, policy: &SecurityPolicy) -> PolicyResult<()> {
        if policy.id.is_empty() {
            return Err(PolicyError::InvalidConfiguration {
                policy_id: policy.id.clone(),
                reason: "Policy ID cannot be empty".to_string(),
            });
        }

        if policy.rules.is_empty() {
            return Err(PolicyError::InvalidConfiguration {
                policy_id: policy.id.clone(),
                reason: "Policy must have at least one rule".to_string(),
            });
        }

        // Validate rules
        for rule in &policy.rules {
            if rule.conditions.is_empty() {
                return Err(PolicyError::InvalidConfiguration {
                    policy_id: policy.id.clone(),
                    reason: format!("Rule {} must have at least one condition", rule.id),
                });
            }

            if rule.actions.is_empty() {
                return Err(PolicyError::InvalidConfiguration {
                    policy_id: policy.id.clone(),
                    reason: format!("Rule {} must have at least one action", rule.id),
                });
            }
        }

        Ok(())
    }

    fn check_evaluation_cache(&self, _policy: &SecurityPolicy, _context: &DotVMContext, _opcode: &CustomOpcode) -> PolicyResult<Option<PolicyEvaluationResult>> {
        // Cache checking would be implemented here
        Ok(None)
    }

    fn cache_evaluation_result(&self, _policy: &SecurityPolicy, _context: &DotVMContext, _opcode: &CustomOpcode, _result: &PolicyEvaluationResult) -> PolicyResult<()> {
        // Result caching would be implemented here
        Ok(())
    }

    fn clear_evaluation_cache(&self) -> PolicyResult<()> {
        let mut engine = self.evaluation_engine.write().map_err(|e| PolicyError::EnforcementFailed {
            policy_id: "unknown".to_string(),
            reason: format!("Failed to acquire evaluation engine lock: {}", e),
        })?;

        engine.cache.clear();
        Ok(())
    }

    fn update_compliance_tracking(&self, _result: &PolicyEnforcementResult) -> PolicyResult<()> {
        // Compliance tracking would be implemented here
        Ok(())
    }

    fn generate_violation_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        format!("violation_{:x}", now.as_nanos())
    }
}

impl Default for PolicyEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEvaluationEngine {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            evaluation_statistics: HashMap::new(),
        }
    }
}

/// Policy enforcement result
#[derive(Debug, Clone)]
pub struct PolicyEnforcementResult {
    /// Whether the operation is allowed
    pub allowed: bool,
    /// Actions that were taken
    pub actions_taken: Vec<String>,
    /// Policies that were violated
    pub violated_policies: Vec<String>,
    /// Effective enforcement level
    pub enforcement_level: EnforcementLevel,
}

/// Policy enforcement statistics
#[derive(Debug, Clone)]
pub struct PolicyEnforcementStatistics {
    pub total_policies: usize,
    pub active_policies: usize,
    pub total_violations: usize,
    pub unresolved_violations: usize,
    pub cache_hit_rate: f64,
    pub overall_compliance_score: f64,
}

/// Helper functions for creating common security policies
pub fn create_default_security_policies() -> Vec<SecurityPolicy> {
    vec![
        SecurityPolicy {
            id: "resource_limits".to_string(),
            name: "Resource Limits Policy".to_string(),
            description: "Enforce resource usage limits".to_string(),
            version: "1.0".to_string(),
            rules: vec![PolicyRule {
                id: "memory_limit".to_string(),
                name: "Memory Usage Limit".to_string(),
                conditions: vec![PolicyCondition::ResourceUsage {
                    resource_type: "memory".to_string(),
                    operator: ComparisonOperator::GreaterThan,
                    value: 100 * 1024 * 1024, // 100MB
                }],
                actions: vec![PolicyAction::Deny {
                    reason: "Memory usage exceeds limit".to_string(),
                }],
                enabled: true,
                priority: 100,
                metadata: HashMap::new(),
            }],
            metadata: HashMap::new(),
            valid_from: SystemTime::now(),
            valid_until: None,
            enforcement_level: EnforcementLevel::Enforcing,
            category: PolicyCategory::Resource,
            priority: 100,
        },
        SecurityPolicy {
            id: "security_level_enforcement".to_string(),
            name: "Security Level Enforcement".to_string(),
            description: "Enforce minimum security levels for sensitive operations".to_string(),
            version: "1.0".to_string(),
            rules: vec![PolicyRule {
                id: "high_security_required".to_string(),
                name: "High Security Required for System Operations".to_string(),
                conditions: vec![
                    PolicyCondition::OpcodeType {
                        operator: StringOperator::Contains,
                        value: "System".to_string(),
                    },
                    PolicyCondition::SecurityLevel {
                        operator: ComparisonOperator::LessThan,
                        value: SecurityLevel::High,
                    },
                ],
                actions: vec![PolicyAction::Deny {
                    reason: "High security level required for system operations".to_string(),
                }],
                enabled: true,
                priority: 200,
                metadata: HashMap::new(),
            }],
            metadata: HashMap::new(),
            valid_from: SystemTime::now(),
            valid_until: None,
            enforcement_level: EnforcementLevel::Strict,
            category: PolicyCategory::Security,
            priority: 200,
        },
    ]
}
