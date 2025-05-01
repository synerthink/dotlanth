use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents system state type that can be checkpointed and restored
pub type SystemState = HashMap<String, Vec<u8>>;

/// Generates a unique identifier for checkpoints
pub fn generate_checkpoint_id() -> String {
    // Use full nanoseconds since epoch for uniqueness even within the same millisecond
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let nanos = now.as_secs() as u128 * 1_000_000_000 + now.subsec_nanos() as u128;
    format!("checkpoint-{}", nanos)
}

/// Log levels for the rollback system
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

/// Simple logging function for rollback-related operations
pub fn log_event(level: LogLevel, component: &str, message: &str) {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    let level_str = match level {
        LogLevel::Info => "INFO",
        LogLevel::Warning => "WARNING",
        LogLevel::Error => "ERROR",
    };

    eprintln!("[{}][{}][{}] {}", timestamp, level_str, component, message);
}

/// Represents an error in the rollback system
#[derive(Debug, thiserror::Error)]
pub enum RollbackError {
    #[error("Checkpoint not found: {0}")]
    CheckpointNotFound(String),

    #[error("State storage error: {0}")]
    StateStorageError(String),

    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Rollback operation failed: {0}")]
    RollbackFailed(String),
}

/// Result type for rollback operations
pub type RollbackResult<T> = Result<T, RollbackError>;
