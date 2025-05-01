use crate::rollback::checkpoint::CheckpointManager;
use crate::rollback::lib::{LogLevel, RollbackError, RollbackResult, SystemState, log_event};
use std::sync::{Arc, Mutex};

/// Triggers that can initiate a rollback operation
#[derive(Debug, Clone)]
pub enum RollbackTrigger {
    /// Error detection triggered rollback
    ErrorDetected(String),
    /// Manual intervention triggered rollback
    ManualIntervention,
    /// System inconsistency detected
    InconsistencyDetected(String),
}

/// Trait that defines state rollback capabilities
pub trait StateRollback {
    /// Rolls back the system state to a specified checkpoint
    fn rollback_to_checkpoint(&mut self, checkpoint_id: &str) -> RollbackResult<()>;

    /// Records a transaction that can later be used for rollback
    fn record_transaction(&mut self, transaction_id: &str, before_state: SystemState) -> RollbackResult<()>;

    /// Initiates a rollback based on a specific trigger
    fn trigger_rollback(&mut self, trigger: RollbackTrigger) -> RollbackResult<()>;
}

/// Manager responsible for handling state rollback operations
pub struct RollbackManager {
    checkpoint_manager: Arc<Mutex<dyn CheckpointManager + Send>>,
    transaction_log: Mutex<Vec<(String, SystemState)>>,
    last_rollback_trigger: Mutex<Option<RollbackTrigger>>,
    max_transaction_log_size: usize,
}

impl RollbackManager {
    /// Creates a new RollbackManager with the specified checkpoint manager
    pub fn new(checkpoint_manager: Arc<Mutex<dyn CheckpointManager + Send>>) -> Self {
        Self {
            checkpoint_manager,
            transaction_log: Mutex::new(Vec::new()),
            last_rollback_trigger: Mutex::new(None),
            max_transaction_log_size: 1000, // Default transaction log size
        }
    }

    /// Sets the maximum size of the transaction log
    pub fn set_max_transaction_log_size(&mut self, size: usize) {
        self.max_transaction_log_size = size;
    }

    /// Gets the most recent rollback trigger
    pub fn get_last_rollback_trigger(&self) -> Option<RollbackTrigger> {
        self.last_rollback_trigger.lock().unwrap().clone()
    }

    /// Clears the transaction log
    pub fn clear_transaction_log(&self) -> RollbackResult<()> {
        let mut log = self
            .transaction_log
            .lock()
            .map_err(|_| RollbackError::RollbackFailed("Failed to acquire transaction log lock".to_string()))?;
        log.clear();
        log_event(LogLevel::Info, "StateRollback", "Transaction log cleared");
        Ok(())
    }

    /// Handles maintenance of the transaction log size
    fn maintain_transaction_log_size(&self) -> RollbackResult<()> {
        let mut log = self
            .transaction_log
            .lock()
            .map_err(|_| RollbackError::RollbackFailed("Failed to acquire transaction log lock".to_string()))?;

        if log.len() > self.max_transaction_log_size {
            // Doğrudan drain kullan
            let start = log.len() - self.max_transaction_log_size;
            let drained: Vec<_> = log.drain(start..).collect();
            // Keep only the latest entries up to max size
            *log = drained;
            log_event(LogLevel::Info, "StateRollback", "Transaction log trimmed to max size");
        }

        Ok(())
    }
}

impl StateRollback for RollbackManager {
    fn rollback_to_checkpoint(&mut self, checkpoint_id: &str) -> RollbackResult<()> {
        log_event(LogLevel::Info, "StateRollback", &format!("Rolling back to checkpoint: {}", checkpoint_id));

        // Get checkpoint manager and attempt to retrieve the checkpoint
        let checkpoint_manager = self
            .checkpoint_manager
            .lock()
            .map_err(|_| RollbackError::RollbackFailed("Failed to acquire checkpoint manager lock".to_string()))?;

        // Retrieve and apply the checkpoint
        let checkpoint = checkpoint_manager
            .get_checkpoint(checkpoint_id)
            .map_err(|e| RollbackError::RollbackFailed(format!("Failed to get checkpoint: {}", e)))?;

        checkpoint_manager
            .apply_checkpoint(&checkpoint)
            .map_err(|e| RollbackError::RollbackFailed(format!("Failed to apply checkpoint: {}", e)))?;

        log_event(LogLevel::Info, "StateRollback", &format!("Successfully rolled back to checkpoint: {}", checkpoint_id));

        // Clear transaction log after successful rollback
        self.clear_transaction_log()?;

        Ok(())
    }

    fn record_transaction(&mut self, transaction_id: &str, before_state: SystemState) -> RollbackResult<()> {
        let mut log = self
            .transaction_log
            .lock()
            .map_err(|_| RollbackError::RollbackFailed("Failed to acquire transaction log lock".to_string()))?;

        log.push((transaction_id.to_string(), before_state));
        log_event(LogLevel::Info, "StateRollback", &format!("Recorded transaction: {}", transaction_id));

        // Maintain transaction log size
        drop(log); // Release lock before calling other methods
        self.maintain_transaction_log_size()?;

        Ok(())
    }

    fn trigger_rollback(&mut self, trigger: RollbackTrigger) -> RollbackResult<()> {
        // Store the trigger
        {
            let mut last_trigger = self
                .last_rollback_trigger
                .lock()
                .map_err(|_| RollbackError::RollbackFailed("Failed to acquire trigger lock".to_string()))?;
            *last_trigger = Some(trigger.clone());
        }

        // Log the rollback trigger
        let trigger_msg = match &trigger {
            RollbackTrigger::ErrorDetected(err) => format!("Error detected: {}", err),
            RollbackTrigger::ManualIntervention => "Manual intervention".to_string(),
            RollbackTrigger::InconsistencyDetected(details) => format!("Inconsistency detected: {}", details),
        };

        log_event(LogLevel::Warning, "StateRollback", &format!("Rollback triggered: {}", trigger_msg));

        // Get checkpoint manager to find the most recent checkpoint
        let checkpoint_manager = self
            .checkpoint_manager
            .lock()
            .map_err(|_| RollbackError::RollbackFailed("Failed to acquire checkpoint manager lock".to_string()))?;

        let latest_checkpoint_id = checkpoint_manager
            .get_latest_checkpoint_id()
            .ok_or_else(|| RollbackError::RollbackFailed("No checkpoints available for rollback".to_string()))?;

        drop(checkpoint_manager); // Release lock before calling rollback

        // Perform the actual rollback
        self.rollback_to_checkpoint(&latest_checkpoint_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollback::checkpoint::{Checkpoint, CheckpointManager};
    use std::collections::HashMap;

    // Mock implementation of CheckpointManager for testing
    struct MockCheckpointManager {
        checkpoints: HashMap<String, Checkpoint>,
        applied_checkpoint: Option<String>,
    }

    impl MockCheckpointManager {
        fn new() -> Self {
            Self {
                checkpoints: HashMap::new(),
                applied_checkpoint: None,
            }
        }

        fn add_test_checkpoint(&mut self, id: &str, state: SystemState) {
            self.checkpoints.insert(
                id.to_string(),
                Checkpoint {
                    id: id.to_string(),
                    timestamp: 12345,
                    state,
                },
            );
        }

        fn get_applied_checkpoint(&self) -> Option<String> {
            self.applied_checkpoint.clone()
        }
    }

    impl CheckpointManager for MockCheckpointManager {
        fn create_checkpoint(&mut self, _state: SystemState) -> RollbackResult<Checkpoint> {
            unimplemented!("Not needed for this test")
        }

        fn get_checkpoint(&self, id: &str) -> RollbackResult<Checkpoint> {
            self.checkpoints.get(id).cloned().ok_or_else(|| RollbackError::CheckpointNotFound(id.to_string()))
        }

        fn get_latest_checkpoint_id(&self) -> Option<String> {
            self.checkpoints.keys().last().cloned()
        }

        fn apply_checkpoint(&self, checkpoint: &Checkpoint) -> RollbackResult<()> {
            // In a real implementation, this would apply the state to the system
            let mut manager = self as *const Self as *mut MockCheckpointManager;
            // This is unsafe but acceptable for test purposes
            unsafe {
                (*manager).applied_checkpoint = Some(checkpoint.id.clone());
            }
            Ok(())
        }

        fn delete_checkpoint(&mut self, _id: &str) -> RollbackResult<()> {
            unimplemented!("Not needed for this test")
        }
    }

    #[test]
    fn test_rollback_to_checkpoint() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let mut test_state = SystemState::new();
        test_state.insert("test_key".to_string(), vec![1, 2, 3]);
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state);

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let mut rollback_manager = RollbackManager::new(checkpoint_manager.clone());

        // Test rollback
        let result = rollback_manager.rollback_to_checkpoint("test-cp-1");
        assert!(result.is_ok());

        // Verify checkpoint was applied
        let mock_cp_manager = checkpoint_manager.lock().unwrap();
        assert_eq!(mock_cp_manager.get_applied_checkpoint(), Some("test-cp-1".to_string()));
    }

    #[test]
    fn test_record_transaction() {
        // Setup
        let mock_checkpoint_manager = MockCheckpointManager::new();
        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let mut rollback_manager = RollbackManager::new(checkpoint_manager);

        // Test recording a transaction
        let mut test_state = SystemState::new();
        test_state.insert("test_key".to_string(), vec![1, 2, 3]);

        let result = rollback_manager.record_transaction("tx-1", test_state);
        assert!(result.is_ok());

        // Verify transaction was recorded
        let log = rollback_manager.transaction_log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].0, "tx-1");
    }

    #[test]
    fn test_trigger_rollback() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let mut test_state = SystemState::new();
        test_state.insert("test_key".to_string(), vec![1, 2, 3]);
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state);

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let mut rollback_manager = RollbackManager::new(checkpoint_manager.clone());

        // Test trigger rollback
        let trigger = RollbackTrigger::ErrorDetected("Test error".to_string());
        let result = rollback_manager.trigger_rollback(trigger);

        assert!(result.is_ok());

        // Verify trigger was stored
        let last_trigger = rollback_manager.get_last_rollback_trigger();
        assert!(last_trigger.is_some());

        // Verify checkpoint was applied
        let guard = checkpoint_manager.lock().unwrap();
        let any = &*guard as &dyn std::any::Any;
        let mock = any.downcast_ref::<MockCheckpointManager>().unwrap();
        assert_eq!(mock.get_applied_checkpoint(), Some("test-cp-1".to_string()));
    }

    #[test]
    fn test_transaction_log_size_maintenance() {
        // Setup
        let mock_checkpoint_manager = MockCheckpointManager::new();
        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let mut rollback_manager = RollbackManager::new(checkpoint_manager);

        // Set small max log size
        rollback_manager.set_max_transaction_log_size(2);

        // Add multiple transactions
        let test_state = SystemState::new();

        rollback_manager.record_transaction("tx-1", test_state.clone()).unwrap();
        rollback_manager.record_transaction("tx-2", test_state.clone()).unwrap();
        rollback_manager.record_transaction("tx-3", test_state.clone()).unwrap();

        // Verify log was trimmed
        let log = rollback_manager.transaction_log.lock().unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].0, "tx-2");
        assert_eq!(log[1].0, "tx-3");
    }
}
