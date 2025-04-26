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

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a state change between two system states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]

pub struct StateTransition {
    pub id: String,                   // Unique transition identifier
    pub state_before: State,          // System state before transition
    pub state_after: State,           // System state after transition
    pub timestamp: u64,               // Millisecond timestamp of transition
    pub metadata: TransitionMetadata, // Contextual information about the transition
}

impl StateTransition {
    /// Creates a new state transition with auto-generated timestamp
    pub fn new(id: String, state_before: State, state_after: State, metadata: TransitionMetadata) -> Self {
        Self {
            id,
            state_before,
            state_after,
            timestamp: generate_timestamp(), // Uses system time for timestamp
            metadata,
        }
    }
}

/// Snapshot of system state at a specific version
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct State {
    pub data: String, // Serialized state data (JSON/protobuf placeholder)
    pub version: u64, // Monotonically increasing version number
}

/// Metadata associated with a state transition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TransitionMetadata {
    pub initiator: String,               // Entity triggering the transition
    pub reason: String,                  // Business/technical reason for change
    pub additional_info: Option<String>, // Optional free-form context
}

/// Validation result containing success status and optional error message
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

/// Result of validation checks for state transitions
impl ValidationResult {
    /// Create successful validation result
    pub fn success() -> Self {
        Self { is_valid: true, error_message: None }
    }

    /// Create failed validation result with message
    pub fn failure(message: &str) -> Self {
        Self {
            is_valid: false,
            error_message: Some(message.to_string()),
        }
    }
}

/// Lifecycle stage of a state transition's finality
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FinalityStatus {
    Pending,   // Initial unconfirmed state
    Validated, // Passed initial checks
    Finalized, // Irreversibly committed
    Failed,    // Rejected by system
}

impl fmt::Display for FinalityStatus {
    /// Human-readable status representation
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinalityStatus::Pending => write!(f, "PENDING"),
            FinalityStatus::Validated => write!(f, "VALIDATED"),
            FinalityStatus::Finalized => write!(f, "FINALIZED"),
            FinalityStatus::Failed => write!(f, "FAILED"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FinalityError {
    #[error("Validation failed: {0}")] // Validation rule violation
    Validation(String),

    #[error("Already finalized")] // Illegal operation on finalized state
    AlreadyFinalized,

    #[error("Internal error")] // System-level failures
    Internal(#[from] std::io::Error),
}

/// Standard Result type for finality-related operations
pub type FinalityResult<T> = Result<T, FinalityError>;

/// Generate UNIX timestamp in milliseconds
pub fn generate_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Create unique identifier with prefix-timestamp-random pattern
pub fn generate_unique_id(prefix: &str) -> String {
    let timestamp = generate_timestamp();
    let random_part = rand::random::<u16>();
    format!("{}-{}-{}", prefix, timestamp, random_part)
}
