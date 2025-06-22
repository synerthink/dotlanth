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

// Purpose: Implements the required metadata fields validation rule.

use crate::dots::DotSegment;
use crate::dots::error::ValidationError;
use crate::dots::validation::{ValidationResult, ValidationRule};

/// Validates that a dot segment has all required metadata fields.
pub struct RequiredMetadataRule {
    required_fields: Vec<String>,
}

impl RequiredMetadataRule {
    /// Create a new required metadata rule.
    pub fn new(required_fields: Vec<String>) -> Self {
        Self { required_fields }
    }
}

impl ValidationRule for RequiredMetadataRule {
    fn validate(&self, segment: &DotSegment) -> ValidationResult {
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
