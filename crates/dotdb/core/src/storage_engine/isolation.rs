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

//! Transaction Isolation Levels Implementation
//!
//! This module implements the four standard SQL transaction isolation levels:
//! Read Uncommitted, Read Committed, Repeatable Read, and Serializable.
//! Each level provides different guarantees about concurrent transaction behavior.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::storage_engine::file_format::PageId;
use crate::storage_engine::lib::StorageResult;
use crate::storage_engine::mvcc::{MVCCManager, Timestamp};
use crate::storage_engine::transaction::{IsolationLevel, TransactionId};

/// Lock type for concurrency control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Shared lock for reading
    Shared,
    /// Exclusive lock for writing
    Exclusive,
}

/// Lock request information
#[derive(Debug, Clone)]
pub struct LockRequest {
    /// Transaction requesting the lock
    pub transaction_id: TransactionId,
    /// Type of lock requested
    pub lock_type: LockType,
    /// Page being locked
    pub page_id: PageId,
    /// Timestamp when lock was requested
    pub requested_at: Timestamp,
}

/// Lock grant information
#[derive(Debug, Clone)]
pub struct LockGrant {
    /// Transaction holding the lock
    pub transaction_id: TransactionId,
    /// Type of lock held
    pub lock_type: LockType,
    /// Timestamp when lock was granted
    pub granted_at: Timestamp,
}

/// Lock manager for concurrency control
pub struct LockManager {
    /// Granted locks per page
    granted_locks: RwLock<HashMap<PageId, Vec<LockGrant>>>,
    /// Waiting lock requests
    waiting_requests: RwLock<HashMap<PageId, Vec<LockRequest>>>,
    /// Locks held by each transaction
    transaction_locks: RwLock<HashMap<TransactionId, HashSet<PageId>>>,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new() -> Self {
        Self {
            granted_locks: RwLock::new(HashMap::new()),
            waiting_requests: RwLock::new(HashMap::new()),
            transaction_locks: RwLock::new(HashMap::new()),
        }
    }

    /// Request a lock on a page
    pub fn request_lock(&self, txn_id: TransactionId, page_id: PageId, lock_type: LockType) -> StorageResult<bool> {
        let mut granted_locks = self.granted_locks.write().unwrap();
        let mut waiting_requests = self.waiting_requests.write().unwrap();
        let mut transaction_locks = self.transaction_locks.write().unwrap();

        let current_locks = granted_locks.get(&page_id).cloned().unwrap_or_default();

        // Check if lock can be granted immediately
        if self.can_grant_lock(&current_locks, txn_id, lock_type) {
            // Grant the lock
            let grant = LockGrant {
                transaction_id: txn_id,
                lock_type,
                granted_at: crate::storage_engine::mvcc::MVCCManager::current_timestamp(),
            };

            granted_locks.entry(page_id).or_default().push(grant);
            transaction_locks.entry(txn_id).or_default().insert(page_id);

            Ok(true)
        } else {
            // Add to waiting queue
            let request = LockRequest {
                transaction_id: txn_id,
                lock_type,
                page_id,
                requested_at: crate::storage_engine::mvcc::MVCCManager::current_timestamp(),
            };

            waiting_requests.entry(page_id).or_default().push(request);
            Ok(false)
        }
    }

    /// Check if a lock can be granted
    fn can_grant_lock(&self, current_locks: &[LockGrant], txn_id: TransactionId, lock_type: LockType) -> bool {
        // If no current locks, can always grant
        if current_locks.is_empty() {
            return true;
        }

        // If transaction already holds a lock on this page
        if current_locks.iter().any(|lock| lock.transaction_id == txn_id) {
            // Can upgrade from shared to exclusive if we're the only holder
            if lock_type == LockType::Exclusive {
                return current_locks.len() == 1 && current_locks[0].transaction_id == txn_id;
            }
            return true;
        }

        match lock_type {
            LockType::Shared => {
                // Can grant shared lock if all current locks are shared
                current_locks.iter().all(|lock| lock.lock_type == LockType::Shared)
            }
            LockType::Exclusive => {
                // Cannot grant exclusive lock if any other locks exist
                false
            }
        }
    }

    /// Release all locks held by a transaction
    pub fn release_transaction_locks(&self, txn_id: TransactionId) -> StorageResult<()> {
        let mut granted_locks = self.granted_locks.write().unwrap();
        let mut waiting_requests = self.waiting_requests.write().unwrap();
        let mut transaction_locks = self.transaction_locks.write().unwrap();

        // Get pages locked by this transaction
        let locked_pages = transaction_locks.remove(&txn_id).unwrap_or_default();

        // Release locks on each page
        for page_id in locked_pages {
            if let Some(locks) = granted_locks.get_mut(&page_id) {
                locks.retain(|lock| lock.transaction_id != txn_id);

                // If no more locks on this page, remove the entry
                if locks.is_empty() {
                    granted_locks.remove(&page_id);
                }
            }

            // Try to grant waiting requests
            self.process_waiting_requests(page_id, &mut granted_locks, &mut waiting_requests, &mut transaction_locks)?;
        }

        Ok(())
    }

    /// Process waiting lock requests for a page
    fn process_waiting_requests(
        &self,
        page_id: PageId,
        granted_locks: &mut HashMap<PageId, Vec<LockGrant>>,
        waiting_requests: &mut HashMap<PageId, Vec<LockRequest>>,
        transaction_locks: &mut HashMap<TransactionId, HashSet<PageId>>,
    ) -> StorageResult<()> {
        if let Some(requests) = waiting_requests.get_mut(&page_id) {
            let current_locks = granted_locks.get(&page_id).cloned().unwrap_or_default();
            let mut granted_any = false;

            // Process requests in order
            requests.retain(|request| {
                if self.can_grant_lock(&current_locks, request.transaction_id, request.lock_type) {
                    // Grant the lock
                    let grant = LockGrant {
                        transaction_id: request.transaction_id,
                        lock_type: request.lock_type,
                        granted_at: crate::storage_engine::mvcc::MVCCManager::current_timestamp(),
                    };

                    granted_locks.entry(page_id).or_default().push(grant);
                    transaction_locks.entry(request.transaction_id).or_default().insert(page_id);
                    granted_any = true;
                    false // Remove from waiting queue
                } else {
                    true // Keep in waiting queue
                }
            });

            // If we granted any locks, clean up empty entries
            if granted_any && requests.is_empty() {
                waiting_requests.remove(&page_id);
            }
        }

        Ok(())
    }

    /// Check if a transaction holds a specific type of lock on a page
    pub fn holds_lock(&self, txn_id: TransactionId, page_id: PageId, lock_type: LockType) -> bool {
        let granted_locks = self.granted_locks.read().unwrap();
        if let Some(locks) = granted_locks.get(&page_id) {
            locks
                .iter()
                .any(|lock| lock.transaction_id == txn_id && (lock.lock_type == lock_type || (lock_type == LockType::Shared && lock.lock_type == LockType::Exclusive)))
        } else {
            false
        }
    }

    /// Get all transactions waiting for locks
    pub fn get_waiting_transactions(&self) -> Vec<TransactionId> {
        let waiting_requests = self.waiting_requests.read().unwrap();
        waiting_requests
            .values()
            .flat_map(|requests| requests.iter().map(|r| r.transaction_id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Get lock statistics
    pub fn get_statistics(&self) -> LockStatistics {
        let granted_locks = self.granted_locks.read().unwrap();
        let waiting_requests = self.waiting_requests.read().unwrap();
        let transaction_locks = self.transaction_locks.read().unwrap();

        let total_granted_locks: usize = granted_locks.values().map(|v| v.len()).sum();
        let total_waiting_requests: usize = waiting_requests.values().map(|v| v.len()).sum();
        let pages_with_locks = granted_locks.len();
        let active_lock_holders = transaction_locks.len();

        LockStatistics {
            total_granted_locks,
            total_waiting_requests,
            pages_with_locks,
            active_lock_holders,
        }
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the lock manager
#[derive(Debug, Clone)]
pub struct LockStatistics {
    /// Total number of granted locks
    pub total_granted_locks: usize,
    /// Total number of waiting lock requests
    pub total_waiting_requests: usize,
    /// Number of pages with locks
    pub pages_with_locks: usize,
    /// Number of transactions holding locks
    pub active_lock_holders: usize,
}

/// Isolation level enforcer
pub struct IsolationLevelEnforcer {
    /// MVCC manager for version control
    mvcc_manager: Arc<MVCCManager>,
    /// Lock manager for concurrency control
    lock_manager: Arc<LockManager>,
}

impl IsolationLevelEnforcer {
    /// Create a new isolation level enforcer
    pub fn new(mvcc_manager: Arc<MVCCManager>, lock_manager: Arc<LockManager>) -> Self {
        Self { mvcc_manager, lock_manager }
    }

    /// Check read operation according to isolation level
    pub fn check_read(&self, txn_id: TransactionId, page_id: PageId, isolation_level: IsolationLevel) -> StorageResult<bool> {
        match isolation_level {
            IsolationLevel::ReadUncommitted => {
                // No restrictions on reads
                Ok(true)
            }
            IsolationLevel::ReadCommitted => {
                // For Read Committed, we allow reads as long as there's no conflicting lock
                // MVCC will handle showing the appropriate version
                Ok(true)
            }
            IsolationLevel::RepeatableRead => {
                // Need shared lock to prevent other transactions from modifying
                if !self.lock_manager.holds_lock(txn_id, page_id, LockType::Shared) {
                    self.lock_manager.request_lock(txn_id, page_id, LockType::Shared)
                } else {
                    Ok(true)
                }
            }
            IsolationLevel::Serializable => {
                // Same as repeatable read but with additional serialization checks
                if !self.lock_manager.holds_lock(txn_id, page_id, LockType::Shared) {
                    let can_lock = self.lock_manager.request_lock(txn_id, page_id, LockType::Shared)?;
                    if can_lock {
                        // Additional check for serialization conflicts
                        self.check_serialization_conflict(txn_id, page_id)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(true)
                }
            }
        }
    }

    /// Check write operation according to isolation level
    pub fn check_write(&self, txn_id: TransactionId, page_id: PageId, isolation_level: IsolationLevel) -> StorageResult<bool> {
        match isolation_level {
            IsolationLevel::ReadUncommitted => {
                // Still need exclusive lock for writes to prevent dirty writes
                self.lock_manager.request_lock(txn_id, page_id, LockType::Exclusive)
            }
            IsolationLevel::ReadCommitted => {
                // Need exclusive lock and check for write conflicts
                let can_lock = self.lock_manager.request_lock(txn_id, page_id, LockType::Exclusive)?;
                if can_lock {
                    // Check for write-write conflicts using MVCC
                    let has_conflict = self.mvcc_manager.check_write_conflict(page_id, txn_id)?;
                    Ok(!has_conflict)
                } else {
                    Ok(false)
                }
            }
            IsolationLevel::RepeatableRead | IsolationLevel::Serializable => {
                // Need exclusive lock and strict conflict checking
                let can_lock = self.lock_manager.request_lock(txn_id, page_id, LockType::Exclusive)?;
                if can_lock {
                    let has_conflict = self.mvcc_manager.check_write_conflict(page_id, txn_id)?;
                    if has_conflict {
                        // Release the lock since we can't proceed
                        self.lock_manager.release_transaction_locks(txn_id)?;
                        Ok(false)
                    } else {
                        Ok(true)
                    }
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Check for serialization conflicts (phantom reads, etc.)
    fn check_serialization_conflict(&self, _txn_id: TransactionId, _page_id: PageId) -> StorageResult<bool> {
        // For now, this is a placeholder for more advanced serialization checks
        // In a full implementation, this would check for:
        // - Phantom reads
        // - Serialization anomalies
        // - Predicate locking conflicts
        Ok(true)
    }

    /// Handle transaction commit for isolation level
    pub fn handle_commit(&self, txn_id: TransactionId) -> StorageResult<()> {
        // Commit in MVCC manager
        self.mvcc_manager.commit_transaction(txn_id)?;

        // Release all locks
        self.lock_manager.release_transaction_locks(txn_id)?;

        Ok(())
    }

    /// Handle transaction abort for isolation level
    pub fn handle_abort(&self, txn_id: TransactionId) -> StorageResult<()> {
        // Abort in MVCC manager
        self.mvcc_manager.abort_transaction(txn_id)?;

        // Release all locks
        self.lock_manager.release_transaction_locks(txn_id)?;

        Ok(())
    }

    /// Get isolation level statistics
    pub fn get_isolation_statistics(&self) -> IsolationStatistics {
        let mvcc_stats = self.mvcc_manager.get_statistics();
        let lock_stats = self.lock_manager.get_statistics();

        IsolationStatistics {
            mvcc_statistics: mvcc_stats,
            lock_statistics: lock_stats,
        }
    }
}

/// Combined statistics for isolation levels
#[derive(Debug, Clone)]
pub struct IsolationStatistics {
    /// MVCC system statistics
    pub mvcc_statistics: crate::storage_engine::mvcc::MVCCStatistics,
    /// Lock manager statistics
    pub lock_statistics: LockStatistics,
}

/// Helper functions for isolation level behavior
pub mod isolation_helpers {
    use super::*;

    /// Get the description of an isolation level
    pub fn get_isolation_level_description(level: IsolationLevel) -> &'static str {
        match level {
            IsolationLevel::ReadUncommitted => "Allows dirty reads, prevents dirty writes. Lowest isolation, highest concurrency.",
            IsolationLevel::ReadCommitted => "Prevents dirty reads and dirty writes. Each statement sees committed data.",
            IsolationLevel::RepeatableRead => "Prevents dirty reads, dirty writes, and non-repeatable reads. Consistent snapshot.",
            IsolationLevel::Serializable => "Prevents all concurrency anomalies. Transactions appear to execute serially.",
        }
    }

    /// Check if an isolation level requires locking
    pub fn requires_locking(level: IsolationLevel) -> bool {
        matches!(level, IsolationLevel::RepeatableRead | IsolationLevel::Serializable)
    }

    /// Check if an isolation level uses MVCC
    pub fn uses_mvcc(level: IsolationLevel) -> bool {
        matches!(level, IsolationLevel::ReadCommitted | IsolationLevel::RepeatableRead | IsolationLevel::Serializable)
    }

    /// Get the recommended isolation level for different use cases
    pub fn get_recommended_isolation_level(use_case: &str) -> IsolationLevel {
        match use_case.to_lowercase().as_str() {
            "analytics" | "reporting" => IsolationLevel::ReadCommitted,
            "financial" | "accounting" => IsolationLevel::Serializable,
            "web_application" => IsolationLevel::ReadCommitted,
            "high_throughput" => IsolationLevel::ReadUncommitted,
            "data_migration" => IsolationLevel::ReadUncommitted,
            _ => IsolationLevel::ReadCommitted, // Default safe choice
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_engine::file_format::{Page, PageHeader, PageType};
    use crate::storage_engine::lib::VersionId;

    fn create_test_page() -> Arc<crate::storage_engine::file_format::Page> {
        let data = vec![0u8; 4096];
        Arc::new(Page::new(PageId(1), PageType::Data, VersionId(0), data.len()))
    }

    #[test]
    fn test_lock_manager_basic_operations() {
        let lock_manager = LockManager::new();
        let txn_id = 1;
        let page_id = PageId(1);

        // Request shared lock
        let granted = lock_manager.request_lock(txn_id, page_id, LockType::Shared).unwrap();
        assert!(granted);

        // Check if lock is held
        assert!(lock_manager.holds_lock(txn_id, page_id, LockType::Shared));

        // Release locks
        lock_manager.release_transaction_locks(txn_id).unwrap();
        assert!(!lock_manager.holds_lock(txn_id, page_id, LockType::Shared));
    }

    #[test]
    fn test_lock_compatibility() {
        let lock_manager = LockManager::new();
        let page_id = PageId(1);

        // Multiple shared locks should be compatible
        assert!(lock_manager.request_lock(1, page_id, LockType::Shared).unwrap());
        assert!(lock_manager.request_lock(2, page_id, LockType::Shared).unwrap());

        // Exclusive lock should not be grantable when shared locks exist
        assert!(!lock_manager.request_lock(3, page_id, LockType::Exclusive).unwrap());

        // Release shared locks
        lock_manager.release_transaction_locks(1).unwrap();
        lock_manager.release_transaction_locks(2).unwrap();

        // Now exclusive lock should be grantable
        assert!(lock_manager.request_lock(3, page_id, LockType::Exclusive).unwrap());

        // No other locks should be grantable
        assert!(!lock_manager.request_lock(4, page_id, LockType::Shared).unwrap());
        assert!(!lock_manager.request_lock(5, page_id, LockType::Exclusive).unwrap());
    }

    #[test]
    fn test_lock_upgrade() {
        let lock_manager = LockManager::new();
        let txn_id = 1;
        let page_id = PageId(1);

        // Get shared lock first
        assert!(lock_manager.request_lock(txn_id, page_id, LockType::Shared).unwrap());

        // Should be able to upgrade to exclusive if we're the only holder
        assert!(lock_manager.request_lock(txn_id, page_id, LockType::Exclusive).unwrap());

        assert!(lock_manager.holds_lock(txn_id, page_id, LockType::Exclusive));
    }

    #[test]
    fn test_isolation_level_enforcer() {
        let mvcc = Arc::new(MVCCManager::new());
        let lock_manager = Arc::new(LockManager::new());
        let enforcer = IsolationLevelEnforcer::new(mvcc.clone(), lock_manager.clone());

        let txn_id = 1;
        let page_id = PageId(1);

        // Create snapshot in MVCC
        mvcc.create_snapshot(txn_id, IsolationLevel::ReadCommitted).unwrap();

        // Test read with different isolation levels
        assert!(enforcer.check_read(txn_id, page_id, IsolationLevel::ReadUncommitted).unwrap());

        // Test write operations
        assert!(enforcer.check_write(txn_id, page_id, IsolationLevel::ReadCommitted).unwrap());

        // Test commit
        enforcer.handle_commit(txn_id).unwrap();
    }

    #[test]
    fn test_read_committed_isolation() {
        let mvcc = Arc::new(MVCCManager::new());
        let lock_manager = Arc::new(LockManager::new());
        let enforcer = IsolationLevelEnforcer::new(mvcc.clone(), lock_manager.clone());

        let txn1 = 1;
        let txn2 = 2;
        let page_id = PageId(1);

        // Setup initial data
        mvcc.create_snapshot(txn1, IsolationLevel::ReadCommitted).unwrap();
        mvcc.add_version(page_id, create_test_page(), txn1).unwrap();
        mvcc.commit_transaction(txn1).unwrap();

        // Start second transaction
        mvcc.create_snapshot(txn2, IsolationLevel::ReadCommitted).unwrap();

        // Should be able to read committed data
        assert!(enforcer.check_read(txn2, page_id, IsolationLevel::ReadCommitted).unwrap());
    }

    #[test]
    fn test_repeatable_read_isolation() {
        let mvcc = Arc::new(MVCCManager::new());
        let lock_manager = Arc::new(LockManager::new());
        let enforcer = IsolationLevelEnforcer::new(mvcc.clone(), lock_manager.clone());

        let txn_id = 1;
        let page_id = PageId(1);

        mvcc.create_snapshot(txn_id, IsolationLevel::RepeatableRead).unwrap();

        // First read should acquire shared lock
        assert!(enforcer.check_read(txn_id, page_id, IsolationLevel::RepeatableRead).unwrap());
        assert!(lock_manager.holds_lock(txn_id, page_id, LockType::Shared));

        // Subsequent reads should still work (lock already held)
        assert!(enforcer.check_read(txn_id, page_id, IsolationLevel::RepeatableRead).unwrap());
    }

    #[test]
    fn test_serializable_isolation() {
        let mvcc = Arc::new(MVCCManager::new());
        let lock_manager = Arc::new(LockManager::new());
        let enforcer = IsolationLevelEnforcer::new(mvcc.clone(), lock_manager.clone());

        let txn_id = 1;
        let page_id = PageId(1);

        mvcc.create_snapshot(txn_id, IsolationLevel::Serializable).unwrap();

        // Serializable should use same locking as repeatable read
        assert!(enforcer.check_read(txn_id, page_id, IsolationLevel::Serializable).unwrap());
        assert!(lock_manager.holds_lock(txn_id, page_id, LockType::Shared));

        // Write should require exclusive lock
        assert!(enforcer.check_write(txn_id, page_id, IsolationLevel::Serializable).unwrap());
        assert!(lock_manager.holds_lock(txn_id, page_id, LockType::Exclusive));
    }

    #[test]
    fn test_lock_statistics() {
        let lock_manager = LockManager::new();

        let initial_stats = lock_manager.get_statistics();
        assert_eq!(initial_stats.total_granted_locks, 0);

        // Add some locks
        lock_manager.request_lock(1, PageId(1), LockType::Shared).unwrap();
        lock_manager.request_lock(2, PageId(1), LockType::Shared).unwrap();
        lock_manager.request_lock(3, PageId(2), LockType::Exclusive).unwrap();

        let stats = lock_manager.get_statistics();
        assert!(stats.total_granted_locks >= 3);
        assert!(stats.pages_with_locks >= 2);
        assert!(stats.active_lock_holders >= 3);
    }

    #[test]
    fn test_isolation_helpers() {
        use super::isolation_helpers::*;

        // Test descriptions
        let desc = get_isolation_level_description(IsolationLevel::ReadCommitted);
        assert!(desc.contains("dirty reads"));

        // Test locking requirements
        assert!(!requires_locking(IsolationLevel::ReadUncommitted));
        assert!(!requires_locking(IsolationLevel::ReadCommitted));
        assert!(requires_locking(IsolationLevel::RepeatableRead));
        assert!(requires_locking(IsolationLevel::Serializable));

        // Test MVCC usage
        assert!(!uses_mvcc(IsolationLevel::ReadUncommitted));
        assert!(uses_mvcc(IsolationLevel::ReadCommitted));
        assert!(uses_mvcc(IsolationLevel::RepeatableRead));
        assert!(uses_mvcc(IsolationLevel::Serializable));

        // Test recommendations
        assert_eq!(get_recommended_isolation_level("financial"), IsolationLevel::Serializable);
        assert_eq!(get_recommended_isolation_level("web_application"), IsolationLevel::ReadCommitted);
        assert_eq!(get_recommended_isolation_level("high_throughput"), IsolationLevel::ReadUncommitted);
    }

    #[test]
    fn test_transaction_abort_releases_locks() {
        let mvcc = Arc::new(MVCCManager::new());
        let lock_manager = Arc::new(LockManager::new());
        let enforcer = IsolationLevelEnforcer::new(mvcc.clone(), lock_manager.clone());

        let txn_id = 1;
        let page_id = PageId(1);

        // Acquire locks
        mvcc.create_snapshot(txn_id, IsolationLevel::RepeatableRead).unwrap();
        assert!(enforcer.check_read(txn_id, page_id, IsolationLevel::RepeatableRead).unwrap());
        assert!(lock_manager.holds_lock(txn_id, page_id, LockType::Shared));

        // Abort transaction
        enforcer.handle_abort(txn_id).unwrap();

        // Locks should be released
        assert!(!lock_manager.holds_lock(txn_id, page_id, LockType::Shared));
    }
}
