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

// Purpose: Implements the non-empty content validation rule.

use crate::dots::DotSegment;
use crate::dots::error::ValidationError;
use crate::dots::validation::{ValidationResult, ValidationRule};

/// Ensures dot segment content is non-empty.
pub struct NonEmptyContentRule;

impl ValidationRule for NonEmptyContentRule {
    /// Checks content.trim().is_empty()
    fn validate(&self, segment: &DotSegment) -> ValidationResult {
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
