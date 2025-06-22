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

// Purpose: Implements the maximum content length validation rule.

use crate::dots::DotSegment;
use crate::dots::error::ValidationError;
use crate::dots::validation::{ValidationResult, ValidationRule};

/// Validates that a dot segment's content does not exceed maximum length requirements.
pub struct MaxContentLengthRule {
    max_length: usize,
}

impl MaxContentLengthRule {
    /// Create a new maximum content length rule.
    pub fn new(max_length: usize) -> Self {
        Self { max_length }
    }
}

impl ValidationRule for MaxContentLengthRule {
    fn validate(&self, segment: &DotSegment) -> ValidationResult {
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
