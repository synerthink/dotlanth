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

//! # State Transition Validation
//!
//! This module implements rules and checks to validate state changes proposed
//! by dot execution. It ensures that state transitions are valid,
//! atomic, and adhere to dot-defined invariants.
//!
//! ## Key Features
//!
//! - Type checking for state values
//! - Permission checking based on caller identity
//! - Invariant validation
//! - Atomic transaction support
//! - Custom validation rules

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use dotdb_core::state::{DotAddress, StorageValue, StorageVariableType};
use serde::{Deserialize, Serialize};

/// Result type for state transition validation
pub type StateTransitionResult<T> = Result<T, StateTransitionError>;

/// Validation result with detailed information
pub type ValidationResult = StateTransitionResult<ValidationSummary>;

/// State transition that needs to be validated
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    /// Dot address where the transition occurs
    pub dot_address: DotAddress,
    /// Storage key being modified
    pub storage_key: Vec<u8>,
    /// Previous value (None if key didn't exist)
    pub old_value: Option<StorageValue>,
    /// New value (None if key is being deleted)
    pub new_value: Option<StorageValue>,
    /// Variable type information
    pub variable_type: StorageVariableType,
    /// Variable name (if known)
    pub variable_name: Option<String>,
}

/// Context for validation operations
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Address of the dot calling the operation
    pub caller: DotAddress,
    /// Address of the dot being executed
    pub dot: DotAddress,
    /// Whether this is a static call (read-only)
    pub is_static_call: bool,
    /// Current block number
    pub block_number: u64,
    /// Current timestamp
    pub timestamp: u64,
    /// Transaction value (if any)
    pub value: u64,
    /// Custom context data
    pub custom_data: HashMap<String, Vec<u8>>,
}

/// Summary of validation results
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationSummary {
    /// Whether all validations passed
    pub is_valid: bool,
    /// List of applied rules
    pub applied_rules: Vec<String>,
    /// List of violations (if any)
    pub violations: Vec<ValidationViolation>,
    /// Warnings (non-blocking issues)
    pub warnings: Vec<String>,
}

/// Validation violation details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationViolation {
    /// Rule that was violated
    pub rule_name: String,
    /// Severity of the violation
    pub severity: ViolationSeverity,
    /// Description of the violation
    pub description: String,
    /// Storage key where violation occurred
    pub storage_key: Option<Vec<u8>>,
}

/// Severity levels for validation violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Informational - does not block execution
    Info,
    /// Warning - should be reviewed but doesn't block execution
    Warning,
    /// Error - blocks execution
    Error,
    /// Critical - serious security or correctness issue
    Critical,
}

/// A validation rule that can be applied to state transitions
pub trait TransitionRule: Send + Sync {
    /// Name of the rule for identification
    fn name(&self) -> &str;

    /// Validate a state transition
    fn validate(&self, transition: &StateTransition, context: &ValidationContext) -> StateTransitionResult<RuleResult>;

    /// Whether this rule is critical (must pass for execution to continue)
    fn is_critical(&self) -> bool {
        true
    }
}

/// Result of applying a single validation rule
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleResult {
    /// Whether the rule passed
    pub passed: bool,
    /// Optional violation details
    pub violation: Option<ValidationViolation>,
    /// Optional warning message
    pub warning: Option<String>,
}

/// State transition validator that orchestrates multiple rules
pub struct StateTransitionValidator {
    /// List of validation rules
    rules: Vec<Arc<dyn TransitionRule>>,
    /// Configuration options
    config: ValidatorConfig,
}

impl std::fmt::Debug for StateTransitionValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateTransitionValidator")
            .field("rules_count", &self.rules.len())
            .field("config", &self.config)
            .finish()
    }
}

/// Configuration for the state transition validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Whether to stop on first critical error
    pub fail_fast: bool,
    /// Whether to collect warnings
    pub collect_warnings: bool,
    /// Whether to validate type consistency
    pub validate_types: bool,
    /// Whether to validate permissions
    pub validate_permissions: bool,
    /// Whether to validate invariants
    pub validate_invariants: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            fail_fast: true,
            collect_warnings: true,
            validate_types: true,
            validate_permissions: true,
            validate_invariants: true,
        }
    }
}

/// Errors that can occur during state transition validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionError {
    /// Validation rule failed
    ValidationFailed(String),
    /// Invalid transition format
    InvalidTransition(String),
    /// Invalid validation context
    InvalidContext(String),
    /// Type mismatch error
    TypeMismatch { expected: String, actual: String },
    /// Permission denied
    PermissionDenied(String),
    /// Invariant violation
    InvariantViolation(String),
    /// Rule execution error
    RuleError { rule: String, error: String },
    /// Configuration error
    ConfigError(String),
}

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateTransitionError::ValidationFailed(msg) => write!(f, "Validation failed: {msg}"),
            StateTransitionError::InvalidTransition(msg) => write!(f, "Invalid transition: {msg}"),
            StateTransitionError::InvalidContext(msg) => write!(f, "Invalid context: {msg}"),
            StateTransitionError::TypeMismatch { expected, actual } => {
                write!(f, "Type mismatch: expected {expected}, got {actual}")
            }
            StateTransitionError::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
            StateTransitionError::InvariantViolation(msg) => write!(f, "Invariant violation: {msg}"),
            StateTransitionError::RuleError { rule, error } => {
                write!(f, "Rule '{rule}' error: {error}")
            }
            StateTransitionError::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
        }
    }
}

impl std::error::Error for StateTransitionError {}

impl StateTransitionValidator {
    /// Create a new validator with default configuration
    pub fn new() -> Self {
        Self::with_config(ValidatorConfig::default())
    }

    /// Create a new validator with custom configuration
    pub fn with_config(config: ValidatorConfig) -> Self {
        let mut validator = Self { rules: Vec::new(), config };

        // Add built-in rules based on configuration
        if validator.config.validate_types {
            validator.add_rule(Arc::new(TypeValidationRule::new()));
        }

        if validator.config.validate_permissions {
            validator.add_rule(Arc::new(PermissionValidationRule::new()));
        }

        if validator.config.validate_invariants {
            validator.add_rule(Arc::new(InvariantValidationRule::new()));
        }

        validator
    }

    /// Add a validation rule
    pub fn add_rule(&mut self, rule: Arc<dyn TransitionRule>) {
        self.rules.push(rule);
    }

    /// Remove a validation rule by name
    pub fn remove_rule(&mut self, name: &str) {
        self.rules.retain(|rule| rule.name() != name);
    }

    /// Validate a single state transition
    pub fn validate_transition(&self, transition: &StateTransition, context: &ValidationContext) -> ValidationResult {
        let mut summary = ValidationSummary {
            is_valid: true,
            applied_rules: Vec::new(),
            violations: Vec::new(),
            warnings: Vec::new(),
        };

        // Apply each validation rule
        for rule in &self.rules {
            // Apply the rule
            match rule.validate(transition, context) {
                Ok(result) => {
                    summary.applied_rules.push(rule.name().to_string());

                    if !result.passed {
                        summary.is_valid = false;

                        if let Some(violation) = result.violation {
                            summary.violations.push(violation.clone());

                            // Stop on first critical error if configured
                            if self.config.fail_fast && rule.is_critical() {
                                break;
                            }
                        }
                    }

                    if let Some(warning) = result.warning
                        && self.config.collect_warnings
                    {
                        summary.warnings.push(warning);
                    }
                }
                Err(e) => {
                    return Err(StateTransitionError::RuleError {
                        rule: rule.name().to_string(),
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(summary)
    }

    /// Validate multiple state transitions atomically
    pub fn validate_batch(&self, transitions: &[StateTransition], context: &ValidationContext) -> ValidationResult {
        let mut combined_summary = ValidationSummary {
            is_valid: true,
            applied_rules: Vec::new(),
            violations: Vec::new(),
            warnings: Vec::new(),
        };

        for (i, transition) in transitions.iter().enumerate() {
            match self.validate_transition(transition, context) {
                Ok(summary) => {
                    combined_summary.applied_rules.extend(summary.applied_rules);
                    combined_summary.violations.extend(summary.violations);
                    combined_summary.warnings.extend(summary.warnings);

                    if !summary.is_valid {
                        combined_summary.is_valid = false;

                        if self.config.fail_fast {
                            break;
                        }
                    }
                }
                Err(e) => {
                    return Err(StateTransitionError::ValidationFailed(format!("Transition {i} failed: {e}")));
                }
            }
        }

        Ok(combined_summary)
    }

    /// Get the list of registered rule names
    pub fn get_rule_names(&self) -> Vec<String> {
        self.rules.iter().map(|rule| rule.name().to_string()).collect()
    }
}

/// Built-in rule for type validation
struct TypeValidationRule;

impl TypeValidationRule {
    fn new() -> Self {
        Self
    }
}

impl TransitionRule for TypeValidationRule {
    fn name(&self) -> &str {
        "type_validation"
    }

    fn validate(&self, transition: &StateTransition, _context: &ValidationContext) -> StateTransitionResult<RuleResult> {
        // Validate that the new value matches the expected type
        if let Some(new_value) = &transition.new_value {
            let type_valid = match &transition.variable_type {
                StorageVariableType::Simple => true, // Any value is valid for simple types
                StorageVariableType::DynamicArray => {
                    matches!(new_value, StorageValue::Array(_) | StorageValue::Bytes(_))
                }
                StorageVariableType::Mapping { .. } => {
                    matches!(new_value, StorageValue::Mapping(_))
                }
                StorageVariableType::Struct { .. } => {
                    // For structs, we'd need more sophisticated validation
                    true
                }
            };

            if !type_valid {
                return Ok(RuleResult {
                    passed: false,
                    violation: Some(ValidationViolation {
                        rule_name: self.name().to_string(),
                        severity: ViolationSeverity::Error,
                        description: format!("Type mismatch: variable type {:?} does not match value type", transition.variable_type),
                        storage_key: Some(transition.storage_key.clone()),
                    }),
                    warning: None,
                });
            }
        }

        Ok(RuleResult {
            passed: true,
            violation: None,
            warning: None,
        })
    }
}

/// Built-in rule for permission validation
struct PermissionValidationRule;

impl PermissionValidationRule {
    fn new() -> Self {
        Self
    }
}

impl TransitionRule for PermissionValidationRule {
    fn name(&self) -> &str {
        "permission_validation"
    }

    fn validate(&self, transition: &StateTransition, context: &ValidationContext) -> StateTransitionResult<RuleResult> {
        // Check if the caller has permission to modify this storage

        // Basic rule: only the dot itself can modify its storage
        if context.caller != transition.dot_address && context.dot == transition.dot_address {
            // External call trying to modify storage - this might be allowed in some cases
            // For now, we'll issue a warning
            return Ok(RuleResult {
                passed: true,
                violation: None,
                warning: Some(format!("External caller {:?} modifying dot {:?} storage", context.caller, transition.dot_address)),
            });
        }

        // Static calls should not modify state
        if context.is_static_call && transition.new_value.is_some() {
            return Ok(RuleResult {
                passed: false,
                violation: Some(ValidationViolation {
                    rule_name: self.name().to_string(),
                    severity: ViolationSeverity::Error,
                    description: "State modification not allowed in static call".to_string(),
                    storage_key: Some(transition.storage_key.clone()),
                }),
                warning: None,
            });
        }

        Ok(RuleResult {
            passed: true,
            violation: None,
            warning: None,
        })
    }
}

/// Built-in rule for invariant validation
struct InvariantValidationRule;

impl InvariantValidationRule {
    fn new() -> Self {
        Self
    }
}

impl TransitionRule for InvariantValidationRule {
    fn name(&self) -> &str {
        "invariant_validation"
    }

    fn validate(&self, _transition: &StateTransition, _context: &ValidationContext) -> StateTransitionResult<RuleResult> {
        // This is a placeholder for dot-specific invariant validation
        // In practice, this would check dot-defined invariants

        // For now, always pass
        Ok(RuleResult {
            passed: true,
            violation: None,
            warning: None,
        })
    }

    fn is_critical(&self) -> bool {
        true // Invariant violations are always critical
    }
}

impl Default for StateTransitionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleResult {
    /// Create a successful rule result
    pub fn success() -> Self {
        Self {
            passed: true,
            violation: None,
            warning: None,
        }
    }

    /// Create a successful rule result with warning
    pub fn success_with_warning(warning: String) -> Self {
        Self {
            passed: true,
            violation: None,
            warning: Some(warning),
        }
    }

    /// Create a failed rule result
    pub fn failure(violation: ValidationViolation) -> Self {
        Self {
            passed: false,
            violation: Some(violation),
            warning: None,
        }
    }
}

impl ValidationViolation {
    /// Create a new validation violation
    pub fn new(rule_name: String, severity: ViolationSeverity, description: String) -> Self {
        Self {
            rule_name,
            severity,
            description,
            storage_key: None,
        }
    }

    /// Create a violation with storage key context
    pub fn with_key(rule_name: String, severity: ViolationSeverity, description: String, storage_key: Vec<u8>) -> Self {
        Self {
            rule_name,
            severity,
            description,
            storage_key: Some(storage_key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotdb_core::state::StorageValue;

    fn create_test_transition() -> StateTransition {
        StateTransition {
            dot_address: [1u8; 20],
            storage_key: vec![0u8; 32],
            old_value: None,
            new_value: Some(StorageValue::U256([42u8; 32])),
            variable_type: StorageVariableType::Simple,
            variable_name: Some("test_var".to_string()),
        }
    }

    fn create_test_context() -> ValidationContext {
        ValidationContext {
            caller: [1u8; 20],
            dot: [1u8; 20],
            is_static_call: false,
            block_number: 100,
            timestamp: 1640995200,
            value: 0,
            custom_data: HashMap::new(),
        }
    }

    #[test]
    fn test_validator_creation() {
        let validator = StateTransitionValidator::new();
        let rule_names = validator.get_rule_names();

        assert!(rule_names.contains(&"type_validation".to_string()));
        assert!(rule_names.contains(&"permission_validation".to_string()));
        assert!(rule_names.contains(&"invariant_validation".to_string()));
    }

    #[test]
    fn test_validator_with_custom_config() {
        let config = ValidatorConfig {
            validate_types: true,
            validate_permissions: false,
            validate_invariants: false,
            ..Default::default()
        };

        let validator = StateTransitionValidator::with_config(config);
        let rule_names = validator.get_rule_names();

        assert_eq!(rule_names.len(), 1);
        assert!(rule_names.contains(&"type_validation".to_string()));
    }

    #[test]
    fn test_single_transition_validation() {
        let validator = StateTransitionValidator::new();
        let transition = create_test_transition();
        let context = create_test_context();

        let result = validator.validate_transition(&transition, &context);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(summary.is_valid);
        assert!(!summary.applied_rules.is_empty());
    }

    #[test]
    fn test_static_call_violation() {
        let validator = StateTransitionValidator::new();
        let transition = create_test_transition();
        let mut context = create_test_context();
        context.is_static_call = true;

        let result = validator.validate_transition(&transition, &context).unwrap();
        assert!(!result.is_valid);
        assert!(!result.violations.is_empty());

        let violation = &result.violations[0];
        assert_eq!(violation.rule_name, "permission_validation");
        assert_eq!(violation.severity, ViolationSeverity::Error);
    }

    #[test]
    fn test_type_validation() {
        let validator = StateTransitionValidator::new();
        let mut transition = create_test_transition();
        transition.variable_type = StorageVariableType::DynamicArray;
        transition.new_value = Some(StorageValue::U256([0u8; 32])); // Wrong type

        let context = create_test_context();

        let result = validator.validate_transition(&transition, &context).unwrap();
        assert!(!result.is_valid);

        // Should have a type validation violation
        let has_type_violation = result.violations.iter().any(|v| v.rule_name == "type_validation");
        assert!(has_type_violation);
    }

    #[test]
    fn test_batch_validation() {
        let validator = StateTransitionValidator::new();
        let transitions = vec![create_test_transition(), create_test_transition()];
        let context = create_test_context();

        let result = validator.validate_batch(&transitions, &context);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(summary.is_valid);
    }

    #[test]
    #[test]
    fn test_rule_management() {
        let mut validator = StateTransitionValidator::new();
        let initial_count = validator.rules.len();

        // Remove a rule
        validator.remove_rule("type_validation");
        assert_eq!(validator.rules.len(), initial_count - 1);

        // Add a custom rule
        struct CustomRule;
        impl TransitionRule for CustomRule {
            fn name(&self) -> &str {
                "custom_rule"
            }
            fn validate(&self, _: &StateTransition, _: &ValidationContext) -> StateTransitionResult<RuleResult> {
                Ok(RuleResult::success())
            }
        }

        validator.add_rule(Arc::new(CustomRule));
        assert_eq!(validator.rules.len(), initial_count);

        let rule_names = validator.get_rule_names();
        assert!(rule_names.contains(&"custom_rule".to_string()));
        assert!(!rule_names.contains(&"type_validation".to_string()));
    }

    #[test]
    fn test_validation_violation_creation() {
        let violation = ValidationViolation::new("test_rule".to_string(), ViolationSeverity::Warning, "Test violation".to_string());

        assert_eq!(violation.rule_name, "test_rule");
        assert_eq!(violation.severity, ViolationSeverity::Warning);
        assert_eq!(violation.description, "Test violation");
        assert!(violation.storage_key.is_none());

        let violation_with_key = ValidationViolation::with_key("test_rule".to_string(), ViolationSeverity::Error, "Test violation with key".to_string(), vec![1, 2, 3]);

        assert!(violation_with_key.storage_key.is_some());
        assert_eq!(violation_with_key.storage_key.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_rule_result_creation() {
        let success = RuleResult::success();
        assert!(success.passed);
        assert!(success.violation.is_none());
        assert!(success.warning.is_none());

        let warning = RuleResult::success_with_warning("Test warning".to_string());
        assert!(warning.passed);
        assert!(warning.warning.is_some());

        let failure = RuleResult::failure(ValidationViolation::new("test".to_string(), ViolationSeverity::Error, "Test error".to_string()));
        assert!(!failure.passed);
        assert!(failure.violation.is_some());
    }

    #[test]
    fn test_violation_severity_ordering() {
        assert!(ViolationSeverity::Info < ViolationSeverity::Warning);
        assert!(ViolationSeverity::Warning < ViolationSeverity::Error);
        assert!(ViolationSeverity::Error < ViolationSeverity::Critical);
    }

    #[test]
    fn test_config_defaults() {
        let config = ValidatorConfig::default();
        assert!(config.fail_fast);
        assert!(config.collect_warnings);
        assert!(config.validate_types);
        assert!(config.validate_permissions);
        assert!(config.validate_invariants);
    }

    #[test]
    fn test_error_display() {
        let error = StateTransitionError::TypeMismatch {
            expected: "uint256".to_string(),
            actual: "string".to_string(),
        };
        assert!(error.to_string().contains("Type mismatch"));
    }
}
