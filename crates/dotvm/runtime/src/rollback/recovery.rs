use crate::rollback::checkpoint::CheckpointManager;
use crate::rollback::lib::{LogLevel, RollbackError, RollbackResult, SystemState, log_event};
use crate::rollback::state::StateRollback;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Result of a recovery operation
#[derive(Debug, Clone)]
pub enum RecoveryResult {
    /// Recovery was successful
    Success,
    /// Recovery was successful but with warnings
    PartialSuccess(Vec<String>),
    /// Recovery failed
    Failed(String),
}

/// Represents a transaction that can be reapplied during recovery
#[derive(Clone)]
pub struct Transaction {
    /// Unique identifier for the transaction
    pub id: String,
    /// Function to reapply the transaction
    pub apply_fn: Arc<dyn Fn(&mut SystemState) -> RollbackResult<()> + Send + Sync>,
    /// Indicates if this transaction is critical (must be reapplied) or can be skipped
    pub is_critical: bool,
}

/// Manager responsible for system recovery operations
pub struct RecoveryManager {
    checkpoint_manager: Arc<Mutex<dyn CheckpointManager + Send>>,
    rollback_manager: Arc<Mutex<dyn StateRollback + Send>>,
    pending_transactions: Mutex<Vec<Transaction>>,
    recovery_listeners: Mutex<Vec<Box<dyn Fn(RecoveryResult) -> () + Send + Sync>>>,
    system_state: Arc<Mutex<SystemState>>,
}

impl RecoveryManager {
    /// Creates a new RecoveryManager
    pub fn new(checkpoint_manager: Arc<Mutex<dyn CheckpointManager + Send>>, rollback_manager: Arc<Mutex<dyn StateRollback + Send>>, system_state: Arc<Mutex<SystemState>>) -> Self {
        Self {
            checkpoint_manager,
            rollback_manager,
            pending_transactions: Mutex::new(Vec::new()),
            recovery_listeners: Mutex::new(Vec::new()),
            system_state,
        }
    }

    /// Registers a transaction that can be reapplied during recovery
    pub fn register_transaction(&self, id: &str, apply_fn: impl Fn(&mut SystemState) -> RollbackResult<()> + Send + Sync + 'static, is_critical: bool) -> RollbackResult<()> {
        let mut transactions = self
            .pending_transactions
            .lock()
            .map_err(|_| RollbackError::RecoveryFailed("Failed to acquire transactions lock".to_string()))?;

        transactions.push(Transaction {
            id: id.to_string(),
            apply_fn: Arc::new(apply_fn),
            is_critical,
        });

        log_event(
            LogLevel::Info,
            "RecoveryManager",
            &format!("Registered {} transaction: {}", if is_critical { "critical" } else { "non-critical" }, id),
        );

        Ok(())
    }

    /// Adds a listener to be notified of recovery results
    pub fn add_recovery_listener(&self, listener: impl Fn(RecoveryResult) -> () + Send + Sync + 'static) -> RollbackResult<()> {
        let mut listeners = self
            .recovery_listeners
            .lock()
            .map_err(|_| RollbackError::RecoveryFailed("Failed to acquire listeners lock".to_string()))?;

        listeners.push(Box::new(listener));
        Ok(())
    }

    /// Clears all pending transactions
    pub fn clear_pending_transactions(&self) -> RollbackResult<()> {
        let mut transactions = self
            .pending_transactions
            .lock()
            .map_err(|_| RollbackError::RecoveryFailed("Failed to acquire transactions lock".to_string()))?;

        transactions.clear();
        log_event(LogLevel::Info, "RecoveryManager", "Cleared all pending transactions");

        Ok(())
    }

    /// Notifies all listeners of a recovery result
    fn notify_listeners(&self, result: RecoveryResult) {
        if let Ok(listeners) = self.recovery_listeners.lock() {
            for listener in listeners.iter() {
                listener(result.clone());
            }
        }
    }

    /// Recovers the system from a specific checkpoint
    pub fn recover_from_checkpoint(&self, checkpoint_id: &str) -> RecoveryResult {
        log_event(LogLevel::Info, "RecoveryManager", &format!("Starting recovery from checkpoint: {}", checkpoint_id));

        // Step 1: Get the checkpoint
        let checkpoint_manager = match self.checkpoint_manager.lock() {
            Ok(manager) => manager,
            Err(_) => {
                let error_msg = "Failed to acquire checkpoint manager lock".to_string();
                log_event(LogLevel::Error, "RecoveryManager", &error_msg);
                return RecoveryResult::Failed(error_msg);
            }
        };

        let checkpoint = match checkpoint_manager.get_checkpoint(checkpoint_id) {
            Ok(cp) => cp,
            Err(e) => {
                let error_msg = format!("Failed to get checkpoint: {}", e);
                log_event(LogLevel::Error, "RecoveryManager", &error_msg);
                return RecoveryResult::Failed(error_msg);
            }
        };

        // Step 2: Perform rollback to the checkpoint
        drop(checkpoint_manager); // Release lock before acquiring next one

        let mut rollback_manager = match self.rollback_manager.lock() {
            Ok(manager) => manager,
            Err(_) => {
                let error_msg = "Failed to acquire rollback manager lock".to_string();
                log_event(LogLevel::Error, "RecoveryManager", &error_msg);
                return RecoveryResult::Failed(error_msg);
            }
        };

        if let Err(e) = rollback_manager.rollback_to_checkpoint(&checkpoint.id) {
            let error_msg = format!("Failed to rollback to checkpoint: {}", e);
            log_event(LogLevel::Error, "RecoveryManager", &error_msg);
            return RecoveryResult::Failed(error_msg);
        }

        drop(rollback_manager); // Release lock before next step

        // Step 3: Reapply non-critical transactions if safe
        let transactions = match self.pending_transactions.lock() {
            Ok(txs) => txs.clone(),
            Err(_) => {
                let error_msg = "Failed to acquire transactions lock".to_string();
                log_event(LogLevel::Error, "RecoveryManager", &error_msg);
                return RecoveryResult::Failed(error_msg);
            }
        };

        let mut warnings = Vec::new();

        // Update system state with checkpoint state
        {
            let mut system_state = match self.system_state.lock() {
                Ok(state) => state,
                Err(_) => {
                    let error_msg = "Failed to acquire system state lock".to_string();
                    log_event(LogLevel::Error, "RecoveryManager", &error_msg);
                    return RecoveryResult::Failed(error_msg);
                }
            };

            // Apply checkpoint state
            *system_state = checkpoint.state.clone();

            // Reapply transactions
            for tx in transactions.iter() {
                match (tx.apply_fn)(&mut system_state) {
                    Ok(_) => {
                        log_event(LogLevel::Info, "RecoveryManager", &format!("Reapplied transaction: {}", tx.id));
                    }
                    Err(e) => {
                        let warning = format!("Failed to reapply transaction {}: {}", tx.id, e);
                        log_event(LogLevel::Warning, "RecoveryManager", &warning);

                        if tx.is_critical {
                            log_event(LogLevel::Error, "RecoveryManager", "Critical transaction failed to apply, recovery cannot continue");
                            return RecoveryResult::Failed(warning);
                        } else {
                            warnings.push(warning);
                        }
                    }
                }
            }
        }

        // Step 4: Recovery completed
        if warnings.is_empty() {
            log_event(LogLevel::Info, "RecoveryManager", "Recovery completed successfully");
            let result = RecoveryResult::Success;
            self.notify_listeners(result.clone());
            result
        } else {
            log_event(LogLevel::Warning, "RecoveryManager", &format!("Recovery completed with {} warnings", warnings.len()));
            let result = RecoveryResult::PartialSuccess(warnings);
            self.notify_listeners(result.clone());
            result
        }
    }

    /// Automates recovery by finding the latest valid checkpoint
    pub fn auto_recover(&self) -> RecoveryResult {
        log_event(LogLevel::Info, "RecoveryManager", "Starting automatic recovery");

        // Find the latest checkpoint
        let checkpoint_manager = match self.checkpoint_manager.lock() {
            Ok(manager) => manager,
            Err(_) => {
                let error_msg = "Failed to acquire checkpoint manager lock".to_string();
                log_event(LogLevel::Error, "RecoveryManager", &error_msg);
                return RecoveryResult::Failed(error_msg);
            }
        };

        let latest_checkpoint_id = match checkpoint_manager.get_latest_checkpoint_id() {
            Some(id) => id,
            None => {
                let error_msg = "No checkpoints available for recovery".to_string();
                log_event(LogLevel::Error, "RecoveryManager", &error_msg);
                return RecoveryResult::Failed(error_msg);
            }
        };

        drop(checkpoint_manager); // Release lock before next call

        // Perform recovery from the latest checkpoint
        self.recover_from_checkpoint(&latest_checkpoint_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollback::checkpoint::{Checkpoint, CheckpointManager};
    use crate::rollback::state::{RollbackManager, RollbackTrigger, StateRollback};
    use std::sync::Mutex;

    // Mock implementation for testing
    struct MockCheckpointManager {
        checkpoints: HashMap<String, Checkpoint>,
        /// record which checkpoint was applied
        applied_checkpoint: std::sync::Mutex<Option<String>>,
    }

    impl MockCheckpointManager {
        fn new() -> Self {
            Self {
                checkpoints: HashMap::new(),
                applied_checkpoint: std::sync::Mutex::new(None),
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
            let mut guard = self.applied_checkpoint.lock().unwrap();
            *guard = Some(checkpoint.id.clone());
            Ok(())
        }

        fn delete_checkpoint(&mut self, _id: &str) -> RollbackResult<()> {
            unimplemented!("Not needed for this test")
        }
    }

    struct MockRollbackManager {
        rollback_success: bool,
    }

    impl MockRollbackManager {
        fn new(success: bool) -> Self {
            Self { rollback_success: success }
        }
    }

    impl StateRollback for MockRollbackManager {
        fn rollback_to_checkpoint(&mut self, checkpoint_id: &str) -> RollbackResult<()> {
            if self.rollback_success {
                Ok(())
            } else {
                Err(RollbackError::RollbackFailed(format!("Mock rollback failure for checkpoint {}", checkpoint_id)))
            }
        }

        fn record_transaction(&mut self, _transaction_id: &str, _before_state: SystemState) -> RollbackResult<()> {
            unimplemented!("Not needed for this test")
        }

        fn trigger_rollback(&mut self, _trigger: RollbackTrigger) -> RollbackResult<()> {
            unimplemented!("Not needed for this test")
        }
    }

    #[test]
    fn test_recover_from_checkpoint_success() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let mut test_state = SystemState::new();
        test_state.insert("test_key".to_string(), vec![1, 2, 3]);
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state.clone());

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let rollback_manager = Arc::new(Mutex::new(MockRollbackManager::new(true)));
        let system_state = Arc::new(Mutex::new(SystemState::new()));

        let recovery_manager = RecoveryManager::new(checkpoint_manager, rollback_manager, system_state.clone());

        // Register a transaction
        recovery_manager
            .register_transaction(
                "tx-1",
                |state| {
                    state.insert("new_key".to_string(), vec![4, 5, 6]);
                    Ok(())
                },
                false,
            )
            .unwrap();

        // Test recovery
        let result = recovery_manager.recover_from_checkpoint("test-cp-1");

        // Verify result
        match result {
            RecoveryResult::Success => {
                // Verify system state
                let state = system_state.lock().unwrap();
                assert_eq!(state.get("test_key"), Some(&vec![1, 2, 3]));
                assert_eq!(state.get("new_key"), Some(&vec![4, 5, 6]));
            }
            _ => panic!("Expected success but got another result"),
        }
    }

    #[test]
    fn test_recover_from_checkpoint_rollback_failure() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let test_state = SystemState::new();
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state);

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let rollback_manager = Arc::new(Mutex::new(MockRollbackManager::new(false))); // Will fail rollback
        let system_state = Arc::new(Mutex::new(SystemState::new()));

        let recovery_manager = RecoveryManager::new(checkpoint_manager, rollback_manager, system_state);

        // Test recovery
        let result = recovery_manager.recover_from_checkpoint("test-cp-1");

        // Verify result is failure
        match result {
            RecoveryResult::Failed(_) => {
                // Expected failure
            }
            _ => panic!("Expected failure but got another result"),
        }
    }

    #[test]
    fn test_recover_with_failed_transaction() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let test_state = SystemState::new();
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state);

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let rollback_manager = Arc::new(Mutex::new(MockRollbackManager::new(true)));
        let system_state = Arc::new(Mutex::new(SystemState::new()));

        let recovery_manager = RecoveryManager::new(checkpoint_manager, rollback_manager, system_state);

        // Register a non-critical transaction that will fail
        recovery_manager
            .register_transaction("tx-1", |_state| Err(RollbackError::RecoveryFailed("Test failure".to_string())), false)
            .unwrap();

        // Test recovery
        let result = recovery_manager.recover_from_checkpoint("test-cp-1");

        // Verify result is partial success
        match result {
            RecoveryResult::PartialSuccess(warnings) => {
                assert_eq!(warnings.len(), 1);
                assert!(warnings[0].contains("tx-1"));
            }
            _ => panic!("Expected partial success but got another result"),
        }
    }

    #[test]
    fn test_recover_with_failed_critical_transaction() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let test_state = SystemState::new();
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state);

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let rollback_manager = Arc::new(Mutex::new(MockRollbackManager::new(true)));
        let system_state = Arc::new(Mutex::new(SystemState::new()));

        let recovery_manager = RecoveryManager::new(checkpoint_manager, rollback_manager, system_state);

        // Register a critical transaction that will fail
        recovery_manager
            .register_transaction(
                "tx-1",
                |_state| Err(RollbackError::RecoveryFailed("Test failure".to_string())),
                true, // Critical
            )
            .unwrap();

        // Test recovery
        let result = recovery_manager.recover_from_checkpoint("test-cp-1");

        // Verify result is failure
        match result {
            RecoveryResult::Failed(_) => {
                // Expected failure due to critical transaction
            }
            _ => panic!("Expected failure but got another result"),
        }
    }

    #[test]
    fn test_auto_recover() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let mut test_state = SystemState::new();
        test_state.insert("test_key".to_string(), vec![1, 2, 3]);
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state.clone());

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let rollback_manager = Arc::new(Mutex::new(MockRollbackManager::new(true)));
        let system_state = Arc::new(Mutex::new(SystemState::new()));

        let recovery_manager = RecoveryManager::new(checkpoint_manager, rollback_manager, system_state.clone());

        // Test auto-recovery
        let result = recovery_manager.auto_recover();

        // Verify result
        match result {
            RecoveryResult::Success => {
                // Verify system state
                let state = system_state.lock().unwrap();
                assert_eq!(state.get("test_key"), Some(&vec![1, 2, 3]));
            }
            _ => panic!("Expected success but got another result"),
        }
    }

    #[test]
    fn test_recovery_listeners() {
        // Setup
        let mut mock_checkpoint_manager = MockCheckpointManager::new();

        // Create a test checkpoint
        let test_state = SystemState::new();
        mock_checkpoint_manager.add_test_checkpoint("test-cp-1", test_state);

        let checkpoint_manager = Arc::new(Mutex::new(mock_checkpoint_manager));
        let rollback_manager = Arc::new(Mutex::new(MockRollbackManager::new(true)));
        let system_state = Arc::new(Mutex::new(SystemState::new()));

        let recovery_manager = RecoveryManager::new(checkpoint_manager, rollback_manager, system_state);

        // Set up listener result tracking
        let listener_called = Arc::new(Mutex::new(false));
        let listener_called_clone = listener_called.clone();

        // Add listener
        recovery_manager
            .add_recovery_listener(move |result| {
                let mut called = listener_called_clone.lock().unwrap();
                *called = true;

                match result {
                    RecoveryResult::Success => {}
                    _ => panic!("Expected success result in listener"),
                }
            })
            .unwrap();

        // Test recovery
        recovery_manager.recover_from_checkpoint("test-cp-1");

        // Verify listener was called
        let called = *listener_called.lock().unwrap();
        assert!(called);
    }
}
