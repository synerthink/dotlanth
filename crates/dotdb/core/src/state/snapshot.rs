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
//! This module provides a comprehensive state snapshot system for capturing and
//! managing point-in-time views of the blockchain state. It enables efficient
//! state reconstruction, rollback capabilities, and state synchronization.
//!
//! # Features
//!
//! - Point-in-time state capture
//! - Efficient snapshot management
//! - Metadata and versioning support
//! - Automatic cleanup of old snapshots
//! - State restoration capabilities
//!
//! # Performance Considerations
//!
//! - Efficient snapshot storage and retrieval
//! - Minimal memory overhead
//! - Optimized cleanup operations
//! - Thread-safe operations
//!
//! # Usage
//!
//! ```rust
//! use dotdb_core::state::snapshot::{SnapshotManager, SnapshotConfig, StateSnapshot};
//! use dotdb_core::state::mpt::MerklePatriciaTrie;
//!
//! // Create a snapshot manager
//! let config = SnapshotConfig::default();
//! let mut manager = SnapshotManager::new(config);
//!
//! // Create a trie (empty for simplicity in docs)
//! let trie = MerklePatriciaTrie::new_in_memory();
//!
//! // Create a snapshot  
//! let snapshot = manager.create_snapshot(
//!     "snapshot-1".to_string(),
//!     &trie,
//!     Some(1000),
//!     Some("Block 1000".to_string())
//! ).unwrap();
//!
//! // Restore state from snapshot (using a new base trie)
//! let base_trie = MerklePatriciaTrie::new_in_memory();
//! let snapshot_id = "snapshot-1".to_string();
//! let restored_trie = manager.restore_from_snapshot(&snapshot_id, base_trie).unwrap();
//! ```
//!
//! # Error Handling
//!
//! All operations return `SnapshotResult<T>` which is a type alias for
//! `Result<T, SnapshotError>`. The `SnapshotError` enum provides detailed
//! error information for different failure scenarios.

use crate::state::dot_storage_layout::DotAddress;
use crate::state::mpt::trie::NodeStorage;
use crate::state::mpt::{Hash, MPTError, MerklePatriciaTrie};
use crate::state::versioning::{DotVersionManager, DotVersioningError, StateVersionId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a snapshot of the state at a specific point in time
///
/// This struct captures all necessary information to reconstruct the state
/// at a particular moment, including metadata and timing information.
/// Now integrated with the versioning system for unified state management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Unique identifier for this snapshot
    pub id: SnapshotId,
    /// Versioning system state version ID
    pub version_id: StateVersionId,
    /// Optional contract address for contract-specific snapshots
    pub dot_address: Option<DotAddress>,
    /// Root hash of the state tree at snapshot time
    pub root_hash: Hash,
    /// Optional metadata for the snapshot
    pub metadata: HashMap<String, String>,
    /// Block height or sequence number (if applicable)
    pub height: Option<u64>,
    /// Optional description of the snapshot
    pub description: Option<String>,
}

/// Unique identifier for snapshots
pub type SnapshotId = String;

/// Error types for snapshot operations
///
/// This enum defines various error conditions that can occur during
/// snapshot operations, with detailed error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// Snapshot not found
    NotFound(SnapshotId),
    /// Snapshot already exists
    AlreadyExists(SnapshotId),
    /// Invalid snapshot data
    InvalidSnapshot(String),
    /// MPT operation failed
    MPTError(String),
    /// Versioning operation failed
    VersioningError(String),
    /// Contract not found
    DotNotFound(DotAddress),
    /// Serialization/deserialization error
    SerializationError(String),
    /// I/O error
    IoError(String),
}

impl From<MPTError> for SnapshotError {
    fn from(err: MPTError) -> Self {
        SnapshotError::MPTError(format!("{err:?}"))
    }
}

impl From<DotVersioningError> for SnapshotError {
    fn from(err: DotVersioningError) -> Self {
        SnapshotError::VersioningError(format!("{err:?}"))
    }
}

/// Type alias for snapshot operation results
pub type SnapshotResult<T> = Result<T, SnapshotError>;

impl Default for StateSnapshot {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            version_id: StateVersionId::default(),
            dot_address: None,
            root_hash: [0u8; 32],
            metadata: HashMap::new(),
            height: None,
            description: None,
        }
    }
}

impl StateSnapshot {
    /// Create a new snapshot with current timestamp
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the snapshot
    /// * `root_hash` - Root hash of the state tree
    /// * `height` - Optional block height
    /// * `description` - Optional description
    ///
    /// # Returns
    ///
    /// A new StateSnapshot instance
    pub fn new(id: SnapshotId, root_hash: Hash, height: Option<u64>, description: Option<String>) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        // Create a version ID with logical version 0 (for snapshot-only versions)
        let version_id = StateVersionId::new(0, timestamp);

        Self {
            id,
            version_id,
            dot_address: None,
            root_hash,
            metadata: HashMap::new(),
            height,
            description,
        }
    }

    /// Create a new snapshot with specific version ID and optional contract address
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the snapshot
    /// * `version_id` - State version ID from versioning system
    /// * `dot_address` - Optional contract address for contract-specific snapshots
    /// * `root_hash` - Root hash of the state tree
    /// * `height` - Optional block height
    /// * `description` - Optional description
    ///
    /// # Returns
    ///
    /// A new StateSnapshot instance
    pub fn new_with_version(id: SnapshotId, version_id: StateVersionId, dot_address: Option<DotAddress>, root_hash: Hash, height: Option<u64>, description: Option<String>) -> Self {
        Self {
            id,
            version_id,
            dot_address,
            root_hash,
            metadata: HashMap::new(),
            height,
            description,
        }
    }

    /// Create a snapshot from a trie
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the snapshot
    /// * `trie` - The trie to snapshot
    /// * `height` - Optional block height
    /// * `description` - Optional description
    ///
    /// # Returns
    ///
    /// A new StateSnapshot instance
    pub fn from_trie<S: NodeStorage>(id: SnapshotId, trie: &MerklePatriciaTrie<S>, height: Option<u64>, description: Option<String>) -> Self {
        Self::new(id, trie.root_hash(), height, description)
    }

    /// Add metadata to the snapshot
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    /// * `value` - Metadata value
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to look up
    ///
    /// # Returns
    ///
    /// Optional reference to the metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Get snapshot timestamp from version ID
    ///
    /// # Returns
    ///
    /// Timestamp when the snapshot was created
    pub fn timestamp(&self) -> u64 {
        self.version_id.timestamp()
    }

    /// Check if snapshot is older than specified seconds
    ///
    /// # Arguments
    ///
    /// * `seconds` - Age threshold in seconds
    ///
    /// # Returns
    ///
    /// True if snapshot is older than the threshold
    pub fn is_older_than(&self, seconds: u64) -> bool {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        current_time.saturating_sub(self.timestamp()) > seconds
    }

    /// Get age of snapshot in seconds
    ///
    /// # Returns
    ///
    /// Age of the snapshot in seconds
    pub fn age_seconds(&self) -> u64 {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        current_time.saturating_sub(self.timestamp())
    }

    /// Check if this is a contract-specific snapshot
    ///
    /// # Returns
    ///
    /// True if this snapshot is for a specific contract
    pub fn is_dot_snapshot(&self) -> bool {
        self.dot_address.is_some()
    }

    /// Get contract address if this is a contract-specific snapshot
    ///
    /// # Returns
    ///
    /// Optional reference to the contract address
    pub fn get_dot_address(&self) -> Option<&DotAddress> {
        self.dot_address.as_ref()
    }
}

/// Configuration for snapshot management
///
/// This struct defines the behavior and constraints for snapshot management,
/// including retention policies and cleanup settings.
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Maximum number of snapshots to keep
    pub max_snapshots: Option<usize>,
    /// Maximum age of snapshots in seconds
    pub max_age_seconds: Option<u64>,
    /// Whether to enable automatic cleanup
    pub auto_cleanup: bool,
    /// Interval for automatic cleanup in seconds
    pub cleanup_interval_seconds: u64,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            max_snapshots: Some(100),
            max_age_seconds: Some(86400 * 30), // 30 days
            auto_cleanup: true,
            cleanup_interval_seconds: 3600, // 1 hour
        }
    }
}

/// Snapshot management system with versioning integration
///
/// This manager provides comprehensive snapshot capabilities with integration
/// to the contract versioning system for unified state management.
pub struct SnapshotManager<S: NodeStorage> {
    /// Configuration for snapshot management
    config: SnapshotConfig,
    /// In-memory snapshot registry (in production, this would be persisted)
    snapshots: HashMap<SnapshotId, StateSnapshot>,
    /// Reference to the underlying MPT for state reconstruction
    current_trie: Option<MerklePatriciaTrie<S>>,
    /// Contract version manager for versioning integration
    version_manager: DotVersionManager,
}

impl<S: NodeStorage> SnapshotManager<S> {
    /// Create a new snapshot manager
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for snapshot management
    ///
    /// # Returns
    ///
    /// A new SnapshotManager instance
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            config,
            snapshots: HashMap::new(),
            current_trie: None,
            version_manager: DotVersionManager::new(100), // Default: keep 100 versions per contract
        }
    }

    /// Create a snapshot manager with default configuration
    ///
    /// # Returns
    ///
    /// A new SnapshotManager instance with default settings
    pub fn with_defaults() -> Self {
        Self::new(SnapshotConfig::default())
    }

    /// Set the current trie reference
    ///
    /// # Arguments
    ///
    /// * `trie` - The trie to use as current state
    pub fn set_current_trie(&mut self, trie: MerklePatriciaTrie<S>) {
        self.current_trie = Some(trie);
    }

    /// Create a snapshot
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the snapshot
    /// * `trie` - The trie to snapshot
    /// * `height` - Optional block height
    /// * `description` - Optional description
    ///
    /// # Returns
    ///
    /// A Result containing the created snapshot or an error
    pub fn create_snapshot(&mut self, id: SnapshotId, trie: &MerklePatriciaTrie<S>, height: Option<u64>, description: Option<String>) -> SnapshotResult<StateSnapshot> {
        // Check if snapshot already exists
        if self.snapshots.contains_key(&id) {
            return Err(SnapshotError::AlreadyExists(id));
        }

        // Create the snapshot
        let snapshot = StateSnapshot::from_trie(id.clone(), trie, height, description);

        // Store the snapshot
        self.snapshots.insert(id, snapshot.clone());

        // Auto-cleanup if enabled
        if self.config.auto_cleanup {
            let _ = self.cleanup_old_snapshots();
        }

        Ok(snapshot)
    }

    /// Create a contract-specific snapshot with versioning integration
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the snapshot
    /// * `dot_address` - Contract address for this snapshot
    /// * `trie` - The trie to snapshot
    /// * `height` - Optional block height
    /// * `description` - Optional description
    ///
    /// # Returns
    ///
    /// A Result containing the created snapshot or an error
    pub fn create_dot_snapshot(&mut self, id: SnapshotId, dot_address: DotAddress, trie: &MerklePatriciaTrie<S>, height: Option<u64>, description: Option<String>) -> SnapshotResult<StateSnapshot> {
        // Check if snapshot already exists
        if self.snapshots.contains_key(&id) {
            return Err(SnapshotError::AlreadyExists(id));
        }

        // Create a version in the version manager
        let version_id = self
            .version_manager
            .create_version(dot_address, trie.root_hash(), description.clone().unwrap_or_else(|| format!("Snapshot: {id}")))?;

        // Create the snapshot with versioning information
        let snapshot = StateSnapshot::new_with_version(id.clone(), version_id, Some(dot_address), trie.root_hash(), height, description);

        // Acquire snapshot reference in version manager
        self.version_manager.acquire_snapshot(dot_address, version_id)?;

        // Store the snapshot
        self.snapshots.insert(id, snapshot.clone());

        // Auto-cleanup if enabled
        if self.config.auto_cleanup {
            let _ = self.cleanup_old_snapshots();
        }

        Ok(snapshot)
    }

    /// Create a snapshot from an existing contract version
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the snapshot
    /// * `contract_address` - Contract address
    /// * `version_id` - Existing version ID from contract version manager
    /// * `trie` - The trie to snapshot
    ///
    /// # Returns
    ///
    /// A Result containing the created snapshot or an error
    pub fn create_snapshot_from_version(&mut self, id: SnapshotId, dot_address: DotAddress, version_id: StateVersionId, trie: &MerklePatriciaTrie<S>) -> SnapshotResult<StateSnapshot> {
        // Check if snapshot already exists
        if self.snapshots.contains_key(&id) {
            return Err(SnapshotError::AlreadyExists(id));
        }

        // Get the contract version to extract metadata
        let dot_version = self
            .version_manager
            .get_version(dot_address, version_id)
            .ok_or_else(|| SnapshotError::VersioningError(format!("Version not found: {version_id:?}")))?;

        // Create the snapshot with versioning information
        let snapshot = StateSnapshot::new_with_version(id.clone(), version_id, Some(dot_address), trie.root_hash(), dot_version.block_height, Some(dot_version.description));

        // Acquire snapshot reference in version manager
        self.version_manager.acquire_snapshot(dot_address, version_id)?;

        // Store the snapshot
        self.snapshots.insert(id, snapshot.clone());

        Ok(snapshot)
    }

    /// Get a snapshot by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The snapshot ID to look up
    ///
    /// # Returns
    ///
    /// A Result containing a reference to the snapshot or an error
    pub fn get_snapshot(&self, id: &SnapshotId) -> SnapshotResult<&StateSnapshot> {
        self.snapshots.get(id).ok_or_else(|| SnapshotError::NotFound(id.clone()))
    }

    /// List all snapshots
    ///
    /// # Returns
    ///
    /// A vector of references to all snapshots
    pub fn list_snapshots(&self) -> Vec<&StateSnapshot> {
        self.snapshots.values().collect()
    }

    /// List snapshots sorted by timestamp (newest first)
    ///
    /// # Returns
    ///
    /// A vector of references to snapshots, sorted by timestamp
    pub fn list_snapshots_by_time(&self) -> Vec<&StateSnapshot> {
        let mut snapshots: Vec<&StateSnapshot> = self.snapshots.values().collect();
        snapshots.sort_by(|a, b| b.timestamp().cmp(&a.timestamp()));
        snapshots
    }

    /// Delete a snapshot
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the snapshot to delete
    ///
    /// # Returns
    ///
    /// A Result containing the deleted snapshot or an error
    pub fn delete_snapshot(&mut self, id: &SnapshotId) -> SnapshotResult<StateSnapshot> {
        let snapshot = self.snapshots.remove(id).ok_or_else(|| SnapshotError::NotFound(id.clone()))?;

        // If this is a contract snapshot, release the version reference
        if let Some(dot_address) = snapshot.dot_address {
            self.version_manager.release_snapshot(dot_address, snapshot.version_id);
        }

        Ok(snapshot)
    }

    /// Get snapshots by height range
    ///
    /// # Arguments
    ///
    /// * `min_height` - Minimum block height
    /// * `max_height` - Maximum block height
    ///
    /// # Returns
    ///
    /// A vector of references to snapshots within the height range
    pub fn get_snapshots_by_height(&self, min_height: u64, max_height: u64) -> Vec<&StateSnapshot> {
        self.snapshots
            .values()
            .filter(|snapshot| snapshot.height.map(|h| h >= min_height && h <= max_height).unwrap_or(false))
            .collect()
    }

    /// Get the latest snapshot
    ///
    /// # Returns
    ///
    /// Optional reference to the most recent snapshot
    pub fn get_latest_snapshot(&self) -> Option<&StateSnapshot> {
        self.snapshots.values().max_by_key(|snapshot| snapshot.timestamp())
    }

    /// Get snapshots newer than specified timestamp
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp threshold
    ///
    /// # Returns
    ///
    /// A vector of references to snapshots newer than the timestamp
    pub fn get_snapshots_after(&self, timestamp: u64) -> Vec<&StateSnapshot> {
        self.snapshots.values().filter(|snapshot| snapshot.timestamp() > timestamp).collect()
    }

    /// Cleanup old snapshots based on configuration
    ///
    /// # Returns
    ///
    /// A Result containing the number of snapshots removed
    pub fn cleanup_old_snapshots(&mut self) -> SnapshotResult<usize> {
        let mut removed_count = 0;

        // Remove snapshots older than max_age_seconds
        if let Some(max_age) = self.config.max_age_seconds {
            let to_remove: Vec<SnapshotId> = self.snapshots.iter().filter(|(_, snapshot)| snapshot.is_older_than(max_age)).map(|(id, _)| id.clone()).collect();

            for id in to_remove {
                self.snapshots.remove(&id);
                removed_count += 1;
            }
        }

        // Remove excess snapshots if we have more than max_snapshots
        if let Some(max_count) = self.config.max_snapshots
            && self.snapshots.len() > max_count
        {
            let mut snapshots: Vec<(SnapshotId, StateSnapshot)> = self.snapshots.drain().collect();

            // Sort by timestamp (newest first)
            snapshots.sort_by(|a, b| b.1.timestamp().cmp(&a.1.timestamp()));

            // Keep only the newest max_count snapshots
            let excess_count = snapshots.len() - max_count;
            snapshots.truncate(max_count);
            removed_count += excess_count;

            // Put back the remaining snapshots
            self.snapshots = snapshots.into_iter().collect();
        }

        Ok(removed_count)
    }

    /// Restore state from a snapshot
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - The ID of the snapshot to restore from
    /// * `base_trie` - The base trie to use for restoration
    ///
    /// # Returns
    ///
    /// A Result containing the restored trie or an error
    pub fn restore_from_snapshot(&self, snapshot_id: &SnapshotId, base_trie: MerklePatriciaTrie<S>) -> SnapshotResult<MerklePatriciaTrie<S>> {
        let snapshot = self.get_snapshot(snapshot_id)?;

        // Create a new trie with the snapshot's root
        let mut restored_trie = base_trie;
        restored_trie.set_root(snapshot.root_hash);

        // Verify the restored state
        let restored_root = restored_trie.root_hash();
        if restored_root != snapshot.root_hash {
            return Err(SnapshotError::InvalidSnapshot(format!(
                "State restoration failed: root hash mismatch (expected {:?}, got {:?})",
                snapshot.root_hash, restored_root
            )));
        }

        // Apply any metadata-based state modifications
        if let Some(height) = snapshot.height {
            let _ = restored_trie.add_metadata("height".to_string(), height.to_string());
        }
        if let Some(desc) = &snapshot.description {
            let _ = restored_trie.add_metadata("description".to_string(), desc.clone());
        }

        // Add restoration metadata
        let _ = restored_trie.add_metadata("restored_from".to_string(), snapshot_id.clone());
        let _ = restored_trie.add_metadata("restored_at".to_string(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().to_string());

        Ok(restored_trie)
    }

    /// Get snapshot management statistics
    ///
    /// # Returns
    ///
    /// Statistics about the current snapshot state
    pub fn get_statistics(&self) -> SnapshotStatistics {
        let mut stats = SnapshotStatistics {
            total_snapshots: self.snapshots.len(),
            oldest_timestamp: None,
            newest_timestamp: None,
            estimated_size_bytes: 0,
        };

        if !self.snapshots.is_empty() {
            stats.oldest_timestamp = self.snapshots.values().map(|s| s.timestamp()).min();
            stats.newest_timestamp = self.snapshots.values().map(|s| s.timestamp()).max();
            stats.estimated_size_bytes = self.snapshots.values().map(|s| s.metadata.len() * std::mem::size_of::<String>() * 2).sum();
        }

        stats
    }

    /// Check if a snapshot exists
    ///
    /// # Arguments
    ///
    /// * `id` - The snapshot ID to check
    ///
    /// # Returns
    ///
    /// True if the snapshot exists, false otherwise
    pub fn snapshot_exists(&self, id: &SnapshotId) -> bool {
        self.snapshots.contains_key(id)
    }

    /// Update snapshot metadata
    ///
    /// # Arguments
    ///
    /// * `id` - The snapshot ID to update
    /// * `metadata` - New metadata to set
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    pub fn update_snapshot_metadata(&mut self, id: &SnapshotId, metadata: HashMap<String, String>) -> SnapshotResult<()> {
        let snapshot = self.snapshots.get_mut(id).ok_or_else(|| SnapshotError::NotFound(id.clone()))?;
        snapshot.metadata = metadata;
        Ok(())
    }

    /// Get a reference to the underlying storage
    ///
    /// # Returns
    ///
    /// A clone of the storage implementation
    pub fn get_storage(&self) -> SnapshotResult<S>
    where
        S: Clone,
    {
        self.current_trie
            .as_ref()
            .ok_or_else(|| SnapshotError::InvalidSnapshot("No current trie available".to_string()))
            .map(|trie| trie.get_storage_clone())
    }

    /// Get mutable access to the current trie for node operations
    ///
    /// # Returns
    ///
    /// A mutable reference to the current trie, or an error if no trie is set
    pub fn get_current_trie_mut(&mut self) -> SnapshotResult<&mut MerklePatriciaTrie<S>> {
        self.current_trie.as_mut().ok_or_else(|| SnapshotError::InvalidSnapshot("No current trie available".to_string()))
    }

    /// Get snapshots for a specific contract
    ///
    /// # Arguments
    ///
    /// * `dot_address` - Contract address to filter by
    ///
    /// # Returns
    ///
    /// Vector of snapshots for the specified contract
    pub fn get_dot_snapshots(&self, dot_address: DotAddress) -> Vec<&StateSnapshot> {
        self.snapshots.values().filter(|snapshot| snapshot.dot_address == Some(dot_address)).collect()
    }

    /// Get global (non-contract-specific) snapshots
    ///
    /// # Returns
    ///
    /// Vector of global snapshots
    pub fn get_global_snapshots(&self) -> Vec<&StateSnapshot> {
        self.snapshots.values().filter(|snapshot| snapshot.dot_address.is_none()).collect()
    }
}

/// Statistics about snapshot management
///
/// This struct provides various metrics about the current state of
/// snapshot management for monitoring and analysis.
#[derive(Debug, Clone)]
pub struct SnapshotStatistics {
    /// Total number of snapshots
    pub total_snapshots: usize,
    /// Timestamp of the oldest snapshot
    pub oldest_timestamp: Option<u64>,
    /// Timestamp of the newest snapshot
    pub newest_timestamp: Option<u64>,
    /// Estimated total size of snapshots in bytes
    pub estimated_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mpt::MerklePatriciaTrie;
    use crate::state::mpt::trie::InMemoryStorage;

    fn create_mock_trie() -> MerklePatriciaTrie<InMemoryStorage> {
        MerklePatriciaTrie::new_in_memory()
    }

    #[test]
    fn test_state_snapshot_creation() {
        let id = "test_snapshot".to_string();
        let root_hash = [1u8; 32];
        let height = Some(100);
        let description = Some("Test snapshot".to_string());

        let snapshot = StateSnapshot::new(id.clone(), root_hash, height, description.clone());

        assert_eq!(snapshot.id, id);
        assert_eq!(snapshot.root_hash, root_hash);
        assert_eq!(snapshot.height, height);
        assert_eq!(snapshot.description, description);
        assert!(snapshot.timestamp() > 0);
        assert!(snapshot.metadata.is_empty());
    }

    #[test]
    fn test_snapshot_metadata() {
        let mut snapshot = StateSnapshot::new("test".to_string(), [1u8; 32], None, None);

        assert!(snapshot.get_metadata("key1").is_none());

        snapshot.add_metadata("key1".to_string(), "value1".to_string());
        snapshot.add_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(snapshot.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(snapshot.get_metadata("key2"), Some(&"value2".to_string()));
        assert!(snapshot.get_metadata("key3").is_none());
    }

    #[test]
    fn test_snapshot_age() {
        let snapshot = StateSnapshot::new("test".to_string(), [1u8; 32], None, None);

        // Snapshot should be very new
        assert!(!snapshot.is_older_than(60)); // Not older than 1 minute
        assert!(snapshot.age_seconds() < 5); // Should be less than 5 seconds old
    }

    #[test]
    fn test_snapshot_manager_creation() {
        let config = SnapshotConfig::default();
        let manager: SnapshotManager<InMemoryStorage> = SnapshotManager::new(config);

        assert_eq!(manager.list_snapshots().len(), 0);
        assert!(manager.get_latest_snapshot().is_none());
    }

    #[test]
    fn test_snapshot_creation_and_retrieval() {
        let mut manager: SnapshotManager<InMemoryStorage> = SnapshotManager::with_defaults();
        let trie = create_mock_trie();
        let id = "test_snapshot".to_string();

        // Create snapshot
        let result = manager.create_snapshot(id.clone(), &trie, Some(100), Some("Test description".to_string()));
        assert!(result.is_ok());

        // Retrieve snapshot
        let retrieved = manager.get_snapshot(&id);
        assert!(retrieved.is_ok());
        let snapshot = retrieved.unwrap();
        assert_eq!(snapshot.id, id);
        assert_eq!(snapshot.height, Some(100));

        // Try to create duplicate
        let duplicate_result = manager.create_snapshot(id.clone(), &trie, Some(101), None);
        assert!(matches!(duplicate_result, Err(SnapshotError::AlreadyExists(_))));
    }

    #[test]
    fn test_snapshot_listing() {
        let mut manager: SnapshotManager<InMemoryStorage> = SnapshotManager::with_defaults();
        let trie = create_mock_trie();

        // Create multiple snapshots
        for i in 0..5 {
            let id = format!("snapshot_{}", i);
            let height = Some(i as u64 * 10);
            manager.create_snapshot(id, &trie, height, None).unwrap();
        }

        let snapshots = manager.list_snapshots();
        assert_eq!(snapshots.len(), 5);

        let by_time = manager.list_snapshots_by_time();
        assert_eq!(by_time.len(), 5);
        // Should be sorted by time (newest first)
        for i in 0..4 {
            assert!(by_time[i].timestamp() >= by_time[i + 1].timestamp());
        }
    }

    #[test]
    fn test_snapshot_deletion() {
        let mut manager: SnapshotManager<InMemoryStorage> = SnapshotManager::with_defaults();
        let trie = create_mock_trie();
        let id = "test_snapshot".to_string();

        // Create and then delete
        manager.create_snapshot(id.clone(), &trie, None, None).unwrap();
        assert!(manager.snapshot_exists(&id));

        let deleted = manager.delete_snapshot(&id);
        assert!(deleted.is_ok());
        assert!(!manager.snapshot_exists(&id));

        // Try to delete again
        let not_found = manager.delete_snapshot(&id);
        assert!(matches!(not_found, Err(SnapshotError::NotFound(_))));
    }

    #[test]
    fn test_snapshot_height_filtering() {
        let mut manager: SnapshotManager<InMemoryStorage> = SnapshotManager::with_defaults();
        let trie = create_mock_trie();

        // Create snapshots with different heights
        let heights = vec![10, 20, 30, 40, 50];
        for (i, &height) in heights.iter().enumerate() {
            let id = format!("snapshot_{}", i);
            manager.create_snapshot(id, &trie, Some(height), None).unwrap();
        }

        let filtered = manager.get_snapshots_by_height(25, 45);
        assert_eq!(filtered.len(), 2); // Heights 30, 40 should match

        for snapshot in filtered {
            let height = snapshot.height.unwrap();
            assert!(height >= 25 && height <= 45);
        }
    }

    #[test]
    fn test_snapshot_statistics() {
        let mut manager = SnapshotManager::with_defaults();
        let mut trie = create_mock_trie();

        // Add some data to the trie to ensure non-zero size
        for i in 0..5 {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            trie.put(key, value).unwrap();
        }

        // Initially empty
        let stats = manager.get_statistics();
        assert_eq!(stats.total_snapshots, 0);
        assert!(stats.oldest_timestamp.is_none());
        assert!(stats.newest_timestamp.is_none());
        assert_eq!(stats.estimated_size_bytes, 0);

        // Add some snapshots
        for i in 0..3 {
            let id = format!("snapshot_{}", i);
            let mut snapshot = manager.create_snapshot(id.clone(), &trie, Some(i as u64), None).unwrap();
            // Metadata ekle
            manager
                .update_snapshot_metadata(&id, [("meta_key".to_string(), format!("meta_value_{}", i))].iter().cloned().collect())
                .unwrap();
        }

        let stats = manager.get_statistics();
        assert_eq!(stats.total_snapshots, 3);
        assert!(stats.oldest_timestamp.is_some());
        assert!(stats.newest_timestamp.is_some());
        assert!(stats.estimated_size_bytes > 0, "Expected non-zero estimated size");
    }

    #[test]
    fn test_snapshot_metadata_update() {
        let mut manager: SnapshotManager<InMemoryStorage> = SnapshotManager::with_defaults();
        let trie = create_mock_trie();
        let id = "test_snapshot".to_string();

        manager.create_snapshot(id.clone(), &trie, None, None).unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1.0".to_string());
        metadata.insert("author".to_string(), "test".to_string());

        let result = manager.update_snapshot_metadata(&"nonexistent".to_string(), HashMap::new());
        assert!(matches!(result, Err(SnapshotError::NotFound(_))));
    }
}
