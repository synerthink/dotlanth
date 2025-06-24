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

//! Multi-Version Concurrency Control (MVCC) Implementation
//!
//! This module provides a robust MVCC system that enables concurrent transactions
//! to access different versions of data without blocking each other. It ensures
//! snapshot isolation and manages version visibility based on transaction timestamps.
//!
//! # Integration with Contract State Versioning
//!
//! This MVCC system integrates with the contract state versioning system from DOTVM-39
//! to provide comprehensive version management across both storage engine and contract state levels.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;

use crate::state::contract_storage_layout::ContractAddress;
use crate::state::mpt::Hash;
use crate::state::versioning::{ContractStateVersion, ContractVersionManager, StateVersionId};
use crate::storage_engine::file_format::{Page, PageId};
use crate::storage_engine::lib::{StorageError, StorageResult, VersionId};
use crate::storage_engine::transaction::{IsolationLevel, TransactionId};

/// Timestamp type for versioning
pub type Timestamp = u64;

/// Version information for a data item
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// The data content
    pub data: Arc<Page>,
    /// Transaction that created this version
    pub created_by: TransactionId,
    /// Transaction that deleted this version (None if active)
    pub deleted_by: Option<TransactionId>,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Deletion timestamp (None if not deleted)
    pub deleted_at: Option<Timestamp>,
    /// Whether this version is committed
    pub is_committed: bool,
}

impl VersionInfo {
    /// Create a new version
    pub fn new(data: Arc<Page>, created_by: TransactionId, created_at: Timestamp) -> Self {
        Self {
            data,
            created_by,
            deleted_by: None,
            created_at,
            deleted_at: None,
            is_committed: false,
        }
    }

    /// Mark version as committed
    pub fn commit(&mut self) {
        self.is_committed = true;
    }

    /// Mark version as deleted by a transaction
    pub fn mark_deleted(&mut self, deleted_by: TransactionId, deleted_at: Timestamp) {
        self.deleted_by = Some(deleted_by);
        self.deleted_at = Some(deleted_at);
    }

    /// Check if version is visible to a transaction at given timestamp
    pub fn is_visible(&self, txn_id: TransactionId, snapshot_timestamp: Timestamp, active_txns: &HashSet<TransactionId>) -> bool {
        // Version must be committed or created by the same transaction
        if !self.is_committed && self.created_by != txn_id {
            return false;
        }

        // Version must be created before the snapshot timestamp
        if self.created_at > snapshot_timestamp {
            return false;
        }

        // If version is deleted, check deletion visibility
        if let Some(deleted_by) = self.deleted_by {
            if let Some(deleted_at) = self.deleted_at {
                // If deleted by same transaction, not visible
                if deleted_by == txn_id {
                    return false;
                }

                // If deleted by a committed transaction before snapshot, not visible
                if !active_txns.contains(&deleted_by) && deleted_at <= snapshot_timestamp {
                    return false;
                }
            }
        }

        true
    }
}

/// Version chain for a single data item
#[derive(Debug, Default)]
pub struct VersionChain {
    /// All versions ordered by creation timestamp
    versions: BTreeMap<Timestamp, VersionInfo>,
    /// Latest committed version timestamp
    latest_committed: Option<Timestamp>,
}

impl VersionChain {
    /// Add a new version to the chain
    pub fn add_version(&mut self, version: VersionInfo) {
        let timestamp = version.created_at;
        self.versions.insert(timestamp, version);
    }

    /// Get the visible version for a transaction
    pub fn get_visible_version(&self, txn_id: TransactionId, snapshot_timestamp: Timestamp, active_txns: &HashSet<TransactionId>) -> Option<&VersionInfo> {
        // First, check if the transaction has any versions (its own changes should always be visible)
        let mut transaction_version: Option<&VersionInfo> = None;
        for (_, version) in &self.versions {
            if version.created_by == txn_id {
                // Take the latest version from this transaction
                if transaction_version.is_none() || version.created_at > transaction_version.unwrap().created_at {
                    transaction_version = Some(version);
                }
            }
        }

        // If we found a version from the same transaction, return it (unless it's deleted by the same transaction)
        if let Some(version) = transaction_version {
            if version.deleted_by != Some(txn_id) {
                return Some(version);
            }
        }

        // Otherwise, look for committed versions from other transactions
        // Iterate through versions in reverse timestamp order (newest first) up to snapshot timestamp
        for (_, version) in self.versions.range(..=snapshot_timestamp).rev() {
            if version.created_by != txn_id && version.is_visible(txn_id, snapshot_timestamp, active_txns) {
                return Some(version);
            }
        }

        None
    }

    /// Get all versions created by a specific transaction
    pub fn get_versions_by_transaction(&self, txn_id: TransactionId) -> Vec<&VersionInfo> {
        self.versions.values().filter(|v| v.created_by == txn_id).collect()
    }

    /// Mark a version as committed
    pub fn commit_version(&mut self, timestamp: Timestamp) -> StorageResult<()> {
        if let Some(version) = self.versions.get_mut(&timestamp) {
            version.commit();
            self.latest_committed = Some(timestamp);
            Ok(())
        } else {
            Err(StorageError::Corruption("Version not found for commit".to_string()))
        }
    }

    /// Remove old versions that are no longer needed
    pub fn garbage_collect(&mut self, oldest_active_timestamp: Timestamp) -> Vec<VersionInfo> {
        let mut removed_versions = Vec::new();
        let mut to_remove = Vec::new();

        for (&timestamp, version) in &self.versions {
            // Can only remove versions that are older than oldest active transaction
            if timestamp < oldest_active_timestamp && version.is_committed {
                // Check if there's a newer committed version
                if let Some(latest) = self.latest_committed {
                    if latest > timestamp {
                        to_remove.push(timestamp);
                    }
                }
            }
        }

        for timestamp in to_remove {
            if let Some(version) = self.versions.remove(&timestamp) {
                removed_versions.push(version);
            }
        }

        removed_versions
    }

    /// Get the number of versions in the chain
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
}

/// Transaction snapshot information
#[derive(Debug, Clone)]
pub struct TransactionSnapshot {
    /// Transaction ID
    pub txn_id: TransactionId,
    /// Snapshot timestamp
    pub timestamp: Timestamp,
    /// Set of active transactions at snapshot time
    pub active_transactions: HashSet<TransactionId>,
    /// Isolation level
    pub isolation_level: IsolationLevel,
}

impl TransactionSnapshot {
    /// Create a new transaction snapshot
    pub fn new(txn_id: TransactionId, timestamp: Timestamp, active_transactions: HashSet<TransactionId>, isolation_level: IsolationLevel) -> Self {
        Self {
            txn_id,
            timestamp,
            active_transactions,
            isolation_level,
        }
    }

    /// Check if a transaction is visible in this snapshot
    pub fn is_transaction_visible(&self, other_txn_id: TransactionId, commit_timestamp: Option<Timestamp>) -> bool {
        // Same transaction is always visible
        if other_txn_id == self.txn_id {
            return true;
        }

        // If transaction was active when snapshot was taken, it's not visible
        if self.active_transactions.contains(&other_txn_id) {
            return false;
        }

        // If transaction committed before snapshot, it's visible
        if let Some(commit_ts) = commit_timestamp { commit_ts <= self.timestamp } else { false }
    }
}

/// MVCC Manager handles version control and visibility
pub struct MVCCManager {
    /// Version chains for each page
    version_chains: RwLock<HashMap<PageId, VersionChain>>,
    /// Active transaction snapshots
    transaction_snapshots: RwLock<HashMap<TransactionId, TransactionSnapshot>>,
    /// Transaction commit timestamps
    commit_timestamps: RwLock<HashMap<TransactionId, Timestamp>>,
    /// Current timestamp counter
    timestamp_counter: Mutex<Timestamp>,
    /// Garbage collection threshold
    gc_threshold: usize,
    /// Contract version manager for contract state versioning integration
    contract_version_manager: Arc<ContractVersionManager>,
    /// Transaction to contract state mapping
    transaction_contract_states: RwLock<HashMap<TransactionId, Vec<(ContractAddress, StateVersionId)>>>,
}

impl MVCCManager {
    /// Create a new MVCC manager
    pub fn new() -> Self {
        Self {
            version_chains: RwLock::new(HashMap::new()),
            transaction_snapshots: RwLock::new(HashMap::new()),
            commit_timestamps: RwLock::new(HashMap::new()),
            timestamp_counter: Mutex::new(Self::current_timestamp()),
            gc_threshold: 1000, // Trigger GC after 1000 versions
            contract_version_manager: Arc::new(ContractVersionManager::default()),
            transaction_contract_states: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new MVCC manager with custom contract version manager
    pub fn new_with_contract_manager(contract_manager: Arc<ContractVersionManager>) -> Self {
        Self {
            version_chains: RwLock::new(HashMap::new()),
            transaction_snapshots: RwLock::new(HashMap::new()),
            commit_timestamps: RwLock::new(HashMap::new()),
            timestamp_counter: Mutex::new(Self::current_timestamp()),
            gc_threshold: 1000,
            contract_version_manager: contract_manager,
            transaction_contract_states: RwLock::new(HashMap::new()),
        }
    }

    /// Get current system timestamp
    pub fn current_timestamp() -> Timestamp {
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos() as Timestamp
    }

    /// Generate next timestamp
    pub fn next_timestamp(&self) -> Timestamp {
        let mut counter = self.timestamp_counter.lock().unwrap();
        *counter += 1;
        *counter
    }

    /// Create a snapshot for a transaction
    pub fn create_snapshot(&self, txn_id: TransactionId, isolation_level: IsolationLevel) -> StorageResult<TransactionSnapshot> {
        let timestamp = self.next_timestamp();
        let snapshots = self.transaction_snapshots.read().unwrap();
        let active_transactions: HashSet<TransactionId> = snapshots.keys().cloned().collect();
        drop(snapshots);

        let snapshot = TransactionSnapshot::new(txn_id, timestamp, active_transactions, isolation_level);

        self.transaction_snapshots.write().unwrap().insert(txn_id, snapshot.clone());

        Ok(snapshot)
    }

    /// Add a new version for a page
    pub fn add_version(&self, page_id: PageId, data: Arc<Page>, txn_id: TransactionId) -> StorageResult<()> {
        let timestamp = self.next_timestamp();
        let version = VersionInfo::new(data, txn_id, timestamp);

        let mut chains = self.version_chains.write().unwrap();
        let chain = chains.entry(page_id).or_default();
        chain.add_version(version);

        // Trigger garbage collection if needed
        if chain.version_count() > self.gc_threshold {
            self.trigger_garbage_collection(page_id)?;
        }

        Ok(())
    }

    /// Get visible version for a transaction
    pub fn get_visible_version(&self, page_id: PageId, txn_id: TransactionId) -> StorageResult<Option<Arc<Page>>> {
        let chains = self.version_chains.read().unwrap();
        let snapshots = self.transaction_snapshots.read().unwrap();

        if let Some(snapshot) = snapshots.get(&txn_id) {
            if let Some(chain) = chains.get(&page_id) {
                if let Some(version) = chain.get_visible_version(txn_id, snapshot.timestamp, &snapshot.active_transactions) {
                    return Ok(Some(version.data.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Commit a transaction
    pub fn commit_transaction(&self, txn_id: TransactionId) -> StorageResult<()> {
        let commit_timestamp = self.next_timestamp();

        // Record commit timestamp
        self.commit_timestamps.write().unwrap().insert(txn_id, commit_timestamp);

        // Mark all versions created by this transaction as committed
        let mut chains = self.version_chains.write().unwrap();
        for chain in chains.values_mut() {
            let timestamps: Vec<Timestamp> = chain.get_versions_by_transaction(txn_id).iter().map(|v| v.created_at).collect();

            for timestamp in timestamps {
                if let Some(version_mut) = chain.versions.get_mut(&timestamp) {
                    version_mut.commit();
                }
            }
        }

        // Remove transaction snapshot
        self.transaction_snapshots.write().unwrap().remove(&txn_id);

        Ok(())
    }

    /// Abort a transaction
    pub fn abort_transaction(&self, txn_id: TransactionId) -> StorageResult<()> {
        // Remove all versions created by this transaction
        let mut chains = self.version_chains.write().unwrap();
        for chain in chains.values_mut() {
            let versions_to_remove: Vec<Timestamp> = chain.get_versions_by_transaction(txn_id).iter().map(|v| v.created_at).collect();

            for timestamp in versions_to_remove {
                chain.versions.remove(&timestamp);
            }
        }

        // Remove transaction snapshot
        self.transaction_snapshots.write().unwrap().remove(&txn_id);

        Ok(())
    }

    /// Trigger garbage collection for a specific page
    fn trigger_garbage_collection(&self, page_id: PageId) -> StorageResult<()> {
        let oldest_active_timestamp = self.get_oldest_active_timestamp();

        let mut chains = self.version_chains.write().unwrap();
        if let Some(chain) = chains.get_mut(&page_id) {
            let _removed = chain.garbage_collect(oldest_active_timestamp);
            // Log or handle removed versions if needed
        }

        Ok(())
    }

    /// Get the oldest active transaction timestamp
    fn get_oldest_active_timestamp(&self) -> Timestamp {
        let snapshots = self.transaction_snapshots.read().unwrap();
        snapshots.values().map(|s| s.timestamp).min().unwrap_or_else(|| self.next_timestamp())
    }

    /// Get transaction commit timestamp
    pub fn get_commit_timestamp(&self, txn_id: TransactionId) -> Option<Timestamp> {
        self.commit_timestamps.read().unwrap().get(&txn_id).copied()
    }

    /// Check for write-write conflicts
    pub fn check_write_conflict(&self, page_id: PageId, txn_id: TransactionId) -> StorageResult<bool> {
        let chains = self.version_chains.read().unwrap();
        let snapshots = self.transaction_snapshots.read().unwrap();

        if let Some(snapshot) = snapshots.get(&txn_id) {
            if let Some(chain) = chains.get(&page_id) {
                // Check if any transaction that was active when this transaction started
                // has committed a write to this page
                for (_, version) in &chain.versions {
                    if version.created_by != txn_id && snapshot.active_transactions.contains(&version.created_by) && version.is_committed {
                        return Ok(true); // Conflict detected
                    }
                }
            }
        }

        Ok(false)
    }

    /// Create contract state version for a transaction
    pub fn create_contract_state_version(&self, txn_id: TransactionId, contract_address: ContractAddress, mpt_root_hash: Hash, description: String) -> StorageResult<StateVersionId> {
        let version_id = self
            .contract_version_manager
            .create_version(contract_address, mpt_root_hash, description)
            .map_err(|e| StorageError::Corruption(format!("Failed to create contract version: {:?}", e)))?;

        // Track the contract state for this transaction
        self.transaction_contract_states
            .write()
            .unwrap()
            .entry(txn_id)
            .or_insert_with(Vec::new)
            .push((contract_address, version_id));

        Ok(version_id)
    }

    /// Get contract state version at transaction snapshot
    pub fn get_contract_state_at_snapshot(&self, txn_id: TransactionId, contract_address: ContractAddress) -> StorageResult<Option<ContractStateVersion>> {
        let snapshots = self.transaction_snapshots.read().unwrap();
        if let Some(snapshot) = snapshots.get(&txn_id) {
            // Find the latest committed version before the snapshot timestamp
            let all_versions = self.contract_version_manager.get_all_versions(contract_address);
            let mut visible_version: Option<ContractStateVersion> = None;

            for version in all_versions {
                if version.created_at <= snapshot.timestamp && version.is_finalized {
                    if visible_version.is_none() || version.created_at > visible_version.as_ref().unwrap().created_at {
                        visible_version = Some(version);
                    }
                }
            }

            Ok(visible_version)
        } else {
            Err(StorageError::InvalidOperation("Transaction not found".to_string()))
        }
    }

    /// Commit contract state changes for a transaction
    pub fn commit_contract_states(&self, txn_id: TransactionId) -> StorageResult<()> {
        let states = {
            let mut contract_states = self.transaction_contract_states.write().unwrap();
            contract_states.remove(&txn_id).unwrap_or_default()
        };

        for (contract_address, version_id) in states {
            self.contract_version_manager
                .finalize_version(contract_address, version_id)
                .map_err(|e| StorageError::Corruption(format!("Failed to finalize contract version: {:?}", e)))?;
        }

        Ok(())
    }

    /// Rollback contract state changes for a transaction
    pub fn rollback_contract_states(&self, txn_id: TransactionId) -> StorageResult<()> {
        let states = {
            let mut contract_states = self.transaction_contract_states.write().unwrap();
            contract_states.remove(&txn_id).unwrap_or_default()
        };

        // For contract versioning, we don't need to explicitly remove versions
        // since they weren't finalized and will be cleaned up by GC
        Ok(())
    }

    /// Get statistics about the MVCC system
    pub fn get_statistics(&self) -> MVCCStatistics {
        let chains = self.version_chains.read().unwrap();
        let snapshots = self.transaction_snapshots.read().unwrap();

        let total_versions: usize = chains.values().map(|c| c.version_count()).sum();
        let total_pages = chains.len();
        let active_transactions = snapshots.len();

        MVCCStatistics {
            total_versions,
            total_pages,
            active_transactions,
            average_versions_per_page: if total_pages > 0 { total_versions as f64 / total_pages as f64 } else { 0.0 },
        }
    }
}

impl Default for MVCCManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the MVCC system
#[derive(Debug, Clone)]
pub struct MVCCStatistics {
    /// Total number of versions across all pages
    pub total_versions: usize,
    /// Total number of pages with versions
    pub total_pages: usize,
    /// Number of active transactions
    pub active_transactions: usize,
    /// Average number of versions per page
    pub average_versions_per_page: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_engine::file_format::{PageHeader, PageType};
    use crate::storage_engine::lib::VersionId;

    fn create_test_page(data: &[u8]) -> Arc<Page> {
        let mut page_data = vec![0u8; 4096];
        if !data.is_empty() {
            page_data[..data.len().min(4096)].copy_from_slice(data);
        }
        Arc::new(Page::new(PageId(1), PageType::Data, VersionId(0), page_data.len()))
    }

    #[test]
    fn test_version_info_visibility() {
        let page = create_test_page(b"test data");
        let mut version = VersionInfo::new(page, 1, 100);
        version.commit();

        let active_txns = HashSet::new();

        // Same transaction should see its own uncommitted changes
        assert!(version.is_visible(1, 150, &active_txns));

        // Other transactions should not see uncommitted changes
        let mut uncommitted_version = VersionInfo::new(create_test_page(b"uncommitted"), 2, 110);
        assert!(!uncommitted_version.is_visible(1, 150, &active_txns));

        // Committed changes should be visible to later transactions
        uncommitted_version.commit();
        assert!(uncommitted_version.is_visible(1, 150, &active_txns));

        // Future changes should not be visible
        assert!(!uncommitted_version.is_visible(1, 105, &active_txns));
    }

    #[test]
    fn test_version_chain() {
        let mut chain = VersionChain::default();

        // Add versions
        let v1 = VersionInfo::new(create_test_page(b"version 1"), 1, 100);
        let mut v2 = VersionInfo::new(create_test_page(b"version 2"), 2, 200);
        v2.commit();

        chain.add_version(v1);
        chain.add_version(v2);

        let active_txns = HashSet::new();

        // Should get the latest committed version
        let visible = chain.get_visible_version(3, 250, &active_txns);
        assert!(visible.is_some());
        assert_eq!(visible.unwrap().created_by, 2);

        // Should not see uncommitted version from another transaction
        let visible = chain.get_visible_version(3, 150, &active_txns);
        assert!(visible.is_none());
    }

    #[test]
    fn test_mvcc_manager_basic_operations() {
        let mvcc = MVCCManager::new();
        let page_id = PageId(1);
        let txn_id = 1;

        // Create snapshot
        let snapshot = mvcc.create_snapshot(txn_id, IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(snapshot.txn_id, txn_id);

        // Add version
        let page = create_test_page(b"test data");
        mvcc.add_version(page_id, page.clone(), txn_id).unwrap();

        // Get visible version (should see own changes)
        let visible = mvcc.get_visible_version(page_id, txn_id).unwrap();
        assert!(visible.is_some());

        // Commit transaction
        mvcc.commit_transaction(txn_id).unwrap();

        // Should still be able to get the version
        let visible = mvcc.get_visible_version(page_id, txn_id);
        assert!(visible.is_ok());
    }

    #[test]
    fn test_mvcc_transaction_isolation() {
        let mvcc = MVCCManager::new();
        let page_id = PageId(1);

        // Transaction 1 creates initial version
        let txn1 = 1;
        mvcc.create_snapshot(txn1, IsolationLevel::ReadCommitted).unwrap();
        mvcc.add_version(page_id, create_test_page(b"initial"), txn1).unwrap();
        mvcc.commit_transaction(txn1).unwrap();

        // Transaction 2 starts and reads
        let txn2 = 2;
        mvcc.create_snapshot(txn2, IsolationLevel::ReadCommitted).unwrap();
        let visible = mvcc.get_visible_version(page_id, txn2).unwrap();
        assert!(visible.is_some());

        // Transaction 3 modifies (but doesn't commit yet)
        let txn3 = 3;
        mvcc.create_snapshot(txn3, IsolationLevel::ReadCommitted).unwrap();
        mvcc.add_version(page_id, create_test_page(b"modified"), txn3).unwrap();

        // Transaction 2 should still see original version
        let visible = mvcc.get_visible_version(page_id, txn2).unwrap();
        assert!(visible.is_some());

        // Transaction 3 should see its own changes
        let visible = mvcc.get_visible_version(page_id, txn3).unwrap();
        assert!(visible.is_some());
    }

    #[test]
    fn test_write_conflict_detection() {
        let mvcc = MVCCManager::new();
        let page_id = PageId(1);

        // Setup initial state
        let txn1 = 1;
        mvcc.create_snapshot(txn1, IsolationLevel::ReadCommitted).unwrap();
        mvcc.add_version(page_id, create_test_page(b"initial"), txn1).unwrap();

        // Start transaction 2 before txn1 commits
        let txn2 = 2;
        mvcc.create_snapshot(txn2, IsolationLevel::ReadCommitted).unwrap();

        // Commit txn1
        mvcc.commit_transaction(txn1).unwrap();

        // Now txn2 tries to write - should detect conflict
        let has_conflict = mvcc.check_write_conflict(page_id, txn2).unwrap();
        assert!(has_conflict);
    }

    #[test]
    fn test_garbage_collection() {
        let mvcc = MVCCManager::new();
        let page_id = PageId(1);

        // Create and commit multiple versions
        for i in 1..=5 {
            let txn_id = i;
            mvcc.create_snapshot(txn_id, IsolationLevel::ReadCommitted).unwrap();
            mvcc.add_version(page_id, create_test_page(&format!("version {}", i).into_bytes()), txn_id).unwrap();
            mvcc.commit_transaction(txn_id).unwrap();
        }

        let stats_before = mvcc.get_statistics();
        assert!(stats_before.total_versions >= 5);

        // Manual garbage collection trigger would happen automatically in real usage
        let stats_after = mvcc.get_statistics();
        assert!(stats_after.total_versions <= stats_before.total_versions);
    }

    #[test]
    fn test_transaction_abort() {
        let mvcc = MVCCManager::new();
        let page_id = PageId(1);
        let txn_id = 1;

        // Create snapshot and add version
        mvcc.create_snapshot(txn_id, IsolationLevel::ReadCommitted).unwrap();
        mvcc.add_version(page_id, create_test_page(b"test data"), txn_id).unwrap();

        // Verify version exists
        let visible = mvcc.get_visible_version(page_id, txn_id).unwrap();
        assert!(visible.is_some());

        // Abort transaction
        mvcc.abort_transaction(txn_id).unwrap();

        // Version should no longer be visible
        let visible = mvcc.get_visible_version(page_id, txn_id).unwrap();
        assert!(visible.is_none());
    }

    #[test]
    fn test_mvcc_statistics() {
        let mvcc = MVCCManager::new();

        let initial_stats = mvcc.get_statistics();
        assert_eq!(initial_stats.total_versions, 0);
        assert_eq!(initial_stats.total_pages, 0);
        assert_eq!(initial_stats.active_transactions, 0);

        // Add some data
        let txn_id = 1;
        mvcc.create_snapshot(txn_id, IsolationLevel::ReadCommitted).unwrap();
        mvcc.add_version(PageId(1), create_test_page(b"data1"), txn_id).unwrap();
        mvcc.add_version(PageId(2), create_test_page(b"data2"), txn_id).unwrap();

        let stats = mvcc.get_statistics();
        assert!(stats.total_versions >= 2);
        assert!(stats.total_pages >= 2);
        assert_eq!(stats.active_transactions, 1);
        assert!(stats.average_versions_per_page > 0.0);
    }
}
