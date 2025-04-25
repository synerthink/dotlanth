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

use crate::finalizer::finality_confirmation::FinalityConfirmation;
use crate::finalizer::lib::{FinalityStatus, StateTransition, ValidationResult, generate_timestamp};
use std::env;
use std::sync::{Mutex, MutexGuard};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

/// Immutable record of a system event for auditing
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub timestamp: u64,         // Event occurrence time
    pub event_type: String,     // PROPOSAL/VALIDATION/FINALIZATION
    pub transition_id: String,  // Associated transition ID
    pub details: String,        // Human-readable description
    pub status: FinalityStatus, // Event outcome status
}

/// Centralized audit logging component
pub struct AuditLogger {
    logs: Mutex<Vec<AuditLogEntry>>, // Thread-safe in-memory log storage
}

impl AuditLogger {
    /// Initialize empty logger
    pub fn new() -> Self {
        Self { logs: Mutex::new(Vec::new()) }
    }

    /// Safely acquire lock with poison handling
    fn get_logs(&self) -> Result<MutexGuard<Vec<AuditLogEntry>>, String> {
        self.logs.lock().map_err(|e| format!("Mutex poisoned: {}. Attempting recovery...", e.to_string()))
    }

    /// Asynchronously append log entry to disk
    async fn persist_log(entry: &AuditLogEntry) -> Result<(), String> {
        let mut log_path = env::temp_dir();
        log_path.push("dotvm-audit.log");

        // Format log line with structured data
        let log_line = format!("[{}] [{}] [{}] {} - {}\n", entry.timestamp, entry.event_type, entry.status, entry.transition_id, entry.details);

        // Async file append operation
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await
            .map_err(|e| format!("Failed to open {}: {}", log_path.display(), e))?
            .write_all(log_line.as_bytes())
            .await
            .map_err(|e| format!("Write to {} failed: {}", log_path.display(), e))
    }

    /// Core logging method with dual in-memory and disk persistence
    fn add_log_entry(&self, event_type: &str, transition_id: &str, details: &str, status: FinalityStatus) {
        let entry = AuditLogEntry {
            timestamp: generate_timestamp(),
            event_type: event_type.to_string(),
            transition_id: transition_id.to_string(),
            details: details.to_string(),
            status,
        };

        // Memory logging with poisoning handling
        match self.get_logs() {
            Ok(mut logs) => logs.push(entry.clone()),
            Err(e) => eprintln!("{}", e), // Recovery logic can be added here
        }

        // Async persistent logging (non-blocking)
        #[cfg(not(test))]
        {
            let entry_clone = entry.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::persist_log(&entry_clone).await {
                    eprintln!("Async logging failed: {}", e);
                }
            });
        }
    }

    /// Log a state transition proposal
    pub fn log_transition_proposal(&self, transition: &StateTransition) {
        self.add_log_entry(
            "PROPOSAL",
            &transition.id,
            &format!(
                "Received state transition proposal: {} -> {} (initiator: {})",
                transition.state_before.version, transition.state_after.version, transition.metadata.initiator
            ),
            FinalityStatus::Pending,
        );
    }

    /// Log a validation result
    pub fn log_validation_result(&self, transition: &StateTransition, result: &ValidationResult) {
        let status = if result.is_valid { FinalityStatus::Validated } else { FinalityStatus::Failed };

        let details = if result.is_valid {
            format!("Validation passed for transition: {}", transition.id)
        } else {
            format!(
                "Validation failed for transition: {}, reason: {}",
                transition.id,
                result.error_message.as_ref().unwrap_or(&"Unknown error".to_string())
            )
        };

        self.add_log_entry("VALIDATION", &transition.id, &details, status);
    }

    /// Log successful finalization
    pub fn log_finalization_success(&self, transition: &StateTransition, confirmation: &FinalityConfirmation) {
        self.add_log_entry(
            "FINALIZATION",
            &transition.id,
            &format!("Successfully finalized transition: {}, confirmation ID: {}", transition.id, confirmation.id),
            FinalityStatus::Finalized,
        );
    }

    /// Log finalization failure
    pub fn log_finalization_failure(&self, transition: &StateTransition, error: &str) {
        self.add_log_entry(
            "FINALIZATION",
            &transition.id,
            &format!("Failed to finalize transition: {}, error: {}", transition.id, error),
            FinalityStatus::Failed,
        );
    }

    /// Get all log entries for a specific transition
    pub fn get_logs_for_transition(&self, transition_id: &str) -> Vec<AuditLogEntry> {
        self.get_logs()
            .map(|logs| logs.iter().filter(|entry| entry.transition_id == transition_id).cloned().collect())
            .unwrap_or_else(|_| Vec::new())
    }

    /// Get all log entries with a specific status
    pub fn get_logs_by_status(&self, status: FinalityStatus) -> Vec<AuditLogEntry> {
        self.get_logs()
            .map(|logs| logs.iter().filter(|entry| entry.status == status).cloned().collect())
            .unwrap_or_else(|_| Vec::new())
    }

    /// Get total log count
    pub fn log_count(&self) -> usize {
        self.get_logs().map(|logs| logs.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finalizer::lib::{State, TransitionMetadata, generate_unique_id};

    fn create_test_transition() -> StateTransition {
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

    fn create_test_confirmation(transition: &StateTransition) -> FinalityConfirmation {
        FinalityConfirmation::new(transition.clone(), FinalityStatus::Finalized, "Test confirmation")
    }

    #[test]
    fn test_log_transition_proposal() {
        let logger = AuditLogger::new();
        let transition = create_test_transition();

        // Log a proposal
        logger.log_transition_proposal(&transition);

        // Verify log entry was created
        let logs = logger.get_logs_for_transition(&transition.id);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event_type, "PROPOSAL");
        assert_eq!(logs[0].status, FinalityStatus::Pending);
    }

    #[test]
    fn test_log_validation_result() {
        let logger = AuditLogger::new();
        let transition = create_test_transition();

        // Log a successful validation
        let success_result = ValidationResult::success();
        logger.log_validation_result(&transition, &success_result);

        // Log a failed validation
        let failure_result = ValidationResult::failure("Test failure");
        logger.log_validation_result(&transition, &failure_result);

        // Verify log entries
        let logs = logger.get_logs_for_transition(&transition.id);
        assert_eq!(logs.len(), 2);

        // Check status of log entries
        let validated_logs = logger.get_logs_by_status(FinalityStatus::Validated);
        let failed_logs = logger.get_logs_by_status(FinalityStatus::Failed);
        assert_eq!(validated_logs.len(), 1);
        assert_eq!(failed_logs.len(), 1);
    }

    #[test]
    fn test_log_finalization() {
        let logger = AuditLogger::new();
        let transition = create_test_transition();
        let confirmation = create_test_confirmation(&transition);

        // Log successful finalization
        logger.log_finalization_success(&transition, &confirmation);

        // Log failed finalization
        logger.log_finalization_failure(&transition, "Test error");

        // Verify log entries
        let logs = logger.get_logs_for_transition(&transition.id);
        assert_eq!(logs.len(), 2);

        // Check finalized logs
        let finalized_logs = logger.get_logs_by_status(FinalityStatus::Finalized);
        assert_eq!(finalized_logs.len(), 1);
        assert!(finalized_logs[0].details.contains("confirmation ID"));
    }

    #[test]
    fn test_log_counts() {
        let logger = AuditLogger::new();
        let transition = create_test_transition();

        // Add multiple log entries
        logger.log_transition_proposal(&transition);
        logger.log_validation_result(&transition, &ValidationResult::success());

        // Verify total count
        assert_eq!(logger.log_count(), 2);
    }
}
