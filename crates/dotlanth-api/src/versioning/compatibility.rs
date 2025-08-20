// Dotlanth
// Copyright (C) 2025 Synerthink

use crate::versioning::{ApiVersion, ProtocolType, ServiceType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Compatibility check errors
#[derive(Error, Debug)]
pub enum CompatibilityError {
    #[error("Breaking change detected: {0}")]
    BreakingChange(String),
    #[error("Incompatible versions: {client} and {server}")]
    IncompatibleVersions { client: String, server: String },
    #[error("Feature not available in version {version}: {feature}")]
    FeatureNotAvailable { version: String, feature: String },
    #[error("Deprecated feature used: {feature} (deprecated in {version})")]
    DeprecatedFeature { feature: String, version: String },
}

/// Change type for API modifications
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Addition of new features (backwards compatible)
    Addition,
    /// Modification of existing features (potentially breaking)
    Modification,
    /// Removal of features (breaking change)
    Removal,
    /// Deprecation of features (backwards compatible but warns)
    Deprecation,
}

/// API change description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChange {
    pub change_type: ChangeType,
    pub component: String,
    pub description: String,
    pub introduced_in: ApiVersion,
    pub removed_in: Option<ApiVersion>,
    pub migration_guide: Option<String>,
}

/// Compatibility rule for validating API changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityRule {
    pub name: String,
    pub description: String,
    pub applies_to: Vec<ProtocolType>,
    pub check_function: String, // Function name for dynamic checking
    pub severity: CompatibilitySeverity,
}

/// Severity level for compatibility issues
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilitySeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Compatibility check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResult {
    pub is_compatible: bool,
    pub issues: Vec<CompatibilityIssue>,
    pub warnings: Vec<String>,
    pub required_migrations: Vec<MigrationInfo>,
}

/// Individual compatibility issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityIssue {
    pub severity: CompatibilitySeverity,
    pub message: String,
    pub component: String,
    pub suggested_action: Option<String>,
}

/// Migration information for version transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    pub from_version: ApiVersion,
    pub to_version: ApiVersion,
    pub description: String,
    pub steps: Vec<String>,
    pub breaking_changes: Vec<String>,
}

/// Backwards compatibility checker
#[derive(Debug, Clone)]
pub struct CompatibilityChecker {
    /// Known API changes by version
    changes: HashMap<(ProtocolType, ServiceType, ApiVersion), Vec<ApiChange>>,
    /// Compatibility rules
    rules: Vec<CompatibilityRule>,
    /// Migration guides
    migrations: HashMap<(ApiVersion, ApiVersion), MigrationInfo>,
}

impl Default for CompatibilityChecker {
    fn default() -> Self {
        let mut checker = Self {
            changes: HashMap::new(),
            rules: Vec::new(),
            migrations: HashMap::new(),
        };

        checker.initialize_default_rules();
        checker.initialize_default_changes();

        checker
    }
}

impl CompatibilityChecker {
    /// Create a new compatibility checker
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an API change
    pub fn register_change(&mut self, protocol: ProtocolType, service: ServiceType, change: ApiChange) {
        let key = (protocol, service, change.introduced_in.clone());
        self.changes.entry(key).or_insert_with(Vec::new).push(change);
    }

    /// Register a migration guide
    pub fn register_migration(&mut self, migration: MigrationInfo) {
        let key = (migration.from_version.clone(), migration.to_version.clone());
        self.migrations.insert(key, migration);
    }

    /// Check compatibility between two versions
    pub fn check_compatibility(&self, protocol: &ProtocolType, service: &ServiceType, from_version: &ApiVersion, to_version: &ApiVersion) -> CompatibilityResult {
        let mut result = CompatibilityResult {
            is_compatible: true,
            issues: Vec::new(),
            warnings: Vec::new(),
            required_migrations: Vec::new(),
        };

        // Check if it's a breaking change (major version bump)
        if to_version.is_breaking_change_from(from_version) {
            result.is_compatible = false;
            result.issues.push(CompatibilityIssue {
                severity: CompatibilitySeverity::Critical,
                message: format!("Major version change from {} to {} introduces breaking changes", from_version, to_version),
                component: "version".to_string(),
                suggested_action: Some("Review migration guide and update client code".to_string()),
            });
        }

        // Check all changes between versions
        let changes = self.get_changes_between_versions(protocol, service, from_version, to_version);
        for change in changes {
            match change.change_type {
                ChangeType::Removal => {
                    result.is_compatible = false;
                    result.issues.push(CompatibilityIssue {
                        severity: CompatibilitySeverity::Error,
                        message: format!("Component removed: {}", change.description),
                        component: change.component.clone(),
                        suggested_action: change.migration_guide.clone(),
                    });
                }
                ChangeType::Modification => {
                    result.warnings.push(format!("Component modified: {}", change.description));
                    result.issues.push(CompatibilityIssue {
                        severity: CompatibilitySeverity::Warning,
                        message: format!("Component modified: {}", change.description),
                        component: change.component.clone(),
                        suggested_action: change.migration_guide.clone(),
                    });
                }
                ChangeType::Deprecation => {
                    result.warnings.push(format!("Component deprecated: {}", change.description));
                    result.issues.push(CompatibilityIssue {
                        severity: CompatibilitySeverity::Warning,
                        message: format!("Component deprecated: {}", change.description),
                        component: change.component.clone(),
                        suggested_action: Some("Plan migration to replacement".to_string()),
                    });
                }
                ChangeType::Addition => {
                    // Additions are always compatible
                }
            }
        }

        // Add migration information if available
        if let Some(migration) = self.migrations.get(&(from_version.clone(), to_version.clone())) {
            result.required_migrations.push(migration.clone());
        }

        result
    }

    /// Validate that a request is compatible with the negotiated version
    pub fn validate_request_compatibility(&self, protocol: &ProtocolType, service: &ServiceType, version: &ApiVersion, request_features: &[String]) -> Result<(), CompatibilityError> {
        for feature in request_features {
            if !self.is_feature_available(protocol, service, version, feature) {
                return Err(CompatibilityError::FeatureNotAvailable {
                    version: version.to_string(),
                    feature: feature.clone(),
                });
            }

            if self.is_feature_deprecated(protocol, service, version, feature) {
                // Log warning but don't fail
                tracing::warn!("Using deprecated feature: {} in version {}", feature, version);
            }
        }

        Ok(())
    }

    /// Check if a feature is available in a specific version
    pub fn is_feature_available(&self, protocol: &ProtocolType, service: &ServiceType, version: &ApiVersion, feature: &str) -> bool {
        // Check if feature was introduced before or in this version
        for ((p, s, v), changes) in &self.changes {
            if p == protocol && s == service && v <= version {
                for change in changes {
                    if change.component == feature {
                        match change.change_type {
                            ChangeType::Addition => return true,
                            ChangeType::Removal => {
                                if let Some(removed_in) = &change.removed_in {
                                    return version < removed_in;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Default features are always available unless explicitly removed
        true
    }

    /// Check if a feature is deprecated in a specific version
    pub fn is_feature_deprecated(&self, protocol: &ProtocolType, service: &ServiceType, version: &ApiVersion, feature: &str) -> bool {
        for ((p, s, v), changes) in &self.changes {
            if p == protocol && s == service && v <= version {
                for change in changes {
                    if change.component == feature && change.change_type == ChangeType::Deprecation {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get all changes between two versions
    fn get_changes_between_versions(&self, protocol: &ProtocolType, service: &ServiceType, from_version: &ApiVersion, to_version: &ApiVersion) -> Vec<&ApiChange> {
        let mut changes = Vec::new();

        for ((p, s, v), change_list) in &self.changes {
            if p == protocol && s == service && v > from_version && v <= to_version {
                changes.extend(change_list.iter());
            }
        }

        changes
    }

    /// Initialize default compatibility rules
    fn initialize_default_rules(&mut self) {
        self.rules = vec![
            CompatibilityRule {
                name: "no_breaking_changes_in_minor".to_string(),
                description: "Minor version updates should not introduce breaking changes".to_string(),
                applies_to: vec![ProtocolType::Rest, ProtocolType::GraphQL, ProtocolType::Grpc, ProtocolType::WebSocket],
                check_function: "check_no_breaking_changes_in_minor".to_string(),
                severity: CompatibilitySeverity::Error,
            },
            CompatibilityRule {
                name: "deprecation_before_removal".to_string(),
                description: "Features should be deprecated before removal".to_string(),
                applies_to: vec![ProtocolType::Rest, ProtocolType::GraphQL, ProtocolType::Grpc, ProtocolType::WebSocket],
                check_function: "check_deprecation_before_removal".to_string(),
                severity: CompatibilitySeverity::Warning,
            },
        ];
    }

    /// Initialize default changes for current API versions
    fn initialize_default_changes(&mut self) {
        // Example changes - in a real implementation, these would be loaded from configuration
        let v1_0_0 = ApiVersion::new(1, 0, 0);

        // VM service initial features
        let vm_changes = vec![
            ApiChange {
                change_type: ChangeType::Addition,
                component: "execute_dot".to_string(),
                description: "Basic dot execution functionality".to_string(),
                introduced_in: v1_0_0.clone(),
                removed_in: None,
                migration_guide: None,
            },
            ApiChange {
                change_type: ChangeType::Addition,
                component: "deploy_dot".to_string(),
                description: "Dot deployment functionality".to_string(),
                introduced_in: v1_0_0.clone(),
                removed_in: None,
                migration_guide: None,
            },
        ];

        for change in vm_changes {
            self.register_change(ProtocolType::Rest, ServiceType::Vm, change.clone());
            self.register_change(ProtocolType::Grpc, ServiceType::Vm, change.clone());
            self.register_change(ProtocolType::GraphQL, ServiceType::Vm, change);
        }

        // Database service initial features
        let db_changes = vec![
            ApiChange {
                change_type: ChangeType::Addition,
                component: "get".to_string(),
                description: "Basic get operation".to_string(),
                introduced_in: v1_0_0.clone(),
                removed_in: None,
                migration_guide: None,
            },
            ApiChange {
                change_type: ChangeType::Addition,
                component: "put".to_string(),
                description: "Basic put operation".to_string(),
                introduced_in: v1_0_0.clone(),
                removed_in: None,
                migration_guide: None,
            },
        ];

        for change in db_changes {
            self.register_change(ProtocolType::Rest, ServiceType::Database, change.clone());
            self.register_change(ProtocolType::Grpc, ServiceType::Database, change);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compatibility_check() {
        let checker = CompatibilityChecker::new();
        let v1_0_0 = ApiVersion::new(1, 0, 0);
        let v1_1_0 = ApiVersion::new(1, 1, 0);

        let result = checker.check_compatibility(&ProtocolType::Rest, &ServiceType::Vm, &v1_0_0, &v1_1_0);

        assert!(result.is_compatible);
    }

    #[test]
    fn test_breaking_change_detection() {
        let checker = CompatibilityChecker::new();
        let v1_0_0 = ApiVersion::new(1, 0, 0);
        let v2_0_0 = ApiVersion::new(2, 0, 0);

        let result = checker.check_compatibility(&ProtocolType::Rest, &ServiceType::Vm, &v1_0_0, &v2_0_0);

        assert!(!result.is_compatible);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_feature_availability() {
        let checker = CompatibilityChecker::new();
        let v1_0_0 = ApiVersion::new(1, 0, 0);

        assert!(checker.is_feature_available(&ProtocolType::Rest, &ServiceType::Vm, &v1_0_0, "execute_dot"));
    }
}
