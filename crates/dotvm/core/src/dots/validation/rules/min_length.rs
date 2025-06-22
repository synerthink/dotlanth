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

// Purpose: Implements the minimum content length validation rule.

use crate::dots::DotSegment;
use crate::dots::error::ValidationError;
use crate::dots::validation::{ValidationResult, ValidationRule};

/// Enforces minimum content length for a dot segment.
pub struct MinContentLengthRule {
    min_length: usize,
}

impl MinContentLengthRule {
    /// Create a new minimum content length rule.
    pub fn new(min_length: usize) -> Self {
        Self { min_length }
    }
}

impl ValidationRule for MinContentLengthRule {
    fn validate(&self, segment: &DotSegment) -> ValidationResult {
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
