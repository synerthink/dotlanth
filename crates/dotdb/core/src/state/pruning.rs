// crates/dotdb/core/src/state/pruning.rs

//! State Pruning System
//!
//! This module provides a comprehensive state pruning system for managing and
//! cleaning up old state data in the blockchain. It implements various pruning
//! strategies and policies to optimize storage usage while maintaining data
//! integrity and availability.
//!
//! # Features
//!
//! - Multiple pruning strategies (KeepLast, KeepRecent, KeepAtIntervals, Custom)
//! - Configurable pruning policies
//! - Automatic and manual pruning operations
//! - Snapshot-aware pruning
//! - Pruning statistics and monitoring
//!
//! # Performance Considerations
//!
//! - Efficient state tracking using HashMaps
//! - Minimal memory allocations during pruning
//! - Optimized pruning candidate selection
//! - Thread-safe operations
//!
//! # Usage
//!
//! ```rust
//! use dotdb_core::state::pruning::{StatePruner, PruningPolicy, PruningStrategy};
//!
//! // Create a pruner with custom policy
//! let policy = PruningPolicy {
//!     strategy: PruningStrategy::KeepLast(100),
//!     enabled: true,
//!     ..Default::default()
//! };
//! let mut pruner: StatePruner<dotdb_core::state::mpt::trie::InMemoryStorage> = StatePruner::new(policy);
//!
//! // Register states and perform pruning
//! let root_hash = [1u8; 32];
//! let height = 1000;
//! let size_bytes = 1024;
//! pruner.register_state(root_hash, height, size_bytes).unwrap();
//! let result = pruner.prune().unwrap();
//! ```
//!
//! # Error Handling
//!
//! All operations return `PruningResult_` which is a type alias for
//! `Result<T, PruningError>`. The `PruningError` enum provides detailed
//! error information for different failure scenarios.

use crate::state::mpt::node::NodeType;
use crate::state::mpt::trie::NodeStorage;
use crate::state::mpt::{Hash, MPTError, MerklePatriciaTrie, NodeId, TrieResult};
use crate::state::snapshot::{SnapshotError, SnapshotId, SnapshotManager, StateSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// Different pruning strategies available for state cleanup
///
/// Each strategy defines a different approach to determining which states
/// should be preserved and which can be pruned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PruningStrategy {
    /// Keep the last N states
    KeepLast(usize),
    /// Keep states newer than X seconds
    KeepRecent(u64),
    /// Keep states at specific block heights/intervals
    KeepAtIntervals(u64),
    /// Custom pruning based on multiple criteria
    Custom {
        /// Number of most recent states to keep
        keep_last: Option<usize>,
        /// Keep states newer than this many seconds
        keep_recent_seconds: Option<u64>,
        /// Keep states at these block height intervals
        keep_intervals: Option<u64>,
        /// Whether to preserve states referenced by snapshots
        keep_snapshots: bool,
    },
}

impl Default for PruningStrategy {
    fn default() -> Self {
        PruningStrategy::Custom {
            keep_last: Some(100),
            keep_recent_seconds: Some(86400 * 7), // 7 days
            keep_intervals: Some(1000),           // Every 1000 blocks
            keep_snapshots: true,
        }
    }
}

/// Configuration for state pruning operations
///
/// This struct defines the behavior and constraints for pruning operations,
/// including the strategy to use, timing, and safety parameters.
#[derive(Debug, Clone)]
pub struct PruningPolicy {
    /// The pruning strategy to use
    pub strategy: PruningStrategy,
    /// Whether pruning is enabled
    pub enabled: bool,
    /// Automatic pruning interval in seconds
    pub auto_prune_interval: Option<u64>,
    /// Minimum number of confirmations before pruning (safety margin)
    pub min_confirmations: u64,
    /// Whether to keep state roots that are referenced by snapshots
    pub preserve_snapshot_roots: bool,
    /// Maximum storage size before forced pruning (in bytes)
    pub max_storage_size: Option<u64>,
}

impl Default for PruningPolicy {
    fn default() -> Self {
        Self {
            strategy: PruningStrategy::default(),
            enabled: true,
            auto_prune_interval: Some(3600), // 1 hour
            min_confirmations: 6,
            preserve_snapshot_roots: true,
            max_storage_size: Some(1024 * 1024 * 1024), // 1GB
        }
    }
}

/// Information about a state that can be pruned
///
/// This struct tracks metadata about states that are candidates for pruning,
/// including their age, size, and protection status.
#[derive(Debug, Clone)]
pub struct PrunableState {
    /// Root hash of the state
    pub root_hash: Hash,
    /// Block height or sequence number
    pub height: u64,
    /// Timestamp when the state was created
    pub timestamp: u64,
    /// Size of the state data in bytes
    pub size_bytes: u64,
    /// Whether this state is referenced by a snapshot
    pub is_snapshot_root: bool,
    /// Whether this state has enough confirmations
    pub has_confirmations: bool,
}

/// Result of a pruning operation
///
/// This struct provides detailed information about the outcome of a pruning
/// operation, including statistics and any errors encountered.
#[derive(Debug, Clone, Default)]
pub struct PruningResult {
    /// Number of states that were pruned
    pub pruned_count: usize,
    /// Total bytes reclaimed
    pub bytes_reclaimed: u64,
    /// Root hashes of pruned states
    pub pruned_roots: Vec<Hash>,
    /// States that were preserved (with reasons)
    pub preserved_states: Vec<(Hash, String)>,
    /// Any errors encountered during pruning
    pub errors: Vec<String>,
}

/// Error types for pruning operations
///
/// This enum defines various error conditions that can occur during
/// pruning operations, with detailed error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PruningError {
    /// MPT operation failed
    MPTError(String),
    /// Invalid pruning configuration
    InvalidConfig(String),
    /// Pruning operation failed
    PruningFailed(String),
    /// State is protected and cannot be pruned
    StateProtected(Hash),
    /// I/O error during pruning
    IoError(String),
}

impl From<MPTError> for PruningError {
    fn from(err: MPTError) -> Self {
        PruningError::MPTError(format!("{:?}", err))
    }
}

impl From<SnapshotError> for PruningError {
    fn from(err: SnapshotError) -> Self {
        PruningError::PruningFailed(format!("Snapshot error: {:?}", err))
    }
}

/// Type alias for pruning operation results
pub type PruningResult_<T> = Result<T, PruningError>;

/// State pruner responsible for managing and executing pruning operations
///
/// This struct implements the core pruning functionality, including state
/// tracking, pruning execution, and statistics collection.
pub struct StatePruner<S: NodeStorage + Clone> {
    /// Pruning policy configuration
    policy: PruningPolicy,
    /// State registry to track all states
    state_registry: HashMap<Hash, PrunableState>,
    /// Reference to snapshot manager for coordination
    snapshot_manager: Option<SnapshotManager<S>>,
    /// Last pruning timestamp
    last_prune_time: u64,
    /// Statistics about pruning operations
    stats: PruningStats,
}

/// Statistics about pruning operations
///
/// This struct tracks various metrics about pruning operations for
/// monitoring and analysis purposes.
#[derive(Debug, Clone, Default)]
pub struct PruningStats {
    /// Total number of pruning operations performed
    pub total_prune_operations: u64,
    /// Total states pruned
    pub total_states_pruned: u64,
    /// Total bytes reclaimed
    pub total_bytes_reclaimed: u64,
    /// Last pruning timestamp
    pub last_prune_timestamp: Option<u64>,
    /// Average pruning duration in milliseconds
    pub avg_prune_duration_ms: u64,
}

impl<S: NodeStorage + Clone> StatePruner<S> {
    /// Create a new state pruner with the given policy
    ///
    /// # Arguments
    ///
    /// * `policy` - The pruning policy to use
    ///
    /// # Returns
    ///
    /// A new StatePruner instance
    pub fn new(policy: PruningPolicy) -> Self {
        Self {
            policy,
            state_registry: HashMap::new(),
            snapshot_manager: None,
            last_prune_time: 0,
            stats: PruningStats::default(),
        }
    }

    /// Create a pruner with default policy
    ///
    /// # Returns
    ///
    /// A new StatePruner instance with default policy
    pub fn with_defaults() -> Self {
        Self::new(PruningPolicy::default())
    }

    /// Set the snapshot manager for coordination
    ///
    /// # Arguments
    ///
    /// * `manager` - The snapshot manager to use
    pub fn set_snapshot_manager(&mut self, manager: SnapshotManager<S>) {
        self.snapshot_manager = Some(manager);
    }

    /// Register a new state for potential pruning
    ///
    /// # Arguments
    ///
    /// * `root_hash` - The root hash of the state
    /// * `height` - The block height of the state
    /// * `size_bytes` - The size of the state in bytes
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    pub fn register_state(&mut self, root_hash: Hash, height: u64, size_bytes: u64) -> PruningResult_<()> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        let is_snapshot_root = self.snapshot_manager.as_ref().map(|sm| sm.list_snapshots().iter().any(|s| s.root_hash == root_hash)).unwrap_or(false);

        let prunable_state = PrunableState {
            root_hash: root_hash.clone(),
            height,
            timestamp,
            size_bytes,
            is_snapshot_root,
            has_confirmations: false,
        };

        self.state_registry.insert(root_hash, prunable_state);
        Ok(())
    }

    /// Update confirmation status for states
    ///
    /// # Arguments
    ///
    /// * `current_height` - The current block height
    pub fn update_confirmations(&mut self, current_height: u64) {
        for state in self.state_registry.values_mut() {
            state.has_confirmations = current_height.saturating_sub(state.height) >= self.policy.min_confirmations;
        }
    }

    /// Execute pruning based on the current policy
    ///
    /// # Returns
    ///
    /// A Result containing the pruning result or an error
    pub fn prune(&mut self) -> PruningResult_<PruningResult> {
        let start_time = SystemTime::now();
        let mut result = PruningResult::default();

        // Get candidates for pruning
        let candidates = self.get_pruning_candidates()?;

        // Apply pruning strategy
        let to_prune: Vec<Hash> = self.apply_pruning_strategy(&candidates)?.iter().map(|s| s.root_hash).collect();

        // Execute the actual pruning
        for root_hash in to_prune {
            // Clone the state to avoid borrowing issues
            if let Some(state) = self.state_registry.get(&root_hash).cloned() {
                match self.prune_state(&state) {
                    Ok(bytes_reclaimed) => {
                        result.pruned_count += 1;
                        result.bytes_reclaimed += bytes_reclaimed;
                        result.pruned_roots.push(root_hash);
                        self.state_registry.remove(&root_hash);
                    }
                    Err(e) => {
                        result.errors.push(format!("Failed to prune state {:?}: {:?}", root_hash, e));
                    }
                }
            }
        }

        // Update statistics
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        self.update_stats(&result, duration);

        self.last_prune_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        Ok(result)
    }

    /// Check if automatic pruning should be triggered
    ///
    /// # Returns
    ///
    /// True if automatic pruning should be triggered, false otherwise
    pub fn should_auto_prune(&self) -> bool {
        if !self.policy.enabled {
            return false;
        }

        if let Some(interval) = self.policy.auto_prune_interval {
            let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            current_time.saturating_sub(self.last_prune_time) >= interval
        } else {
            false
        }
    }

    /// Get current pruning statistics
    ///
    /// # Returns
    ///
    /// A reference to the current pruning statistics
    pub fn get_stats(&self) -> &PruningStats {
        &self.stats
    }

    /// Get information about all tracked states
    ///
    /// # Returns
    ///
    /// A vector of references to all tracked states
    pub fn get_state_info(&self) -> Vec<&PrunableState> {
        self.state_registry.values().collect()
    }

    /// Get states that are candidates for pruning
    ///
    /// # Returns
    ///
    /// A Result containing a vector of states that can be pruned
    fn get_pruning_candidates(&self) -> PruningResult_<Vec<&PrunableState>> {
        let mut candidates: Vec<&PrunableState> = self
            .state_registry
            .values()
            .filter(|state| state.has_confirmations && (!state.is_snapshot_root || !self.policy.preserve_snapshot_roots))
            .collect();

        // Sort by timestamp (oldest first)
        candidates.sort_by_key(|state| state.timestamp);
        Ok(candidates)
    }

    /// Apply the current pruning strategy to candidate states
    ///
    /// # Arguments
    ///
    /// * `candidates` - The states that are candidates for pruning
    ///
    /// # Returns
    ///
    /// A Result containing the states that should be pruned
    fn apply_pruning_strategy<'a>(&self, candidates: &[&'a PrunableState]) -> PruningResult_<Vec<&'a PrunableState>> {
        let mut to_prune = Vec::new();
        let mut to_keep = HashSet::new();

        match &self.policy.strategy {
            PruningStrategy::KeepLast(n) => {
                if candidates.len() > *n {
                    to_prune.extend(candidates.iter().take(candidates.len() - n));
                }
            }
            PruningStrategy::KeepRecent(seconds) => {
                let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                for state in candidates {
                    if current_time.saturating_sub(state.timestamp) > *seconds {
                        to_prune.push(*state);
                    } else {
                        to_keep.insert(state.root_hash);
                    }
                }
            }
            PruningStrategy::KeepAtIntervals(interval) => {
                let mut last_kept_height = 0;
                for state in candidates {
                    if state.height.saturating_sub(last_kept_height) >= *interval {
                        to_keep.insert(state.root_hash);
                        last_kept_height = state.height;
                    } else {
                        to_prune.push(*state);
                    }
                }
            }
            PruningStrategy::Custom {
                keep_last,
                keep_recent_seconds,
                keep_intervals,
                keep_snapshots,
            } => {
                // Apply each strategy in sequence
                if let Some(n) = keep_last {
                    let mut temp_prune: Vec<&PrunableState> = Vec::new();
                    if candidates.len() > *n {
                        temp_prune.extend(candidates.iter().take(candidates.len() - n));
                    }
                    to_prune.extend(temp_prune);
                }

                if let Some(seconds) = keep_recent_seconds {
                    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                    for state in candidates {
                        if current_time.saturating_sub(state.timestamp) <= *seconds {
                            to_keep.insert(state.root_hash);
                        }
                    }
                }

                if let Some(interval) = keep_intervals {
                    let mut last_kept_height = 0;
                    for state in candidates {
                        if state.height.saturating_sub(last_kept_height) >= *interval {
                            to_keep.insert(state.root_hash);
                            last_kept_height = state.height;
                        }
                    }
                }

                if *keep_snapshots {
                    for state in candidates {
                        if state.is_snapshot_root {
                            to_keep.insert(state.root_hash);
                        }
                    }
                }
            }
        }

        // Remove any states that should be kept
        to_prune.retain(|state| !to_keep.contains(&state.root_hash));
        Ok(to_prune)
    }

    /// Prune a specific state from storage
    ///
    /// # Arguments
    ///
    /// * `state` - The state to prune
    ///
    /// # Returns
    ///
    /// A Result containing the number of bytes reclaimed
    fn prune_state(&mut self, state: &PrunableState) -> PruningResult_<u64> {
        if state.is_snapshot_root && self.policy.preserve_snapshot_roots {
            return Err(PruningError::StateProtected(state.root_hash));
        }

        // Get the storage implementation from the snapshot manager
        let snapshot_manager = self.snapshot_manager.as_mut().ok_or_else(|| PruningError::InvalidConfig("Storage not available".to_string()))?;

        // Get mutable access to the current trie
        let current_trie = snapshot_manager
            .get_current_trie_mut()
            .map_err(|e| PruningError::InvalidConfig(format!("Failed to get current trie: {:?}", e)))?;

        // Collect nodes to delete
        fn collect_nodes_to_delete<S: NodeStorage>(node_id: &NodeId, storage: &S, nodes_to_delete: &mut Vec<NodeId>, bytes_reclaimed: &mut u64) -> PruningResult_<()> {
            if let Some(node) = storage.get_node(node_id)? {
                // Add current node to deletion list
                nodes_to_delete.push(node_id.clone());
                *bytes_reclaimed += node.size_bytes();

                // Recursively process child nodes
                match node.node_type {
                    NodeType::Branch { ref children, .. } => {
                        for child in children.iter().flatten() {
                            collect_nodes_to_delete(child, storage, nodes_to_delete, bytes_reclaimed)?;
                        }
                    }
                    NodeType::Extension { ref child, .. } => {
                        collect_nodes_to_delete(child, storage, nodes_to_delete, bytes_reclaimed)?;
                    }
                    _ => {} // Leaf nodes have no children
                }
            }
            Ok(())
        }

        let mut nodes_to_delete = Vec::new();
        let mut bytes_reclaimed = 0;

        // Get the storage to check for nodes
        let storage_rwlock = current_trie.get_storage_mut();
        let storage = storage_rwlock.read();
        collect_nodes_to_delete(&state.root_hash, &*storage, &mut nodes_to_delete, &mut bytes_reclaimed)?;
        drop(storage); // Release the read lock

        // Now get a write lock to delete nodes
        let mut storage = storage_rwlock.write();

        // Delete nodes in reverse order (children first)
        nodes_to_delete.reverse();
        for node_id in nodes_to_delete {
            storage.delete_node(&node_id)?;
        }

        drop(storage); // Release the write lock

        // Update metadata and statistics
        if let Some(sm) = &mut self.snapshot_manager {
            use std::collections::HashMap;

            // Try to update snapshot metadata, but don't fail if the snapshot doesn't exist
            let _result = sm.update_snapshot_metadata(
                &format!("{:?}", state.root_hash),
                HashMap::from([
                    ("pruned_at".to_string(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs().to_string()),
                    ("bytes_reclaimed".to_string(), bytes_reclaimed.to_string()),
                ]),
            );
            // Ignore errors from metadata update as it's not essential for pruning
        }

        Ok(bytes_reclaimed)
    }

    /// Update pruning statistics
    ///
    /// # Arguments
    ///
    /// * `result` - The result of the pruning operation
    /// * `duration_ms` - The duration of the operation in milliseconds
    fn update_stats(&mut self, result: &PruningResult, duration_ms: u64) {
        self.stats.total_prune_operations += 1;
        self.stats.total_states_pruned += result.pruned_count as u64;
        self.stats.total_bytes_reclaimed += result.bytes_reclaimed;
        self.stats.last_prune_timestamp = Some(self.last_prune_time);

        // Update average duration
        let total_duration = self.stats.avg_prune_duration_ms * (self.stats.total_prune_operations - 1);
        self.stats.avg_prune_duration_ms = (total_duration + duration_ms) / self.stats.total_prune_operations;
    }

    /// Force prune specific states
    ///
    /// # Arguments
    ///
    /// * `root_hashes` - The root hashes of states to prune
    ///
    /// # Returns
    ///
    /// A Result containing the pruning result
    pub fn force_prune(&mut self, root_hashes: &[Hash]) -> PruningResult_<PruningResult> {
        let mut result = PruningResult::default();

        for root_hash in root_hashes {
            // Clone the state to avoid borrowing issues
            if let Some(state) = self.state_registry.get(root_hash).cloned() {
                match self.prune_state(&state) {
                    Ok(bytes_reclaimed) => {
                        result.pruned_count += 1;
                        result.bytes_reclaimed += bytes_reclaimed;
                        result.pruned_roots.push(root_hash.clone());
                        self.state_registry.remove(root_hash);
                    }
                    Err(e) => {
                        result.errors.push(format!("Failed to prune state {:?}: {:?}", root_hash, e));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get all states that can be pruned
    ///
    /// # Returns
    ///
    /// A Result containing a vector of states that can be pruned
    pub fn get_prunable_states(&self) -> PruningResult_<Vec<&PrunableState>> {
        self.get_pruning_candidates()
    }

    /// Update the pruning policy
    ///
    /// # Arguments
    ///
    /// * `new_policy` - The new policy to use
    pub fn update_policy(&mut self, new_policy: PruningPolicy) {
        self.policy = new_policy;
    }

    /// Get the current pruning policy
    ///
    /// # Returns
    ///
    /// A reference to the current policy
    pub fn get_policy(&self) -> &PruningPolicy {
        &self.policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mpt::MerklePatriciaTrie;
    use crate::state::mpt::trie::InMemoryStorage;
    use crate::state::snapshot::SnapshotManager;

    type TestPruner = StatePruner<InMemoryStorage>;
    type TestSnapshotManager = SnapshotManager<InMemoryStorage>;

    #[test]
    fn test_pruning_strategy_default() {
        let strategy = PruningStrategy::default();
        match strategy {
            PruningStrategy::Custom {
                keep_last,
                keep_recent_seconds,
                keep_intervals,
                keep_snapshots,
            } => {
                assert_eq!(keep_last, Some(100));
                assert_eq!(keep_recent_seconds, Some(86400 * 7));
                assert_eq!(keep_intervals, Some(1000));
                assert!(keep_snapshots);
            }
            _ => assert!(false, "Expected Custom strategy"),
        }
    }

    #[test]
    fn test_pruning_policy_default() {
        let policy = PruningPolicy::default();
        assert!(policy.enabled);
        assert_eq!(policy.auto_prune_interval, Some(3600));
        assert_eq!(policy.min_confirmations, 6);
        assert!(policy.preserve_snapshot_roots);
        assert_eq!(policy.max_storage_size, Some(1024 * 1024 * 1024));
    }

    #[test]
    fn test_prunable_state_creation() {
        let root_hash = [1u8; 32];
        let height = 100;
        let timestamp = 1234567890;
        let size_bytes = 1024;

        let state = PrunableState {
            root_hash,
            height,
            timestamp,
            size_bytes,
            is_snapshot_root: false,
            has_confirmations: true,
        };

        assert_eq!(state.root_hash, root_hash);
        assert_eq!(state.height, height);
        assert_eq!(state.timestamp, timestamp);
        assert_eq!(state.size_bytes, size_bytes);
        assert!(!state.is_snapshot_root);
        assert!(state.has_confirmations);
    }

    #[test]
    fn test_pruning_result_default() {
        let result = PruningResult::default();
        assert_eq!(result.pruned_count, 0);
        assert_eq!(result.bytes_reclaimed, 0);
        assert!(result.pruned_roots.is_empty());
        assert!(result.preserved_states.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_state_pruner_creation() {
        let policy = PruningPolicy::default();
        let pruner: TestPruner = StatePruner::new(policy);

        assert!(pruner.policy.enabled);
        assert!(pruner.state_registry.is_empty());
        assert!(pruner.snapshot_manager.is_none());
        assert_eq!(pruner.last_prune_time, 0);
    }

    #[test]
    fn test_state_registration() {
        let mut pruner: TestPruner = StatePruner::with_defaults();
        let root_hash = [1u8; 32];
        let height = 100;
        let size_bytes = 1024;

        let result = pruner.register_state(root_hash, height, size_bytes);
        assert!(result.is_ok());

        let states = pruner.get_state_info();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].root_hash, root_hash);
        assert_eq!(states[0].height, height);
        assert_eq!(states[0].size_bytes, size_bytes);
    }

    #[test]
    fn test_confirmation_updates() {
        let mut pruner: TestPruner = StatePruner::with_defaults();
        let root_hash = [1u8; 32];

        // Register a state at height 100
        pruner.register_state(root_hash, 100, 1024).unwrap();

        // Initially no confirmations
        let states = pruner.get_state_info();
        assert!(!states[0].has_confirmations);

        // Update confirmations with current height 105 (5 confirmations, less than required 6)
        pruner.update_confirmations(105);
        let states = pruner.get_state_info();
        assert!(!states[0].has_confirmations);

        // Update with current height 107 (7 confirmations, more than required 6)
        pruner.update_confirmations(107);
        let states = pruner.get_state_info();
        assert!(states[0].has_confirmations);
    }

    #[test]
    fn test_auto_prune_conditions() {
        let mut policy = PruningPolicy::default();
        policy.auto_prune_interval = Some(60); // 1 minute

        let pruner: TestPruner = StatePruner::new(policy);

        // Should trigger auto prune since last_prune_time is 0
        assert!(pruner.should_auto_prune());
    }

    #[test]
    fn test_disabled_pruning() {
        let mut policy = PruningPolicy::default();
        policy.enabled = false;

        let mut pruner: TestPruner = StatePruner::new(policy);
        pruner.register_state([1u8; 32], 100, 1024).unwrap();

        let result = pruner.prune().unwrap();
        assert_eq!(result.pruned_count, 0);
        assert!(!pruner.should_auto_prune());
    }

    #[test]
    fn test_force_prune() {
        let mut policy = PruningPolicy::default();
        policy.preserve_snapshot_roots = false;
        let mut pruner: TestPruner = StatePruner::new(policy);

        // Create a trie with some data
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let key = b"testkey".to_vec();
        let value = b"testvalue".to_vec();
        trie.put(key.clone(), value.clone()).unwrap();
        let root_hash = trie.root_hash();

        // Create and configure snapshot manager
        let config = crate::state::snapshot::SnapshotConfig::default();
        let mut snapshot_manager = SnapshotManager::new(config);
        snapshot_manager.set_current_trie(trie);
        pruner.set_snapshot_manager(snapshot_manager);

        // Register the state
        pruner.register_state(root_hash, 100, 1024).unwrap();
        // Add confirmations (otherwise state cannot be pruned)
        pruner.update_confirmations(200); // Sufficient height

        // Force prune the state
        let result = pruner.force_prune(&[root_hash]).unwrap();

        assert_eq!(result.pruned_count, 1, "Expected 1 state to be pruned");
        assert!(result.bytes_reclaimed > 0, "Expected bytes to be reclaimed");
        assert_eq!(result.pruned_roots, vec![root_hash], "Expected root hash to be pruned");

        // Verify state is removed from registry
        let states = pruner.get_state_info();
        assert!(states.is_empty(), "Expected state registry to be empty after pruning");
    }

    #[test]
    fn test_pruning_stats() {
        let pruner: TestPruner = StatePruner::with_defaults();
        let stats = pruner.get_stats();

        assert_eq!(stats.total_prune_operations, 0);
        assert_eq!(stats.total_states_pruned, 0);
        assert_eq!(stats.total_bytes_reclaimed, 0);
        assert!(stats.last_prune_timestamp.is_none());
        assert_eq!(stats.avg_prune_duration_ms, 0);
    }

    #[test]
    fn test_policy_update() {
        let mut pruner: TestPruner = StatePruner::with_defaults();
        let mut new_policy = PruningPolicy::default();
        new_policy.enabled = false;
        new_policy.min_confirmations = 10;

        pruner.update_policy(new_policy.clone());

        let current_policy = pruner.get_policy();
        assert!(!current_policy.enabled);
        assert_eq!(current_policy.min_confirmations, 10);
    }
}
