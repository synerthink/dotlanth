use crate::rollback::lib::{LogLevel, RollbackError, RollbackResult, SystemState, log_event};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a checkpoint of the system state at a specific point in time
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Unique identifier for the checkpoint
    pub id: String,
    /// Timestamp when the checkpoint was created
    pub timestamp: u64,
    /// The captured system state
    pub state: SystemState,
}

/// Trait defining checkpoint management capabilities
pub trait CheckpointManager: std::any::Any {
    /// Creates a new checkpoint of the current system state
    fn create_checkpoint(&mut self, state: SystemState) -> RollbackResult<Checkpoint>;

    /// Retrieves a checkpoint by its ID
    fn get_checkpoint(&self, id: &str) -> RollbackResult<Checkpoint>;

    /// Gets the ID of the most recent valid checkpoint
    fn get_latest_checkpoint_id(&self) -> Option<String>;

    /// Applies a checkpoint to restore the system state
    fn apply_checkpoint(&self, checkpoint: &Checkpoint) -> RollbackResult<()>;

    /// Deletes a checkpoint by its ID
    fn delete_checkpoint(&mut self, id: &str) -> RollbackResult<()>;
}

/// Default implementation of CheckpointManager that stores checkpoints in memory
pub struct DefaultCheckpointManager {
    checkpoints: Mutex<HashMap<String, Checkpoint>>,
    /// FnMut wrapped in RefCell so we can call it from &self
    checkpoint_apply_callback: Option<RefCell<Box<dyn FnMut(&Checkpoint) -> RollbackResult<()> + Send + Sync>>>,
    max_checkpoints: usize,
}

impl DefaultCheckpointManager {
    /// Creates a new DefaultCheckpointManager
    pub fn new() -> Self {
        Self {
            checkpoints: Mutex::new(HashMap::new()),
            checkpoint_apply_callback: None,
            max_checkpoints: 10, // Default maximum checkpoints to store
        }
    }

    /// Sets the callback function used when applying a checkpoint
    pub fn set_apply_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&Checkpoint) -> RollbackResult<()> + Send + Sync + 'static,
    {
        // wrap FnMut in a RefCell so we can borrow it mutably later
        self.checkpoint_apply_callback = Some(RefCell::new(Box::new(callback)));
    }

    /// Sets the maximum number of checkpoints to keep in storage
    pub fn set_max_checkpoints(&mut self, max: usize) {
        self.max_checkpoints = max;
    }

    /// Gets all stored checkpoint IDs ordered by creation time
    pub fn list_checkpoints(&self) -> Vec<String> {
        let checkpoints = self.checkpoints.lock().unwrap();
        let mut checkpoint_list: Vec<(&String, &u64)> = checkpoints.iter().map(|(id, checkpoint)| (id, &checkpoint.timestamp)).collect();

        checkpoint_list.sort_by_key(|k| k.1);
        checkpoint_list.into_iter().map(|(id, _)| id.clone()).collect()
    }

    /// Maintains the checkpoint storage by removing older checkpoints if over limit
    fn maintain_checkpoint_storage(&self) -> RollbackResult<()> {
        let mut checkpoints = self
            .checkpoints
            .lock()
            .map_err(|_| RollbackError::StateStorageError("Failed to acquire checkpoints lock".to_string()))?;

        if checkpoints.len() <= self.max_checkpoints {
            return Ok(());
        }

        // Get checkpoint IDs sorted by timestamp
        let mut checkpoint_items: Vec<(String, u64)> = checkpoints.iter().map(|(id, checkpoint)| (id.clone(), checkpoint.timestamp)).collect();

        checkpoint_items.sort_by_key(|k| k.1);

        // Remove oldest checkpoints until we're under the limit
        let to_remove = checkpoint_items.len() - self.max_checkpoints;
        for i in 0..to_remove {
            let id_to_remove = &checkpoint_items[i].0;
            checkpoints.remove(id_to_remove);
            log_event(LogLevel::Info, "CheckpointManager", &format!("Removed old checkpoint: {}", id_to_remove));
        }

        Ok(())
    }
}

impl Default for DefaultCheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckpointManager for DefaultCheckpointManager {
    fn create_checkpoint(&mut self, state: SystemState) -> RollbackResult<Checkpoint> {
        // Grab a single nanosecond‐precision instant
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let nanos = now.as_secs() * 1_000_000_000 + now.subsec_nanos() as u64;
        // Use that same value for the checkpoint ID _and_ the timestamp
        let checkpoint_id = format!("checkpoint-{}", nanos);
        let timestamp = nanos;

        let checkpoint = Checkpoint {
            id: checkpoint_id.clone(),
            timestamp,
            state,
        };

        let mut checkpoints = self
            .checkpoints
            .lock()
            .map_err(|_| RollbackError::StateStorageError("Failed to acquire checkpoints lock".to_string()))?;

        checkpoints.insert(checkpoint_id.clone(), checkpoint.clone());
        log_event(LogLevel::Info, "CheckpointManager", &format!("Created checkpoint: {}", checkpoint_id));

        drop(checkpoints); // Release lock before maintenance
        self.maintain_checkpoint_storage()?;

        Ok(checkpoint)
    }

    fn get_checkpoint(&self, id: &str) -> RollbackResult<Checkpoint> {
        let checkpoints = self
            .checkpoints
            .lock()
            .map_err(|_| RollbackError::StateStorageError("Failed to acquire checkpoints lock".to_string()))?;

        checkpoints.get(id).cloned().ok_or_else(|| RollbackError::CheckpointNotFound(id.to_string()))
    }

    fn get_latest_checkpoint_id(&self) -> Option<String> {
        let checkpoints = match self.checkpoints.lock() {
            Ok(checkpoints) => checkpoints,
            Err(_) => return None,
        };

        if checkpoints.is_empty() {
            return None;
        }

        // Find the checkpoint with the latest timestamp
        checkpoints.iter().max_by_key(|(_, checkpoint)| checkpoint.timestamp).map(|(id, _)| id.clone())
    }

    fn apply_checkpoint(&self, checkpoint: &Checkpoint) -> RollbackResult<()> {
        log_event(LogLevel::Info, "CheckpointManager", &format!("Applying checkpoint: {}", checkpoint.id));

        // If a callback was provided, use it to apply the checkpoint
        if let Some(ref cell) = self.checkpoint_apply_callback {
            // borrow the FnMut and invoke it
            (cell.borrow_mut())(checkpoint)?;
        } else {
            // Default implementation would restore state here
            log_event(LogLevel::Warning, "CheckpointManager", "No apply callback set, checkpoint state not fully restored");
        }

        Ok(())
    }

    fn delete_checkpoint(&mut self, id: &str) -> RollbackResult<()> {
        let mut checkpoints = self
            .checkpoints
            .lock()
            .map_err(|_| RollbackError::StateStorageError("Failed to acquire checkpoints lock".to_string()))?;

        if checkpoints.remove(id).is_some() {
            log_event(LogLevel::Info, "CheckpointManager", &format!("Deleted checkpoint: {}", id));
            Ok(())
        } else {
            Err(RollbackError::CheckpointNotFound(id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_checkpoint() {
        // Setup
        let mut checkpoint_manager = DefaultCheckpointManager::new();

        // Create a test state
        let mut state = SystemState::new();
        state.insert("test_key".to_string(), vec![1, 2, 3]);

        // Create checkpoint
        let result = checkpoint_manager.create_checkpoint(state.clone());
        assert!(result.is_ok());

        let checkpoint = result.unwrap();
        assert!(!checkpoint.id.is_empty());
        assert_eq!(checkpoint.state.get("test_key"), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_get_checkpoint() {
        // Setup
        let mut checkpoint_manager = DefaultCheckpointManager::new();

        // Create a test state and checkpoint
        let mut state = SystemState::new();
        state.insert("test_key".to_string(), vec![1, 2, 3]);

        let checkpoint = checkpoint_manager.create_checkpoint(state).unwrap();
        let checkpoint_id = checkpoint.id.clone();

        // Get checkpoint
        let result = checkpoint_manager.get_checkpoint(&checkpoint_id);
        assert!(result.is_ok());

        let retrieved_checkpoint = result.unwrap();
        assert_eq!(retrieved_checkpoint.id, checkpoint_id);
        assert_eq!(retrieved_checkpoint.state.get("test_key"), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_get_latest_checkpoint_id() {
        // Setup
        let mut checkpoint_manager = DefaultCheckpointManager::new();

        // Initially no checkpoints
        assert!(checkpoint_manager.get_latest_checkpoint_id().is_none());

        // Create multiple checkpoints
        let state1 = SystemState::new();
        let state2 = SystemState::new();

        let checkpoint1 = checkpoint_manager.create_checkpoint(state1).unwrap();
        let checkpoint2 = checkpoint_manager.create_checkpoint(state2).unwrap();

        // Verify latest checkpoint
        let latest_id = checkpoint_manager.get_latest_checkpoint_id();
        assert!(latest_id.is_some());
        assert_eq!(latest_id.unwrap(), checkpoint2.id);
    }

    #[test]
    fn test_apply_checkpoint() {
        // Setup
        let mut checkpoint_manager = DefaultCheckpointManager::new();

        // Track if callback was executed (thread-safe)
        use std::sync::{Arc, Mutex};
        let callback_executed = Arc::new(Mutex::new(false));
        let callback_executed_clone = Arc::clone(&callback_executed);

        checkpoint_manager.set_apply_callback(move |checkpoint| {
            assert_eq!(checkpoint.state.get("test_key"), Some(&vec![1, 2, 3]));
            let mut flag = callback_executed_clone.lock().unwrap();
            *flag = true;
            Ok(())
        });

        // Create a test state and checkpoint
        let mut state = SystemState::new();
        state.insert("test_key".to_string(), vec![1, 2, 3]);

        let checkpoint = checkpoint_manager.create_checkpoint(state).unwrap();

        // Apply checkpoint
        let result = checkpoint_manager.apply_checkpoint(&checkpoint);
        assert!(result.is_ok());

        // Verify the callback ran
        assert!(*callback_executed.lock().unwrap());
    }

    #[test]
    fn test_delete_checkpoint() {
        // Setup
        let mut checkpoint_manager = DefaultCheckpointManager::new();

        // Create a test state and checkpoint
        let state = SystemState::new();
        let checkpoint = checkpoint_manager.create_checkpoint(state).unwrap();
        let checkpoint_id = checkpoint.id.clone();

        // Delete checkpoint
        let result = checkpoint_manager.delete_checkpoint(&checkpoint_id);
        assert!(result.is_ok());

        // Verify checkpoint was deleted
        let get_result = checkpoint_manager.get_checkpoint(&checkpoint_id);
        assert!(get_result.is_err());
    }

    #[test]
    fn test_checkpoint_storage_maintenance() {
        // Setup
        let mut checkpoint_manager = DefaultCheckpointManager::new();
        checkpoint_manager.set_max_checkpoints(2);

        // Create more checkpoints than the limit
        let state = SystemState::new();
        let checkpoint1 = checkpoint_manager.create_checkpoint(state.clone()).unwrap();
        let checkpoint2 = checkpoint_manager.create_checkpoint(state.clone()).unwrap();
        let checkpoint3 = checkpoint_manager.create_checkpoint(state.clone()).unwrap();

        // Verify older checkpoints were removed
        let get_result1 = checkpoint_manager.get_checkpoint(&checkpoint1.id);
        let get_result2 = checkpoint_manager.get_checkpoint(&checkpoint2.id);
        let get_result3 = checkpoint_manager.get_checkpoint(&checkpoint3.id);

        assert!(get_result1.is_err()); // Oldest should be removed
        assert!(get_result2.is_ok()); // Should be kept
        assert!(get_result3.is_ok()); // Most recent should be kept
    }
}
