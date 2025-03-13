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

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;

/// Enum representing the possible errors during state operations.
#[derive(Debug, PartialEq)]
pub enum StateStorageError {
    /// The requested state key was not found.
    NotFound,
    /// Key already exists in storage.
    AlreadyExists,
    /// An error occurred while connecting to or accessing the underlying storage.
    ConnectionError,
    /// An unspecified error occurred.
    Unknown,
    /// Operation not implemented.
    NotImplemented,
}

impl fmt::Display for StateStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for StateStorageError {}

/// The StateStorage trait defines the interface for state management.
/// This trait exposes three primary operations: load, save, and update.
pub trait StateStorage {
    /// Loads the state associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - A string slice identifying the storage key.
    ///
    /// # Returns
    ///
    /// * Ok(String) with the state data if found.
    /// * Err(StateStorageError) if an error occurs.
    fn load(&self, key: &str) -> Result<String, StateStorageError>;

    /// Saves the provided state value with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The storage key.
    /// * `value` - The state to be saved.
    ///
    /// # Returns
    ///
    /// * Ok(()) if successful.
    /// * Err(StateStorageError) otherwise.
    fn save(&mut self, key: &str, value: &str) -> Result<(), StateStorageError>;

    /// Updates the state for an existing key with a new value.
    ///
    /// # Arguments
    ///
    /// * `key` - The storage key.
    /// * `value` - The new state value.
    ///
    /// # Returns
    ///
    /// * Ok(()) if the update is successful.
    /// * Err(StateStorageError) if an error occurs.
    fn update(&mut self, key: &str, value: &str) -> Result<(), StateStorageError>;
}

/// DefaultStateStorage is our primary production implementation of the StateStorage interface.
/// Its methods are left unimplemented (using unimplemented!()) to be filled per the TDD process.
/// Thread-safe production implementation using HashMap
#[derive(Debug, Default)]
pub struct DefaultStateStorage {
    storage: Arc<Mutex<HashMap<String, String>>>,
}

impl DefaultStateStorage {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl StateStorage for DefaultStateStorage {
    fn load(&self, key: &str) -> Result<String, StateStorageError> {
        let storage = self.storage.lock().unwrap();
        storage.get(key).cloned().ok_or(StateStorageError::NotFound)
    }

    fn save(&mut self, key: &str, value: &str) -> Result<(), StateStorageError> {
        let mut storage = self.storage.lock().unwrap();
        if storage.contains_key(key) {
            Err(StateStorageError::AlreadyExists)
        } else {
            storage.insert(key.to_string(), value.to_string());
            Ok(())
        }
    }

    fn update(&mut self, key: &str, value: &str) -> Result<(), StateStorageError> {
        let mut storage = self.storage.lock().unwrap();
        if storage.contains_key(key) {
            storage.insert(key.to_string(), value.to_string());
            Ok(())
        } else {
            Err(StateStorageError::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The following tests define the desired behavior of our state storage.
    /// Since DefaultStateStorage is not yet implemented, these tests are expected to
    /// fail. Once the production implementation is complete, they should pass.

    #[test]
    fn test_save_and_load() {
        let mut storage = DefaultStateStorage::new();
        let key = "state_key";
        let value = "initial_state";

        // Expect save to succeed.
        // (Initial implementation of DefaultStateStorage should eventually save the state.)
        assert!(storage.save(key, value).is_ok(), "Saving key-value should succeed");

        // Expect load to retrieve the correct value.
        let loaded_value = storage.load(key).expect("Loading saved key should succeed");
        assert_eq!(loaded_value, value, "Loaded value must match saved value");
    }

    #[test]
    fn test_save_existing_key_fails() {
        let mut storage = DefaultStateStorage::new();
        let key = "state_key";
        let value = "state_one";

        // Initial save should succeed.
        assert!(storage.save(key, value).is_ok(), "Initial save should succeed");
        // Second save with the same key should fail.
        let result = storage.save(key, "state_two");
        assert_eq!(result, Err(StateStorageError::AlreadyExists), "Saving an existing key should fail with Unknown error");
    }

    #[test]
    fn test_update_existing_key() {
        let mut storage = DefaultStateStorage::new();
        let key = "state_key";
        let value = "state_one";

        // Save the key.
        assert!(storage.save(key, value).is_ok(), "Saving initial state should work");

        // Update the key.
        let new_value = "state_updated";
        assert!(storage.update(key, new_value).is_ok(), "Updating existing key should succeed");

        // Verify that update took effect.
        let loaded_value = storage.load(key).expect("Loading updated key should work");
        assert_eq!(loaded_value, new_value, "Loaded value should reflect updated value");
    }

    #[test]
    fn test_update_non_existing_key_fails() {
        let mut storage = DefaultStateStorage::new();
        let key = "non_existing";
        let result = storage.update(key, "value");
        assert_eq!(result, Err(StateStorageError::NotFound), "Updating non-existing key should fail with NotFound error");
    }

    #[test]
    fn test_load_non_existent_key() {
        let storage = DefaultStateStorage::new();
        // Attempt to load a key that has not been saved.
        assert_eq!(storage.load("non_existent"), Err(StateStorageError::NotFound), "Loading non-existent key should return NotFound");
    }

    #[test]
    fn test_save_empty_key_and_value() {
        let mut storage = DefaultStateStorage::new();
        // Saving with an empty key and empty value.
        assert!(storage.save("", "").is_ok(), "Saving empty key and value should succeed");
        // Loading the empty key should return an empty string.
        let loaded = storage.load("").expect("Loading empty key should succeed");
        assert_eq!(loaded, "", "Loaded value for empty key should be empty");
    }

    #[test]
    fn test_save_key_with_spaces() {
        let mut storage = DefaultStateStorage::new();
        let key = "  key with spaces  ";
        let value = "value";
        assert!(storage.save(key, value).is_ok(), "Saving key with spaces should succeed");
        let loaded = storage.load(key).expect("Loading key with spaces should succeed");
        assert_eq!(loaded, value, "Loaded value for key with spaces must match saved value");
    }

    #[test]
    fn test_save_empty_value() {
        let mut storage = DefaultStateStorage::new();
        let key = "empty_value_key";
        let value = "";
        assert!(storage.save(key, value).is_ok(), "Saving key with empty value should succeed");
        let loaded = storage.load(key).expect("Loading key with empty value should succeed");
        assert_eq!(loaded, value, "Loaded value for key with empty value must be empty");
    }

    #[test]
    fn test_update_to_same_value() {
        let mut storage = DefaultStateStorage::new();
        let key = "test_key";
        let value = "value";
        assert!(storage.save(key, value).is_ok(), "Initial save should succeed");
        // Updating with the same value should also succeed.
        assert!(storage.update(key, value).is_ok(), "Updating with the same value should succeed");
        let loaded = storage.load(key).expect("Loading key after update should succeed");
        assert_eq!(loaded, value, "Loaded value should be unchanged after updating with the same value");
    }

    #[test]
    fn test_consecutive_updates() {
        let mut storage = DefaultStateStorage::new();
        let key = "consecutive_key";
        let values = vec!["first", "second", "third"];
        assert!(storage.save(key, values[0]).is_ok(), "Saving initial value should succeed");
        for &v in values.iter().skip(1) {
            assert!(storage.update(key, v).is_ok(), "Updating key should succeed");
            let loaded = storage.load(key).expect("Loading key after update should succeed");
            assert_eq!(loaded, v, "Loaded value must match value updated consecutively");
        }
    }
}
