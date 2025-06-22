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

//! Validation framework for dot segments
//!
//! Enforces content quality and structural rules through composable validation rules

use crate::dots::DotSegment;
use crate::dots::error::ValidationError;
use std::collections::HashMap;

pub mod rules;

/// Aggregates results from multiple validation checks
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the validation passed
    pub is_valid: bool,

    /// Errors that occurred during validation, if any
    pub errors: Vec<ValidationError>,

    /// Warnings that were generated during validation
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Creates successful validation result
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Creates failed result with single error
    pub fn failure(error: ValidationError) -> Self {
        Self {
            is_valid: false,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }

    /// Appends warning to result
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Check if the validation result has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Merges multiple validation results
    pub fn combine(&self, other: &ValidationResult) -> Self {
        let mut combined = Self {
            is_valid: self.is_valid && other.is_valid,
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        };

        combined.errors.extend(other.errors.clone());
        combined.warnings.extend(other.warnings.clone());

        combined
    }
}

/// Interface for validation rules targeting dot segments
pub trait ValidationRule {
    /// Executes validation logic against a segment
    fn validate(&self, segment: &DotSegment) -> ValidationResult;

    /// Returns machine-readable rule identifier
    fn name(&self) -> &str;
}

/// Central validator applying multiple rules to dot segments
pub struct Validator {
    rules: HashMap<String, Box<dyn ValidationRule>>,
}

impl Validator {
    /// Creates validator with default rules:
    /// - Non-empty content
    /// - Min 10 characters
    /// - Max 10,000 characters
    pub fn new() -> Self {
        let mut validator = Self { rules: HashMap::new() };

        validator.add_rule(Box::new(rules::non_empty_content::NonEmptyContentRule));
        validator.add_rule(Box::new(rules::min_length::MinContentLengthRule::new(10)));
        validator.add_rule(Box::new(rules::max_length::MaxContentLengthRule::new(10000)));

        validator
    }

    /// Adds custom validation rule
    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.insert(rule.name().to_string(), rule);
    }

    /// Remove a validation rule
    pub fn remove_rule(&mut self, rule_name: &str) {
        self.rules.remove(rule_name);
    }

    /// Validates a dot segment against all rules
    ///
    /// # Returns
    /// - Ok(()): All rules passed
    /// - Err(ValidationError): First failed rule's error, or a combined error if multiple failed.
    pub fn validate(&self, segment: &DotSegment) -> Result<(), ValidationError> {
        let mut result = ValidationResult::success();

        for rule in self.rules.values() {
            let rule_result = rule.validate(segment);
            result = result.combine(&rule_result);
        }

        if !result.is_valid {
            if result.errors.len() == 1 {
                Err(result.errors[0].clone())
            } else {
                let error_messages: Vec<String> = result.errors.iter().map(|e| format!("{}", e)).collect();

                Err(ValidationError::Multiple(error_messages.join("; ")))
            }
        } else {
            Ok(())
        }
    }

    /// Returns detailed validation results
    pub fn validate_with_details(&self, segment: &DotSegment) -> ValidationResult {
        let mut result = ValidationResult::success();

        for rule in self.rules.values() {
            let rule_result = rule.validate(segment);
            result = result.combine(&rule_result);
        }

        result
    }

    /// Helper function to determine the final Result<(), ValidationError> from ValidationResult.
    fn prepare_validation_outcome(result: &ValidationResult) -> Result<(), ValidationError> {
        if result.is_valid {
            Ok(())
        } else {
            if result.errors.is_empty() {
                // This case should ideally not happen if is_valid is false,
                // but as a safeguard, return a generic error.
                Err(ValidationError::RuleFailed("Validation failed with no specific errors".to_string()))
            } else if result.errors.len() == 1 {
                Err(result.errors[0].clone())
            } else {
                let error_messages: Vec<String> = result.errors.iter().map(|e| format!("{}", e)).collect();
                Err(ValidationError::Multiple(error_messages.join("; ")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rules::min_length::MinContentLengthRule;
    use rules::non_empty_content::NonEmptyContentRule;
    use rules::required_metadata::RequiredMetadataRule;

    #[test]
    fn test_non_empty_content_rule() {
        let rule = NonEmptyContentRule;
        let valid_segment = DotSegment::new("segment-1".to_string(), "dot-001".to_string(), "SECTION".to_string(), "This is valid content".to_string(), 0);
        let invalid_segment = DotSegment::new("segment-2".to_string(), "dot-001".to_string(), "SECTION".to_string(), " ".to_string(), 1); // Content with only spaces
        let invalid_segment_empty = DotSegment::new("segment-3".to_string(), "dot-001".to_string(), "SECTION".to_string(), "".to_string(), 2);

        let valid_result = rule.validate(&valid_segment);
        let invalid_result = rule.validate(&invalid_segment);
        let invalid_result_empty = rule.validate(&invalid_segment_empty);

        assert!(valid_result.is_valid);
        assert!(!invalid_result.is_valid, "Validation should fail for content with only spaces");
        assert!(!invalid_result_empty.is_valid, "Validation should fail for empty content");
    }

    #[test]
    fn test_min_content_length_rule() {
        let rule = MinContentLengthRule::new(10);
        let valid_segment = DotSegment::new("segment-1".to_string(), "dot-001".to_string(), "SECTION".to_string(), "This is long enough content".to_string(), 0); // Length 27
        let invalid_segment = DotSegment::new("segment-2".to_string(), "dot-001".to_string(), "SECTION".to_string(), "Too short".to_string(), 1); // Length 9
        let edge_case_segment = DotSegment::new("segment-3".to_string(), "dot-001".to_string(), "SECTION".to_string(), "1234567890".to_string(), 2); // Length 10

        let valid_result = rule.validate(&valid_segment);
        let invalid_result = rule.validate(&invalid_segment);
        let edge_case_result = rule.validate(&edge_case_segment);

        assert!(valid_result.is_valid);
        assert!(!invalid_result.is_valid);
        assert!(edge_case_result.is_valid, "Content with exact minimum length should be valid");
    }

    #[test]
    fn test_validator() {
        let mut validator = Validator::new(); // Uses default rules (NonEmpty, MinLength(10), MaxLength(10000))

        // Add a special rule for required metadata
        validator.add_rule(Box::new(RequiredMetadataRule::new(vec!["author".to_string(), "version".to_string()])));

        // A segment that should pass all default content rules but is missing metadata
        let mut segment_missing_meta = DotSegment::new(
            "segment-1".to_string(),
            "dot-001".to_string(),
            "SECTION".to_string(),
            "This is valid content, long enough.".to_string(),
            0,
        );
        segment_missing_meta.metadata.insert("author".to_string(), "John Doe".to_string()); // "version" metadata missing

        // Validation must fail due to missing "version" metadata
        let result_missing_meta = validator.validate(&segment_missing_meta);
        assert!(result_missing_meta.is_err(), "Validation should fail due to missing metadata");
        if let Err(ValidationError::MissingField(field)) = result_missing_meta {
            assert_eq!(field, "version");
        } else {
            panic!("Expected MissingField error for version");
        }

        // Add missing metadata
        segment_missing_meta.metadata.insert("version".to_string(), "1.0".to_string());

        // Now the validation should be successful for this segment
        let result_meta_ok = validator.validate(&segment_missing_meta);
        assert!(result_meta_ok.is_ok(), "Validation should pass after adding all required metadata");

        // Test a segment that fails a default rule (e.g., too short)
        let mut segment_too_short = DotSegment::new("segment-2".to_string(), "dot-001".to_string(), "CLAUSE".to_string(), "Short".to_string(), 1);
        // Add required metadata so it only fails the length check by default rules
        segment_too_short.metadata.insert("author".to_string(), "Jane Doe".to_string());
        segment_too_short.metadata.insert("version".to_string(), "0.9".to_string());

        let result_too_short = validator.validate(&segment_too_short);
        assert!(result_too_short.is_err(), "Validation should fail due to short content");
        if let Err(ValidationError::ContentTooShort(_, _)) = result_too_short {
            // Correct error type
        } else {
            panic!("Expected ContentTooShort error");
        }

        // Test a segment that is just empty
        let mut segment_empty_content = DotSegment::new("segment-3".to_string(), "dot-001".to_string(), "TEXT".to_string(), "".to_string(), 2);
        segment_empty_content.metadata.insert("author".to_string(), "Anon".to_string());
        segment_empty_content.metadata.insert("version".to_string(), "0.1".to_string());

        let result_empty_content = validator.validate(&segment_empty_content);
        assert!(result_empty_content.is_err(), "Validation should fail for empty content");
        if let Err(ValidationError::EmptyContent) = result_empty_content {
            // Correct error type
        } else {
            // If multiple errors, it might be ValidationError::Multiple
            // Check if the detailed validation result contains EmptyContent
            let details = validator.validate_with_details(&segment_empty_content);
            assert!(details.errors.iter().any(|e| matches!(e, ValidationError::EmptyContent)));
        }
    }
}
