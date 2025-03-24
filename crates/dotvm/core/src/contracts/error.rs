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

//! Error types for contract processing pipeline

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("Failed to split contract: {0}")]
    SplittingFailed(String),

    #[error("Failed to resolve dependencies: {0}")]
    DependencyResolutionFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(ValidationError),

    #[error("Scheduling failed: {0}")]
    SchedulingFailed(String),
}

#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid field value: {0} - {1}")]
    InvalidFieldValue(String, String),

    #[error("Empty content in segment")]
    EmptyContent,

    #[error("Content too short: {0} characters (minimum {1})")]
    ContentTooShort(usize, usize),

    #[error("Content too long: {0} characters (maximum {1})")]
    ContentTooLong(usize, usize),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Validation rule failed: {0}")]
    RuleFailed(String),

    #[error("Multiple validation errors: {0}")]
    Multiple(String),
}
