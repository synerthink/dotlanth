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

//! Contract State Versioning System (DOTVM-39)
//!
//! This module implements the state versioning system for smart contracts as defined in DOTVM-39.
//! It leverages the Merkle Patricia Trie (MPT) root hash changes to version contract state,
//! enabling historical queries, contract upgrades tracking, and data migration support.
//!
//! # Key Features
//!
//! - **MPT-based versioning**: Each transaction modifying contract state creates a new MPT root
//! - **Historical state queries**: Query contract state at specific historical state roots
//! - **Contract upgrade tracking**: Track contract upgrades and data migrations
//! - **Efficient storage**: Only store incremental changes between versions
//! - **Atomic operations**: Ensure all state changes are applied atomically
//!
//! # Architecture
//!
//! The system builds upon the global state tree (MPT from DOTVM-38) to define how individual
//! contracts store, version, and validate their state. Each contract has its own subtree
//! within the MPT, derived from its address.
//!
//! ## Version Structure
//!
//! ```text
//! Contract State Version = {
//!     version_id: StateVersionId,
//!     mpt_root_hash: Hash,
//!     dot_address: DotAddress,
//!     parent_version: Option<StateVersionId>,
//!     transaction_hash: Option<Hash>,
//!     block_height: Option<u64>,
//!     upgrade_info: Option<DotUpgradeInfo>
//! }
//! ```

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::dot_storage_layout::{DotAddress, StorageLayoutError};
use crate::state::mpt::{Hash, MPTError};

/// Timestamp type for versioning
pub type Timestamp = u64;

/// Transaction hash type
pub type TransactionHash = Hash;

/// Block height type
pub type BlockHeight = u64;

/// Contract state version identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct StateVersionId {
    /// Logical version number
    pub logical_version: u64,
    /// Physical timestamp when version was created
    pub timestamp: Timestamp,
}

impl StateVersionId {
    /// Create a new state version ID
    pub fn new(logical_version: u64, timestamp: Timestamp) -> Self {
        Self { logical_version, timestamp }
    }

    /// Create a new state version ID with current timestamp
    pub fn new_with_current_time(logical_version: u64) -> Self {
        Self::new(logical_version, current_timestamp())
    }

    /// Get the logical version
    pub fn logical_version(&self) -> u64 {
        self.logical_version
    }

    /// Get the timestamp
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

impl Default for StateVersionId {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Contract upgrade information
#[derive(Debug, Clone)]
pub struct DotUpgradeInfo {
    /// Previous contract version
    pub previous_version: StateVersionId,
    /// Upgrade type
    pub upgrade_type: UpgradeType,
    /// Migration description
    pub migration_description: String,
    /// Storage layout changes
    pub layout_changes: Vec<LayoutChange>,
    /// Upgrade timestamp
    pub upgrade_timestamp: Timestamp,
}

/// Types of contract upgrades
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeType {
    /// Minor upgrade with backward compatibility
    Minor,
    /// Major upgrade with potential breaking changes
    Major,
    /// Storage layout migration
    StorageMigration,
    /// Security patch
    SecurityPatch,
    /// Complete contract replacement
    Replacement,
}

/// Storage layout changes during upgrade
#[derive(Debug, Clone)]
pub struct LayoutChange {
    /// Type of change
    pub change_type: LayoutChangeType,
    /// Variable name affected
    pub variable_name: String,
    /// Old storage slot (if applicable)
    pub old_slot: Option<u32>,
    /// New storage slot (if applicable)
    pub new_slot: Option<u32>,
    /// Migration strategy
    pub migration_strategy: String,
}

/// Types of storage layout changes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutChangeType {
    /// New variable added
    Added,
    /// Variable removed
    Removed,
    /// Variable type changed
    TypeChanged,
    /// Variable moved to different slot
    SlotChanged,
    /// Variable renamed
    Renamed,
}

/// Contract state version metadata
#[derive(Debug, Clone)]
pub struct DotStateVersion {
    /// Version identifier
    pub version_id: StateVersionId,
    /// MPT root hash at this version
    pub mpt_root_hash: Hash,
    /// Contract address
    pub dot_address: DotAddress,
    /// Parent version (previous state)
    pub parent_version: Option<StateVersionId>,
    /// Transaction that created this version
    pub transaction_hash: Option<TransactionHash>,
    /// Block height when version was created
    pub block_height: Option<BlockHeight>,
    /// Contract upgrade information (if this is an upgrade)
    pub upgrade_info: Option<DotUpgradeInfo>,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Version description
    pub description: String,
    /// Whether this version is finalized
    pub is_finalized: bool,
    /// State size in bytes
    pub state_size: u64,
    /// Number of storage slots used
    pub storage_slots_count: u64,
}

impl DotStateVersion {
    /// Create a new contract state version
    pub fn new(version_id: StateVersionId, mpt_root_hash: Hash, dot_address: DotAddress, parent_version: Option<StateVersionId>, description: String) -> Self {
        Self {
            version_id,
            mpt_root_hash,
            dot_address,
            parent_version,
            transaction_hash: None,
            block_height: None,
            upgrade_info: None,
            created_at: current_timestamp(),
            description,
            is_finalized: false,
            state_size: 0,
            storage_slots_count: 0,
        }
    }

    /// Create a new version for contract upgrade
    pub fn new_upgrade(
        version_id: StateVersionId,
        mpt_root_hash: Hash,
        dot_address: DotAddress,
        parent_version: StateVersionId,
        upgrade_info: DotUpgradeInfo,
        description: String,
    ) -> Self {
        Self {
            version_id,
            mpt_root_hash,
            dot_address,
            parent_version: Some(parent_version),
            transaction_hash: None,
            block_height: None,
            upgrade_info: Some(upgrade_info),
            created_at: current_timestamp(),
            description,
            is_finalized: false,
            state_size: 0,
            storage_slots_count: 0,
        }
    }

    /// Set transaction information
    pub fn set_transaction_info(&mut self, tx_hash: TransactionHash, block_height: BlockHeight) {
        self.transaction_hash = Some(tx_hash);
        self.block_height = Some(block_height);
    }

    /// Update state statistics
    pub fn update_stats(&mut self, state_size: u64, storage_slots_count: u64) {
        self.state_size = state_size;
        self.storage_slots_count = storage_slots_count;
    }

    /// Mark version as finalized
    pub fn finalize(&mut self) {
        self.is_finalized = true;
    }

    /// Check if this is an upgrade version
    pub fn is_upgrade(&self) -> bool {
        self.upgrade_info.is_some()
    }

    /// Get upgrade type if this is an upgrade
    pub fn upgrade_type(&self) -> Option<&UpgradeType> {
        self.upgrade_info.as_ref().map(|info| &info.upgrade_type)
    }
}

/// Contract versioning manager
pub struct DotVersionManager {
    /// Contract versions by contract address and version ID
    versions: RwLock<HashMap<DotAddress, BTreeMap<StateVersionId, DotStateVersion>>>,
    /// Current version for each contract
    current_versions: RwLock<HashMap<DotAddress, StateVersionId>>,
    /// Version counter for generating new version IDs
    version_counter: Mutex<u64>,
    /// Maximum versions to keep per contract
    max_versions_per_dot: usize,
    /// Active snapshots reference counting
    active_snapshots: Mutex<HashMap<(DotAddress, StateVersionId), usize>>,
}

impl DotVersionManager {
    /// Create a new contract version manager
    pub fn new(max_versions_per_dot: usize) -> Self {
        Self {
            versions: RwLock::new(HashMap::new()),
            current_versions: RwLock::new(HashMap::new()),
            version_counter: Mutex::new(0),
            max_versions_per_dot,
            active_snapshots: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new version for a contract
    pub fn create_version(&self, dot_address: DotAddress, mpt_root_hash: Hash, description: String) -> Result<StateVersionId, DotVersioningError> {
        let mut counter = self.version_counter.lock().unwrap();
        *counter += 1;
        let version_id = StateVersionId::new_with_current_time(*counter);
        drop(counter);

        let current_version = {
            let current_versions = self.current_versions.read().unwrap();
            current_versions.get(&dot_address).copied()
        };

        let version = DotStateVersion::new(version_id, mpt_root_hash, dot_address, current_version, description);

        {
            let mut versions = self.versions.write().unwrap();
            let dot_versions = versions.entry(dot_address).or_default();
            dot_versions.insert(version_id, version);

            // Cleanup old versions if necessary
            self.cleanup_old_versions_for_dot(dot_versions)?;
        }

        {
            let mut current_versions = self.current_versions.write().unwrap();
            current_versions.insert(dot_address, version_id);
        }

        Ok(version_id)
    }

    /// Create a new version for contract upgrade
    pub fn create_upgrade_version(
        &self,
        dot_address: DotAddress,
        mpt_root_hash: Hash,
        upgrade_info: DotUpgradeInfo,
        description: String,
    ) -> Result<StateVersionId, DotVersioningError> {
        let current_version = {
            let current_versions = self.current_versions.read().unwrap();
            current_versions.get(&dot_address).copied().ok_or(DotVersioningError::DotNotFound(dot_address))?
        };

        let mut counter = self.version_counter.lock().unwrap();
        *counter += 1;
        let version_id = StateVersionId::new_with_current_time(*counter);
        drop(counter);

        let version = DotStateVersion::new_upgrade(version_id, mpt_root_hash, dot_address, current_version, upgrade_info, description);

        {
            let mut versions = self.versions.write().unwrap();
            let dot_versions = versions.entry(dot_address).or_default();
            dot_versions.insert(version_id, version);
        }

        {
            let mut current_versions = self.current_versions.write().unwrap();
            current_versions.insert(dot_address, version_id);
        }

        Ok(version_id)
    }

    /// Get a specific version of a contract
    pub fn get_version(&self, dot_address: DotAddress, version_id: StateVersionId) -> Option<DotStateVersion> {
        let versions = self.versions.read().unwrap();
        versions.get(&dot_address)?.get(&version_id).cloned()
    }

    /// Get the current version of a contract
    pub fn get_current_version(&self, dot_address: DotAddress) -> Option<DotStateVersion> {
        let current_versions = self.current_versions.read().unwrap();
        let current_version_id = *current_versions.get(&dot_address)?;
        drop(current_versions);

        self.get_version(dot_address, current_version_id)
    }

    /// Get all versions of a contract
    pub fn get_all_versions(&self, dot_address: DotAddress) -> Vec<DotStateVersion> {
        let versions = self.versions.read().unwrap();
        if let Some(dot_versions) = versions.get(&dot_address) {
            dot_versions.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get versions in a specific time range
    pub fn get_versions_in_range(&self, dot_address: DotAddress, start_time: Timestamp, end_time: Timestamp) -> Vec<DotStateVersion> {
        self.get_all_versions(dot_address)
            .into_iter()
            .filter(|version| version.created_at >= start_time && version.created_at <= end_time)
            .collect()
    }

    /// Get version at specific block height
    pub fn get_version_at_block(&self, dot_address: DotAddress, block_height: BlockHeight) -> Option<DotStateVersion> {
        self.get_all_versions(dot_address)
            .into_iter()
            .filter(|version| version.block_height.is_some())
            .filter(|version| version.block_height.unwrap() <= block_height)
            .max_by_key(|version| version.block_height.unwrap())
    }

    /// Get all upgrade versions for a contract
    pub fn get_upgrade_versions(&self, dot_address: DotAddress) -> Vec<DotStateVersion> {
        self.get_all_versions(dot_address).into_iter().filter(|version| version.is_upgrade()).collect()
    }

    /// Query historical state at specific MPT root
    pub fn query_historical_state(&self, dot_address: DotAddress, mpt_root_hash: Hash) -> Option<DotStateVersion> {
        self.get_all_versions(dot_address).into_iter().find(|version| version.mpt_root_hash == mpt_root_hash)
    }

    /// Finalize a version
    pub fn finalize_version(&self, dot_address: DotAddress, version_id: StateVersionId) -> Result<(), DotVersioningError> {
        let mut versions = self.versions.write().unwrap();
        let dot_versions = versions.get_mut(&dot_address).ok_or(DotVersioningError::DotNotFound(dot_address))?;

        let version = dot_versions.get_mut(&version_id).ok_or(DotVersioningError::VersionNotFound(version_id))?;

        version.finalize();
        Ok(())
    }

    /// Update version transaction information
    pub fn update_transaction_info(&self, dot_address: DotAddress, version_id: StateVersionId, tx_hash: TransactionHash, block_height: BlockHeight) -> Result<(), DotVersioningError> {
        let mut versions = self.versions.write().unwrap();
        let dot_versions = versions.get_mut(&dot_address).ok_or(DotVersioningError::DotNotFound(dot_address))?;

        let version = dot_versions.get_mut(&version_id).ok_or(DotVersioningError::VersionNotFound(version_id))?;

        version.set_transaction_info(tx_hash, block_height);
        Ok(())
    }

    /// Acquire snapshot reference
    pub fn acquire_snapshot(&self, dot_address: DotAddress, version_id: StateVersionId) -> Result<(), DotVersioningError> {
        let mut active_snapshots = self.active_snapshots.lock().unwrap();
        let key = (dot_address, version_id);
        *active_snapshots.entry(key).or_insert(0) += 1;
        Ok(())
    }

    /// Release snapshot reference
    pub fn release_snapshot(&self, dot_address: DotAddress, version_id: StateVersionId) {
        let mut active_snapshots = self.active_snapshots.lock().unwrap();
        let key = (dot_address, version_id);
        if let Some(count) = active_snapshots.get_mut(&key) {
            *count -= 1;
            if *count == 0 {
                active_snapshots.remove(&key);
            }
        }
    }

    /// Check if version is actively referenced
    pub fn is_version_active(&self, dot_address: DotAddress, version_id: StateVersionId) -> bool {
        let active_snapshots = self.active_snapshots.lock().unwrap();
        active_snapshots.contains_key(&(dot_address, version_id))
    }

    /// Get contract versioning statistics
    pub fn get_dot_statistics(&self, dot_address: DotAddress) -> DotVersioningStatistics {
        let versions = self.versions.read().unwrap();
        let current_versions = self.current_versions.read().unwrap();

        if let Some(dot_versions) = versions.get(&dot_address) {
            let total_versions = dot_versions.len();
            let finalized_versions = dot_versions.values().filter(|v| v.is_finalized).count();
            let upgrade_versions = dot_versions.values().filter(|v| v.is_upgrade()).count();
            let current_version = current_versions.get(&dot_address).copied();

            let total_state_size = dot_versions.values().map(|v| v.state_size).sum();

            DotVersioningStatistics {
                dot_address,
                total_versions,
                finalized_versions,
                upgrade_versions,
                current_version,
                total_state_size_bytes: total_state_size,
                max_versions_per_dot: self.max_versions_per_dot,
            }
        } else {
            DotVersioningStatistics {
                dot_address,
                total_versions: 0,
                finalized_versions: 0,
                upgrade_versions: 0,
                current_version: None,
                total_state_size_bytes: 0,
                max_versions_per_dot: self.max_versions_per_dot,
            }
        }
    }

    /// Clean up old versions for a contract
    fn cleanup_old_versions_for_dot(&self, dot_versions: &mut BTreeMap<StateVersionId, DotStateVersion>) -> Result<(), DotVersioningError> {
        if dot_versions.len() <= self.max_versions_per_dot {
            return Ok(());
        }

        // Keep finalized versions and recent versions
        let mut versions_to_remove = Vec::new();
        let mut version_list: Vec<_> = dot_versions.iter().collect();
        version_list.sort_by_key(|(version_id, _)| version_id.timestamp);

        // Remove oldest non-finalized versions
        for (version_id, version) in version_list.iter().take(dot_versions.len() - self.max_versions_per_dot) {
            if !version.is_finalized && !self.is_version_active(version.dot_address, **version_id) {
                versions_to_remove.push(**version_id);
            }
        }

        for version_id in versions_to_remove {
            dot_versions.remove(&version_id);
        }

        Ok(())
    }
}

impl Default for DotVersionManager {
    fn default() -> Self {
        Self::new(100) // Default: keep 100 versions per contract
    }
}

/// Contract versioning statistics
#[derive(Debug, Clone)]
pub struct DotVersioningStatistics {
    /// Contract address
    pub dot_address: DotAddress,
    /// Total number of versions
    pub total_versions: usize,
    /// Number of finalized versions
    pub finalized_versions: usize,
    /// Number of upgrade versions
    pub upgrade_versions: usize,
    /// Current version ID
    pub current_version: Option<StateVersionId>,
    /// Total state size across all versions
    pub total_state_size_bytes: u64,
    /// Maximum versions to keep
    pub max_versions_per_dot: usize,
}

/// Errors that can occur during contract versioning operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DotVersioningError {
    /// Contract not found
    DotNotFound(DotAddress),
    /// Version not found
    VersionNotFound(StateVersionId),
    /// Version already exists
    VersionAlreadyExists(StateVersionId),
    /// Invalid upgrade operation
    InvalidUpgrade(String),
    /// Storage layout error
    StorageLayoutError(String),
    /// MPT operation error
    MPTError(String),
    /// Serialization error
    SerializationError(String),
    /// Internal error
    InternalError(String),
}

impl std::fmt::Display for DotVersioningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DotVersioningError::DotNotFound(addr) => {
                write!(f, "Contract not found: {addr:?}")
            }
            DotVersioningError::VersionNotFound(version) => {
                write!(f, "Version not found: {version:?}")
            }
            DotVersioningError::VersionAlreadyExists(version) => {
                write!(f, "Version already exists: {version:?}")
            }
            DotVersioningError::InvalidUpgrade(msg) => {
                write!(f, "Invalid upgrade: {msg}")
            }
            DotVersioningError::StorageLayoutError(msg) => {
                write!(f, "Storage layout error: {msg}")
            }
            DotVersioningError::MPTError(msg) => {
                write!(f, "MPT error: {msg}")
            }
            DotVersioningError::SerializationError(msg) => {
                write!(f, "Serialization error: {msg}")
            }
            DotVersioningError::InternalError(msg) => {
                write!(f, "Internal error: {msg}")
            }
        }
    }
}

impl std::error::Error for DotVersioningError {}

impl From<StorageLayoutError> for DotVersioningError {
    fn from(err: StorageLayoutError) -> Self {
        DotVersioningError::StorageLayoutError(format!("{err:?}"))
    }
}

impl From<MPTError> for DotVersioningError {
    fn from(err: MPTError) -> Self {
        DotVersioningError::MPTError(format!("{err:?}"))
    }
}

/// Utility functions for contract versioning
pub mod dot_version_utils {
    use super::*;

    /// Compare two contract state versions
    pub fn compare_versions(v1: StateVersionId, v2: StateVersionId) -> std::cmp::Ordering {
        v1.cmp(&v2)
    }

    /// Check if version v1 is newer than v2
    pub fn is_newer(v1: StateVersionId, v2: StateVersionId) -> bool {
        v1 > v2
    }

    /// Calculate the time difference between versions
    pub fn version_time_diff(v1: StateVersionId, v2: StateVersionId) -> u64 {
        v1.timestamp.abs_diff(v2.timestamp)
    }

    /// Find the common ancestor version between two versions
    pub fn find_common_ancestor(v1: StateVersionId, v2: StateVersionId, dot_address: DotAddress, manager: &DotVersionManager) -> Option<StateVersionId> {
        let versions = manager.get_all_versions(dot_address);
        let mut v1_chain = Vec::new();
        let mut v2_chain = Vec::new();

        // Build version chains
        if let Some(version) = versions.iter().find(|v| v.version_id == v1) {
            build_version_chain(version, &versions, &mut v1_chain);
        }

        if let Some(version) = versions.iter().find(|v| v.version_id == v2) {
            build_version_chain(version, &versions, &mut v2_chain);
        }

        // Find common ancestor
        for v1_ancestor in &v1_chain {
            if v2_chain.contains(v1_ancestor) {
                return Some(*v1_ancestor);
            }
        }

        None
    }

    /// Build version chain from current version to root
    fn build_version_chain(version: &DotStateVersion, all_versions: &[DotStateVersion], chain: &mut Vec<StateVersionId>) {
        chain.push(version.version_id);

        if let Some(parent_id) = version.parent_version
            && let Some(parent) = all_versions.iter().find(|v| v.version_id == parent_id)
        {
            build_version_chain(parent, all_versions, chain);
        }
    }

    /// Get versions between two versions
    pub fn get_versions_between(start: StateVersionId, end: StateVersionId, dot_address: DotAddress, manager: &DotVersionManager) -> Vec<StateVersionId> {
        manager
            .get_all_versions(dot_address)
            .into_iter()
            .filter(|version| version.version_id >= start && version.version_id <= end)
            .map(|version| version.version_id)
            .collect()
    }

    /// Check if an upgrade is compatible
    pub fn is_upgrade_compatible(from_version: &DotStateVersion, to_version: &DotStateVersion) -> bool {
        if let Some(upgrade_info) = &to_version.upgrade_info {
            match upgrade_info.upgrade_type {
                UpgradeType::Minor => true,
                UpgradeType::SecurityPatch => true,
                UpgradeType::Major => {
                    // Check if breaking changes are acceptable
                    upgrade_info
                        .layout_changes
                        .iter()
                        .all(|change| !matches!(change.change_type, LayoutChangeType::Removed | LayoutChangeType::TypeChanged))
                }
                UpgradeType::StorageMigration => {
                    // All storage migrations require special handling
                    false
                }
                UpgradeType::Replacement => {
                    // Complete replacements are never compatible
                    false
                }
            }
        } else {
            true // Non-upgrade versions are always compatible
        }
    }
}

/// Get current timestamp
fn current_timestamp() -> Timestamp {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as Timestamp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dot_address() -> DotAddress {
        [1u8; 20]
    }

    fn create_test_mpt_root() -> Hash {
        [42u8; 32]
    }

    #[test]
    fn test_state_version_id_creation() {
        let version_id = StateVersionId::new(1, 1000);
        assert_eq!(version_id.logical_version(), 1);
        assert_eq!(version_id.timestamp(), 1000);

        let version_id_current = StateVersionId::new_with_current_time(2);
        assert_eq!(version_id_current.logical_version(), 2);
        assert!(version_id_current.timestamp() > 0);
    }

    #[test]
    fn test_dot_state_version_creation() {
        let version_id = StateVersionId::new(1, 1000);
        let dot_addr = create_test_dot_address();
        let mpt_root = create_test_mpt_root();

        let version = DotStateVersion::new(version_id, mpt_root, dot_addr, None, "Initial version".to_string());

        assert_eq!(version.version_id, version_id);
        assert_eq!(version.mpt_root_hash, mpt_root);
        assert_eq!(version.dot_address, dot_addr);
        assert_eq!(version.parent_version, None);
        assert!(!version.is_upgrade());
        assert!(!version.is_finalized);
    }

    #[test]
    fn test_dot_upgrade_version() {
        let version_id = StateVersionId::new(2, 2000);
        let parent_version_id = StateVersionId::new(1, 1000);
        let dot_addr = create_test_dot_address();
        let mpt_root = create_test_mpt_root();

        let upgrade_info = DotUpgradeInfo {
            previous_version: parent_version_id,
            upgrade_type: UpgradeType::Major,
            migration_description: "Added new feature".to_string(),
            layout_changes: vec![],
            upgrade_timestamp: 2000,
        };

        let version = DotStateVersion::new_upgrade(version_id, mpt_root, dot_addr, parent_version_id, upgrade_info, "Major upgrade".to_string());

        assert!(version.is_upgrade());
        assert_eq!(version.upgrade_type(), Some(&UpgradeType::Major));
        assert_eq!(version.parent_version, Some(parent_version_id));
    }

    #[test]
    fn test_dot_version_manager_basic_operations() {
        let manager = DotVersionManager::new(10);
        let dot_addr = create_test_dot_address();
        let mpt_root = create_test_mpt_root();

        // Create first version
        let version_id = manager.create_version(dot_addr, mpt_root, "Initial version".to_string()).unwrap();

        // Get current version
        let current_version = manager.get_current_version(dot_addr).unwrap();
        assert_eq!(current_version.version_id, version_id);
        assert_eq!(current_version.mpt_root_hash, mpt_root);

        // Get specific version
        let specific_version = manager.get_version(dot_addr, version_id).unwrap();
        assert_eq!(specific_version.version_id, version_id);
    }

    #[test]
    fn test_dot_versioning_statistics() {
        let manager = DotVersionManager::new(10);
        let dot_addr = create_test_dot_address();
        let mpt_root = create_test_mpt_root();

        // Create multiple versions
        let v1 = manager.create_version(dot_addr, mpt_root, "Version 1".to_string()).unwrap();
        let v2 = manager.create_version(dot_addr, mpt_root, "Version 2".to_string()).unwrap();

        // Finalize first version
        manager.finalize_version(dot_addr, v1).unwrap();

        let stats = manager.get_dot_statistics(dot_addr);
        assert_eq!(stats.total_versions, 2);
        assert_eq!(stats.finalized_versions, 1);
        assert_eq!(stats.current_version, Some(v2));
    }

    #[test]
    fn test_historical_state_query() {
        let manager = DotVersionManager::new(10);
        let dot_addr = create_test_dot_address();
        let mpt_root1 = [1u8; 32];
        let mpt_root2 = [2u8; 32];

        // Create versions with different MPT roots
        manager.create_version(dot_addr, mpt_root1, "Version 1".to_string()).unwrap();
        manager.create_version(dot_addr, mpt_root2, "Version 2".to_string()).unwrap();

        // Query historical state
        let historical_version = manager.query_historical_state(dot_addr, mpt_root1).unwrap();
        assert_eq!(historical_version.mpt_root_hash, mpt_root1);
        assert_eq!(historical_version.description, "Version 1");
    }

    #[test]
    fn test_version_snapshot_reference_counting() {
        let manager = DotVersionManager::new(10);
        let dot_addr = create_test_dot_address();
        let mpt_root = create_test_mpt_root();

        let version_id = manager.create_version(dot_addr, mpt_root, "Test version".to_string()).unwrap();

        // Acquire snapshot reference
        manager.acquire_snapshot(dot_addr, version_id).unwrap();
        assert!(manager.is_version_active(dot_addr, version_id));

        // Release snapshot reference
        manager.release_snapshot(dot_addr, version_id);
        assert!(!manager.is_version_active(dot_addr, version_id));
    }

    #[test]
    fn test_version_utils() {
        let v1 = StateVersionId::new(1, 1000);
        let v2 = StateVersionId::new(2, 2000);

        assert!(dot_version_utils::is_newer(v2, v1));
        assert_eq!(dot_version_utils::version_time_diff(v1, v2), 1000);
        assert_eq!(dot_version_utils::compare_versions(v1, v2), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_upgrade_compatibility() {
        let version_id = StateVersionId::new(1, 1000);
        let dot_addr = create_test_dot_address();
        let mpt_root = create_test_mpt_root();

        // Create non-upgrade version
        let non_upgrade_version = DotStateVersion::new(version_id, mpt_root, dot_addr, None, "Regular version".to_string());

        // Create minor upgrade version
        let upgrade_info = DotUpgradeInfo {
            previous_version: version_id,
            upgrade_type: UpgradeType::Minor,
            migration_description: "Minor upgrade".to_string(),
            layout_changes: vec![],
            upgrade_timestamp: 2000,
        };

        let upgrade_version = DotStateVersion::new_upgrade(StateVersionId::new(2, 2000), mpt_root, dot_addr, version_id, upgrade_info, "Minor upgrade".to_string());

        assert!(dot_version_utils::is_upgrade_compatible(&non_upgrade_version, &upgrade_version));
    }
}
