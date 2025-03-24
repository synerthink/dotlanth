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

//! Validation framework for contract segments
//!
//! Enforces content quality and structural rules through composable validation rules

use crate::contracts::ContractSegment;
use crate::contracts::error::ValidationError;
use std::collections::HashMap;

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

/// Interface for validation rules
pub trait ValidationRule {
    /// Executes validation logic against a segment
    fn validate(&self, segment: &ContractSegment) -> ValidationResult;

    /// Returns machine-readable rule identifier
    fn name(&self) -> &str;
}

/// Ensures segment content is non-empty
pub struct NonEmptyContentRule;

impl ValidationRule for NonEmptyContentRule {
    /// Checks content.trim().is_empty()
    fn validate(&self, segment: &ContractSegment) -> ValidationResult {
        if segment.content.trim().is_empty() {
            ValidationResult::failure(ValidationError::EmptyContent)
        } else {
            ValidationResult::success()
        }
    }

    fn name(&self) -> &str {
        "non_empty_content"
    }
}

/// Enforces minimum content length
pub struct MinContentLengthRule {
    min_length: usize,
}

impl MinContentLengthRule {
    /// Create a new minimum content length rule
    pub fn new(min_length: usize) -> Self {
        Self { min_length }
    }
}

impl ValidationRule for MinContentLengthRule {
    fn validate(&self, segment: &ContractSegment) -> ValidationResult {
        let content_length = segment.content.trim().len();
        if content_length < self.min_length {
            ValidationResult::failure(ValidationError::ContentTooShort(content_length, self.min_length))
        } else {
            ValidationResult::success()
        }
    }

    fn name(&self) -> &str {
        "min_content_length"
    }
}

/// Validates that a segment's content does not exceed maximum length requirements
pub struct MaxContentLengthRule {
    max_length: usize,
}

impl MaxContentLengthRule {
    /// Create a new maximum content length rule
    pub fn new(max_length: usize) -> Self {
        Self { max_length }
    }
}

impl ValidationRule for MaxContentLengthRule {
    fn validate(&self, segment: &ContractSegment) -> ValidationResult {
        let content_length = segment.content.trim().len();
        if content_length > self.max_length {
            ValidationResult::failure(ValidationError::ContentTooLong(content_length, self.max_length))
        } else {
            ValidationResult::success()
        }
    }

    fn name(&self) -> &str {
        "max_content_length"
    }
}

/// Validates that a segment has all required metadata fields
pub struct RequiredMetadataRule {
    required_fields: Vec<String>,
}

impl RequiredMetadataRule {
    /// Create a new required metadata rule
    pub fn new(required_fields: Vec<String>) -> Self {
        Self { required_fields }
    }
}

impl ValidationRule for RequiredMetadataRule {
    fn validate(&self, segment: &ContractSegment) -> ValidationResult {
        for field in &self.required_fields {
            if !segment.metadata.contains_key(field) {
                return ValidationResult::failure(ValidationError::MissingField(field.clone()));
            }
        }
        ValidationResult::success()
    }

    fn name(&self) -> &str {
        "required_metadata"
    }
}

/// Central validator applying multiple rules
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

        // Varsayılan kuralları ekle
        validator.add_rule(Box::new(NonEmptyContentRule));
        validator.add_rule(Box::new(MinContentLengthRule::new(10)));
        validator.add_rule(Box::new(MaxContentLengthRule::new(10000)));

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

    /// Validates segment against all rules
    ///
    /// # Returns
    /// - Ok(()): All rules passed
    /// - Err(ValidationError): First failed rule's error
    pub fn validate(&self, segment: &ContractSegment) -> Result<(), ValidationError> {
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
    pub fn validate_with_details(&self, segment: &ContractSegment) -> ValidationResult {
        let mut result = ValidationResult::success();

        for rule in self.rules.values() {
            let rule_result = rule.validate(segment);
            result = result.combine(&rule_result);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_empty_content_rule() {
        let rule = NonEmptyContentRule;
        let valid_segment = ContractSegment::new("segment-1".to_string(), "contract-001".to_string(), "SECTION".to_string(), "This is valid content".to_string(), 0);
        let invalid_segment = ContractSegment::new("segment-2".to_string(), "contract-001".to_string(), "SECTION".to_string(), "".to_string(), 1);
        let valid_result = rule.validate(&valid_segment);
        let invalid_result = rule.validate(&invalid_segment);

        assert!(valid_result.is_valid);
        assert!(!invalid_result.is_valid);
    }

    #[test]
    fn test_min_content_length_rule() {
        let rule = MinContentLengthRule::new(10);
        let valid_segment = ContractSegment::new("segment-1".to_string(), "contract-001".to_string(), "SECTION".to_string(), "This is long enough content".to_string(), 0);
        let invalid_segment = ContractSegment::new("segment-2".to_string(), "contract-001".to_string(), "SECTION".to_string(), "Too short".to_string(), 1);
        let valid_result = rule.validate(&valid_segment);
        let invalid_result = rule.validate(&invalid_segment);

        assert!(valid_result.is_valid);
        assert!(!invalid_result.is_valid);
    }

    #[test]
    fn test_validator() {
        let mut validator = Validator::new();

        // Add a special rule
        validator.add_rule(Box::new(RequiredMetadataRule::new(vec!["author".to_string(), "version".to_string()])));

        // A segment with missing fields in metadata
        let mut segment = ContractSegment::new("segment-1".to_string(), "contract-001".to_string(), "SECTION".to_string(), "This is valid content".to_string(), 0);

        segment.metadata.insert("author".to_string(), "John Doe".to_string()); // “version” metadata missing

        // Verification must fail
        let result = validator.validate(&segment);
        assert!(result.is_err());

        // Add missing metadata
        segment.metadata.insert("version".to_string(), "1.0".to_string());

        // Now the verification should be successful
        let result = validator.validate(&segment);
        assert!(result.is_ok());
    }
}
