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

use crate::finalizer::lib::{StateTransition, ValidationResult};

/// Finality Validation Processor responsible for validating state transitions
/// before they are finalized
pub struct FinalityValidator {
    // Simple configuration parameters
    min_timestamp_delta: u64,
    strict_version_increment: bool,
    authorized_initiators: Vec<String>,
}

impl Default for FinalityValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl FinalityValidator {
    /// Create a new FinalityValidator with default settings
    pub fn new() -> Self {
        Self {
            min_timestamp_delta: 500,       // Minimum 500ms between state changes
            strict_version_increment: true, // Version must increment by exactly 1
            authorized_initiators: vec!["system".to_string(), "admin".to_string(), "test_user".to_string()],
        }
    }

    /// Create a new FinalityValidator with custom settings
    pub fn with_config(min_timestamp_delta: u64, strict_version_increment: bool, authorized_initiators: Vec<String>) -> Self {
        Self {
            min_timestamp_delta,
            strict_version_increment,
            authorized_initiators,
        }
    }

    /// Validate a state transition against all defined rules
    pub fn validate_transition(&self, transition: &StateTransition) -> ValidationResult {
        // Run all validation checks sequentially
        let checks = [
            self.validate_state_consistency(transition),
            self.validate_version_increment(transition),
            self.validate_metadata(transition),
            self.validate_timestamp(transition),
            self.validate_initiator(transition),
        ];

        // If any check fails, return that failure result
        for check in checks.iter() {
            if !check.is_valid {
                return check.clone();
            }
        }

        // All checks passed
        ValidationResult::success()
    }

    /// Validate that the state transition is internally consistent
    fn validate_state_consistency(&self, transition: &StateTransition) -> ValidationResult {
        // Check if state before and after are properly defined
        if transition.state_before.data.is_empty() || transition.state_after.data.is_empty() {
            return ValidationResult::failure("State data cannot be empty");
        }

        ValidationResult::success()
    }

    /// Validate that the version is properly incremented
    fn validate_version_increment(&self, transition: &StateTransition) -> ValidationResult {
        if self.strict_version_increment {
            // Check if version is incremented by exactly 1
            if transition.state_after.version != transition.state_before.version + 1 {
                return ValidationResult::failure("State version must be incremented by exactly 1");
            }
        } else {
            // Check if version is at least incremented
            if transition.state_after.version <= transition.state_before.version {
                return ValidationResult::failure("State version must be incremented");
            }
        }

        ValidationResult::success()
    }

    /// Validate transition metadata
    fn validate_metadata(&self, transition: &StateTransition) -> ValidationResult {
        // Check if required metadata fields are provided
        if transition.metadata.initiator.is_empty() {
            return ValidationResult::failure("Initiator cannot be empty");
        }

        if transition.metadata.reason.is_empty() {
            return ValidationResult::failure("Transition reason cannot be empty");
        }

        ValidationResult::success()
    }

    /// Validate transition timestamp
    fn validate_timestamp(&self, transition: &StateTransition) -> ValidationResult {
        // Check if timestamp exists
        if transition.timestamp == 0 {
            return ValidationResult::failure("Invalid timestamp");
        }

        // In a real system, we would compare to current time
        // For simplicity, we'll just ensure the timestamp is reasonable
        // This is a placeholder for an actual implementation
        if transition.timestamp < self.min_timestamp_delta {
            return ValidationResult::failure(&format!("Timestamp must be at least {}", self.min_timestamp_delta));
        }

        ValidationResult::success()
    }

    /// Validate that the initiator is authorized
    fn validate_initiator(&self, transition: &StateTransition) -> ValidationResult {
        if !self.authorized_initiators.contains(&transition.metadata.initiator) {
            return ValidationResult::failure(&format!("Initiator '{}' is not authorized", transition.metadata.initiator));
        }

        ValidationResult::success()
    }

    // Helper methods for testing and configuration

    /// Add an authorized initiator
    pub fn add_authorized_initiator(&mut self, initiator: &str) {
        self.authorized_initiators.push(initiator.to_string());
    }

    /// Set strict version increment mode
    pub fn set_strict_version_increment(&mut self, strict: bool) {
        self.strict_version_increment = strict;
    }

    /// Set minimum timestamp delta
    pub fn set_min_timestamp_delta(&mut self, delta: u64) {
        self.min_timestamp_delta = delta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finalizer::lib::{State, TransitionMetadata, generate_unique_id};

    fn create_valid_transition() -> StateTransition {
        StateTransition::new(
            generate_unique_id("trans"),
            State {
                data: "old_state".to_string(),
                version: 1,
            },
            State {
                data: "new_state".to_string(),
                version: 2,
            },
            TransitionMetadata {
                initiator: "test_user".to_string(),
                reason: "test_reason".to_string(),
                additional_info: None,
            },
        )
    }

    #[test]
    fn test_valid_transition() {
        let validator = FinalityValidator::new();
        let transition = create_valid_transition();

        let result = validator.validate_transition(&transition);

        assert!(result.is_valid);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_invalid_version_increment() {
        let validator = FinalityValidator::new();
        let mut transition = create_valid_transition();

        // Set incorrect version increment
        transition.state_after.version = transition.state_before.version + 2;

        let result = validator.validate_transition(&transition);

        assert!(!result.is_valid);
        assert!(result.error_message.unwrap().contains("version"));
    }

    #[test]
    fn test_non_strict_version_increment() {
        let mut validator = FinalityValidator::new();
        validator.set_strict_version_increment(false);

        let mut transition = create_valid_transition();
        // Set version increment to +3 instead of +1
        transition.state_after.version = transition.state_before.version + 3;

        let result = validator.validate_transition(&transition);

        // Should pass with non-strict version increment
        assert!(result.is_valid);
    }

    #[test]
    fn test_empty_state_data() {
        let validator = FinalityValidator::new();
        let mut transition = create_valid_transition();

        // Set empty state data
        transition.state_after.data = "".to_string();

        let result = validator.validate_transition(&transition);

        assert!(!result.is_valid);
        assert!(result.error_message.unwrap().contains("empty"));
    }

    #[test]
    fn test_missing_metadata() {
        let validator = FinalityValidator::new();
        let mut transition = create_valid_transition();

        // Set empty initiator
        transition.metadata.initiator = "".to_string();

        let result = validator.validate_transition(&transition);

        assert!(!result.is_valid);
        assert!(result.error_message.unwrap().contains("Initiator"));
    }

    #[test]
    fn test_unauthorized_initiator() {
        let validator = FinalityValidator::new();
        let mut transition = create_valid_transition();

        // Set unauthorized initiator
        transition.metadata.initiator = "unauthorized_user".to_string();

        let result = validator.validate_transition(&transition);

        assert!(!result.is_valid);
        assert!(result.error_message.unwrap().contains("not authorized"));
    }

    #[test]
    fn test_add_authorized_initiator() {
        let mut validator = FinalityValidator::new();
        let mut transition = create_valid_transition();

        // Set initially unauthorized initiator
        transition.metadata.initiator = "new_user".to_string();

        // Should fail first
        let result1 = validator.validate_transition(&transition);
        assert!(!result1.is_valid);

        // Add initiator to authorized list
        validator.add_authorized_initiator("new_user");

        // Should pass now
        let result2 = validator.validate_transition(&transition);
        assert!(result2.is_valid);
    }

    #[test]
    fn test_custom_validator_config() {
        // Create validator with custom configuration
        let validator = FinalityValidator::with_config(
            1000,                             // min_timestamp_delta
            false,                            // strict_version_increment
            vec!["special_user".to_string()], // authorized_initiators
        );

        let mut transition = create_valid_transition();
        transition.metadata.initiator = "special_user".to_string();
        transition.state_after.version = transition.state_before.version + 5; // Skip versions

        // Should pass with custom config
        let result = validator.validate_transition(&transition);
        assert!(result.is_valid);
    }
}
