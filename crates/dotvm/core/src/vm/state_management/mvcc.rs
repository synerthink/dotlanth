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

//! Multi-Version Concurrency Control implementation for state management
//!
//! This module provides the foundation for handling concurrent state
//! operations by maintaining multiple versions of state entries.

use crate::vm::state_management::lib::{current_timestamp, new_shared_map, Error, Result, SharedMap, StateKey, StateValue};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Version identifier used for MVCC
pub type Version = u64;

/// Represents a value with its version information
#[derive(Debug, Clone)]
pub struct VersionedValue {
    /// The actual value data
    pub value: StateValue,
    /// The version when this value was created
    pub created_at: Version,
    /// The version when this value was marked as deleted (None if active)
    pub deleted_at: Option<Version>,
}

impl VersionedValue {
    /// Creates a new versioned value
    pub fn new(value: StateValue, version: Version) -> Self {
        Self {
            value,
            created_at: version,
            deleted_at: None,
        }
    }

    /// Checks if the value is visible at the given version
    pub fn is_visible_at(&self, version: Version) -> bool {
        self.created_at <= version && (self.deleted_at.is_none() || self.deleted_at.unwrap() > version)
    }

    /// Marks the value as deleted at the given version
    pub fn mark_deleted(&mut self, version: Version) {
        self.deleted_at = Some(version);
    }
}

/// Types of write operations supported by the MVCC store
#[derive(Debug, Clone)]
pub enum WriteOperation {
    /// Insert or update a value
    Put(StateKey, StateValue),
    /// Delete a value
    Delete(StateKey),
}

/// MVCC store implementation for maintaining versioned state data
pub struct MVCCStore {
    /// The main storage for all versions of all keys
    versions: SharedMap<StateKey, Vec<VersionedValue>>,
    /// Lock for coordinating version allocation
    version_lock: Arc<RwLock<Version>>,
}

/// MVCC implementation for concurrent state management.
/// Maintains version history and transactional consistency.
impl MVCCStore {
    /// Creates a new MVCC store
    pub fn new() -> Self {
        Self {
            versions: new_shared_map(),
            version_lock: Arc::new(RwLock::new(0)),
        }
    }

    /// Gets the current latest version
    pub fn current_version(&self) -> Version {
        *self.version_lock.read().unwrap()
    }

    /// Reads a value at the specified version
    pub fn read(&self, key: &StateKey, version: Version) -> Result<Option<StateValue>> {
        let versions_map = self.versions.read().unwrap();

        if let Some(versions) = versions_map.get(key) {
            // Find the latest version that's visible at the requested version
            for v in versions.iter().rev() {
                if v.is_visible_at(version) {
                    return Ok(Some(v.value.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Reads the latest value for a key
    pub fn read_latest(&self, key: &StateKey) -> Result<Option<StateValue>> {
        self.read(key, self.current_version())
    }

    /// Executes atomic transactions with:
    /// - **Version Locking**: Serializes write operations
    /// - **Tombstone Marking**: Soft deletes with version tracking
    /// - **Visibility Rules**: Filters values by version ranges
    /// - **Batch Processing**: Applies multiple operations atomically
    ///
    /// # Arguments
    /// - `operations`: Vector of Put/Delete operations
    ///
    /// # Returns
    /// - `Version`: New version number after commit
    /// - `Error`: On version conflicts or lock poisoning
    pub fn transaction(&self, operations: Vec<WriteOperation>) -> Result<Version> {
        // Acquire write lock to ensure atomic transaction
        let mut version_guard = self.version_lock.write().unwrap();
        let next_version = *version_guard + 1;

        // Apply all operations
        let mut versions_map = self.versions.write().unwrap();

        for op in operations {
            match op {
                WriteOperation::Put(key, value) => {
                    let entry = versions_map.entry(key).or_insert_with(Vec::new);
                    entry.push(VersionedValue::new(value, next_version));
                }
                WriteOperation::Delete(key) => {
                    if let Some(versions) = versions_map.get_mut(&key) {
                        // Mark the latest visible version as deleted
                        for v in versions.iter_mut().rev() {
                            if v.deleted_at.is_none() {
                                v.mark_deleted(next_version);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Update the current version
        *version_guard = next_version;

        Ok(next_version)
    }

    /// Simple put operation that wraps the operation in a transaction
    pub fn put(&self, key: StateKey, value: StateValue) -> Result<Version> {
        self.transaction(vec![WriteOperation::Put(key, value)])
    }

    /// Simple delete operation that wraps the operation in a transaction
    pub fn delete(&self, key: StateKey) -> Result<Version> {
        self.transaction(vec![WriteOperation::Delete(key)])
    }

    /// Gets a map of all keys and their values at a specific version
    pub fn get_state_at_version(&self, version: Version) -> BTreeMap<StateKey, StateValue> {
        let mut result = BTreeMap::new();
        let versions_map = self.versions.read().unwrap();

        for (key, versions) in versions_map.iter() {
            for v in versions.iter().rev() {
                if v.is_visible_at(version) {
                    result.insert(key.clone(), v.value.clone());
                    break;
                }
            }
        }

        result
    }

    /// Gets the latest state map
    pub fn get_latest_state(&self) -> BTreeMap<StateKey, StateValue> {
        self.get_state_at_version(self.current_version())
    }
}

impl Default for MVCCStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_store() -> MVCCStore {
        let store = MVCCStore::new();

        // Add some initial data
        let key1 = StateKey::from_string("key1");
        let key2 = StateKey::from_string("key2");
        let value1 = StateValue::from_string("value1");
        let value2 = StateValue::from_string("value2");

        let _ = store.put(key1, value1);
        let _ = store.put(key2, value2);

        store
    }

    #[test]
    fn test_read_write() {
        let store = MVCCStore::new();

        let key = StateKey::from_string("test_key");
        let value1 = StateValue::from_string("value1");

        // Initial write
        let version1 = store.put(key.clone(), value1.clone()).unwrap();

        // Read at that version
        let read_result = store.read(&key, version1).unwrap();
        assert_eq!(read_result, Some(value1.clone()));

        // Update the value
        let value2 = StateValue::from_string("value2");
        let version2 = store.put(key.clone(), value2.clone()).unwrap();

        // Read at different versions
        assert_eq!(store.read(&key, version1).unwrap(), Some(value1.clone()));
        assert_eq!(store.read(&key, version2).unwrap(), Some(value2.clone()));

        // Latest should be value2
        assert_eq!(store.read_latest(&key).unwrap(), Some(value2.clone()));
    }

    #[test]
    fn test_transaction() {
        let store = MVCCStore::new();

        let key1 = StateKey::from_string("key1");
        let key2 = StateKey::from_string("key2");
        let value1 = StateValue::from_string("value1");
        let value2 = StateValue::from_string("value2");

        // Execute a transaction with multiple operations
        let ops = vec![WriteOperation::Put(key1.clone(), value1.clone()), WriteOperation::Put(key2.clone(), value2.clone())];

        let version = store.transaction(ops).unwrap();

        // Both keys should be updated in the same version
        assert_eq!(store.read(&key1, version).unwrap(), Some(value1));
        assert_eq!(store.read(&key2, version).unwrap(), Some(value2));
    }

    #[test]
    fn test_delete() {
        let store = setup_test_store();
        let key1 = StateKey::from_string("key1");

        // First version should have the value
        let initial_version = store.current_version();
        assert!(store.read(&key1, initial_version).unwrap().is_some());

        // Delete the key
        let delete_version = store.delete(key1.clone()).unwrap();

        // Should still be visible at the initial version
        assert!(store.read(&key1, initial_version).unwrap().is_some());

        // Should be deleted at the delete version
        assert!(store.read(&key1, delete_version).unwrap().is_none());
    }

    #[test]
    fn test_get_state_at_version() {
        let store = setup_test_store();

        let key1 = StateKey::from_string("key1");
        let key3 = StateKey::from_string("key3");
        let value3 = StateValue::from_string("value3");

        // Remember initial version
        let version1 = store.current_version();

        // Add another key
        let version2 = store.put(key3, value3).unwrap();

        // Check state at version1
        let state1 = store.get_state_at_version(version1);
        assert_eq!(state1.len(), 2); // Only key1 and key2

        // Check state at version2
        let state2 = store.get_state_at_version(version2);
        assert_eq!(state2.len(), 3); // key1, key2, and key3

        // Delete key1 and check again
        let version3 = store.delete(key1).unwrap();
        let state3 = store.get_state_at_version(version3);
        assert_eq!(state3.len(), 2); // Only key2 and key3
    }
}
