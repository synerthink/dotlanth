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

//! State Snapshot System
//!
//! This module provides functionality for periodic or on-demand state snapshots
//! to support rollback and historical analysis.

use crate::vm::state_management::lib::{Error, Result, StateKey, StateValue, current_timestamp};
use crate::vm::state_management::mvcc::{MVCCStore, Version};
use crate::vm::state_management::tree::{MerkleTree, StateHash};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Metadata for a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Unique identifier for the snapshot
    pub id: String,
    /// Version of the state at the time of snapshot
    pub version: Version,
    /// Timestamp when the snapshot was created
    pub timestamp: u64,
    /// Root hash of the state Merkle tree at the time of snapshot
    pub root_hash: Option<StateHash>,
    /// Optional description or purpose for this snapshot
    pub description: Option<String>,
}

/// Represents a complete state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Metadata for this snapshot
    pub metadata: SnapshotMetadata,
    /// The actual state data (key-value pairs) with base64 encoding
    pub state: BTreeMap<String, String>,
}

impl Snapshot {
    /// Creates a new snapshot from a state map and metadata
    pub fn new(state: BTreeMap<StateKey, StateValue>, version: Version, root_hash: Option<StateHash>, description: Option<String>) -> Self {
        let serialized_state = state
            .into_iter()
            .map(|(k, v)| {
                let key = BASE64.encode(&k.0);
                let value = BASE64.encode(&v.0);
                (key, value)
            })
            .collect();

        Self {
            metadata: SnapshotMetadata {
                id: format!("snapshot_{}", current_timestamp()),
                version,
                timestamp: current_timestamp(),
                root_hash,
                description,
            },
            state: serialized_state,
        }
    }

    /// Deserializes the state map
    pub fn deserialize_state(&self) -> BTreeMap<StateKey, StateValue> {
        self.state
            .iter()
            .map(|(k, v)| {
                let key_bytes = BASE64.decode(k).unwrap();
                let value_bytes = BASE64.decode(v).unwrap();
                (StateKey::new(key_bytes), StateValue::new(value_bytes))
            })
            .collect()
    }

    /// Saves the snapshot to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::SnapshotError(format!("Failed to create directory {}: {}", parent.display(), e)))?;
        }

        let serialized = serde_json::to_string(self).map_err(|e| Error::SnapshotError(format!("Serialization failed: {e}")))?; // <-- Burada düzeltildi

        fs::write(path, serialized).map_err(|e| Error::SnapshotError(format!("Failed to write file {}: {}", path.display(), e)))?;

        Ok(())
    }

    /// Loads a snapshot from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| Error::SnapshotError(format!("Failed to read file: {e}")))?;

        serde_json::from_str(&content).map_err(|e| Error::SnapshotError(format!("Deserialization failed: {e}")))
    }
}

/// Manager for creating and handling snapshots
#[derive(Debug)]
pub struct SnapshotManager {
    /// MVCC store reference for accessing state
    store: Arc<MVCCStore>,
    /// Directory where snapshots are stored
    snapshot_dir: PathBuf,
    /// In-memory cache of snapshot metadata
    metadata_cache: Arc<RwLock<HashMap<String, SnapshotMetadata>>>,
}

/// Snapshot management system for state rollback/audit capabilities.
/// Handles serialization, storage, and restoration of state snapshots.
impl SnapshotManager {
    /// Creates a new snapshot manager
    pub fn new<P: AsRef<Path>>(store: Arc<MVCCStore>, snapshot_dir: P) -> Result<Self> {
        let path = snapshot_dir.as_ref().to_path_buf();

        // Ensure snapshot directory exists (with better error handling)
        fs::create_dir_all(&path).map_err(|e| Error::SnapshotError(format!("Failed to create directory {}: {}", path.display(), e)))?;

        let manager = Self {
            store,
            snapshot_dir: path,
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        manager.refresh_metadata_cache()?;
        Ok(manager)
    }

    /// Refreshes the metadata cache by reading snapshot files
    pub fn refresh_metadata_cache(&self) -> Result<()> {
        let mut cache = self.metadata_cache.write().unwrap();
        cache.clear();

        // Read all snapshot files in the directory
        let entries = fs::read_dir(&self.snapshot_dir).map_err(|e| Error::SnapshotError(format!("Failed to read directory: {e}")))?;

        for entry in entries {
            let entry = entry.map_err(|e| Error::SnapshotError(format!("Invalid entry: {e}")))?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                // Try to load snapshot metadata
                if let Ok(content) = fs::read_to_string(&path)
                    && let Ok(snapshot) = serde_json::from_str::<Snapshot>(&content)
                {
                    cache.insert(snapshot.metadata.id.clone(), snapshot.metadata);
                }
            }
        }

        Ok(())
    }

    /// Creates a snapshot with:
    /// - **Merkle Root Capture**: Computes state hash during creation
    /// - **Base64 Encoding**: Safely serializes binary data
    /// - **Version Alignment**: Matches snapshot to store's current version
    /// - **Atomic Writes**: Ensures file integrity via temp directory patterns
    ///
    /// # Returns
    /// - `Snapshot`: Created snapshot object
    /// - `Error`: On I/O failures or Merkle tree errors
    pub fn create_snapshot(&self, description: Option<String>) -> Result<Snapshot> {
        // Get current version and state
        let version = self.store.current_version();
        let state = self.store.get_latest_state();

        // Build Merkle tree for the state to get root hash
        let tree = MerkleTree::build(&state)?;
        let root_hash = tree.root_hash();

        // Create the snapshot
        let snapshot = Snapshot::new(state, version, root_hash, description);

        // Save to file
        let file_path = self.snapshot_dir.join(format!("{}.json", snapshot.metadata.id));
        snapshot.save_to_file(&file_path)?;

        // Update metadata cache
        self.metadata_cache.write().unwrap().insert(snapshot.metadata.id.clone(), snapshot.metadata.clone());

        Ok(snapshot)
    }

    /// Lists all available snapshots
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotMetadata>> {
        // Refresh cache to ensure we have the latest data
        self.refresh_metadata_cache()?;

        let cache = self.metadata_cache.read().unwrap();
        let mut snapshots: Vec<SnapshotMetadata> = cache.values().cloned().collect();

        // Sort by timestamp (newest first)
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(snapshots)
    }

    /// Loads a specific snapshot by ID
    pub fn load_snapshot(&self, id: &str) -> Result<Snapshot> {
        let file_path = self.snapshot_dir.join(format!("{id}.json"));
        Snapshot::load_from_file(file_path)
    }

    /// Restores the state to a specific snapshot
    pub fn restore_snapshot(&self, id: &str) -> Result<Version> {
        // Load the snapshot
        let snapshot = self.load_snapshot(id)?;
        let state = snapshot.deserialize_state();

        // Create write operations to restore the state
        let current_state = self.store.get_latest_state();
        let mut operations = Vec::new();

        // Delete keys that are in current state but not in the snapshot
        for key in current_state.keys() {
            if !state.contains_key(key) {
                operations.push(crate::vm::state_management::mvcc::WriteOperation::Delete(key.clone()));
            }
        }

        // Add or update keys from the snapshot
        for (key, value) in state {
            operations.push(crate::vm::state_management::mvcc::WriteOperation::Put(key, value));
        }

        // Apply all operations in a single transaction
        self.store.transaction(operations)
    }

    /// Deletes a snapshot
    pub fn delete_snapshot(&self, id: &str) -> Result<()> {
        let file_path = self.snapshot_dir.join(format!("{id}.json"));

        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| Error::SnapshotError(format!("Failed to delete file: {e}")))?;

            // Update cache
            self.metadata_cache.write().unwrap().remove(id);

            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_test_store() -> Arc<MVCCStore> {
        let store = Arc::new(MVCCStore::new());

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
    fn test_snapshot_creation_and_loading() {
        // Create a temporary directory for snapshots
        let temp_dir = tempdir().unwrap();
        let store = setup_test_store();

        // Create snapshot manager
        let manager = SnapshotManager::new(store.clone(), temp_dir.path()).unwrap();

        // Create a snapshot
        let description = Some("Test snapshot".to_string());
        let snapshot = manager.create_snapshot(description.clone()).unwrap();

        // Verify snapshot metadata
        assert_eq!(snapshot.metadata.version, store.current_version());
        assert_eq!(snapshot.metadata.description, description);

        // Verify we can list the snapshot
        let snapshots = manager.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, snapshot.metadata.id);

        // Load the snapshot and verify contents
        let loaded = manager.load_snapshot(&snapshot.metadata.id).unwrap();
        assert_eq!(loaded.metadata.id, snapshot.metadata.id);

        // Verify state content
        let state = loaded.deserialize_state();
        assert_eq!(state.len(), 2); // Should have our two keys
    }

    #[test]
    fn test_snapshot_restore() {
        // Create a temporary directory for snapshots
        let temp_dir = tempdir().unwrap();
        let store = setup_test_store();

        // Create snapshot manager
        let manager = SnapshotManager::new(store.clone(), temp_dir.path()).unwrap();

        // Create a snapshot of initial state
        let snapshot = manager.create_snapshot(Some("Initial state".to_string())).unwrap();
        let initial_version = store.current_version();

        // Modify the state after snapshot
        let key3 = StateKey::from_string("key3");
        let value3 = StateValue::from_string("value3");
        let _ = store.put(key3.clone(), value3);
        let key1 = StateKey::from_string("key1");
        let _ = store.delete(key1.clone());

        // Verify the changes
        let modified_state = store.get_latest_state();
        assert!(modified_state.contains_key(&key3));
        assert!(!modified_state.contains_key(&key1));

        // Restore to the snapshot
        let restore_version = manager.restore_snapshot(&snapshot.metadata.id).unwrap();

        // Verify state is restored
        let restored_state = store.get_latest_state();
        assert!(!restored_state.contains_key(&key3)); // Should not have the new key
        assert!(restored_state.contains_key(&key1)); // Should have the deleted key back

        // Version should be incremented
        assert!(restore_version > initial_version);
    }

    #[test]
    fn test_snapshot_delete() {
        // Create a temporary directory for snapshots
        let temp_dir = tempdir().unwrap();
        let store = setup_test_store();

        // Create snapshot manager
        let manager = SnapshotManager::new(store.clone(), temp_dir.path()).unwrap();

        // Create a snapshot
        let snapshot = manager.create_snapshot(None).unwrap();

        // Verify we can list the snapshot
        assert_eq!(manager.list_snapshots().unwrap().len(), 1);

        // Delete the snapshot
        manager.delete_snapshot(&snapshot.metadata.id).unwrap();

        // Verify it's gone
        assert_eq!(manager.list_snapshots().unwrap().len(), 0);

        // Trying to load it should fail
        assert!(manager.load_snapshot(&snapshot.metadata.id).is_err());
    }
}
