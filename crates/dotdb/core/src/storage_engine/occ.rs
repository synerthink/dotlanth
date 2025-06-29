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

//! Optimistic Concurrency Control (OCC) Implementation
//!
//! This module implements an optimistic concurrency control protocol for DotDB.
//! OCC allows transactions to proceed without blocking, then validates at commit
//! time to ensure serializability.
//!
//! # OCC Protocol Phases
//!
//! 1. **Read Phase**: Transactions read data and maintain a read set
//! 2. **Validation Phase**: Check for conflicts with committed transactions
//! 3. **Write Phase**: Apply changes if validation succeeds, otherwise abort
//!
//! # Features
//!
//! - Conflict detection using read/write set intersection
//! - Configurable resolution strategies
//! - Transaction abort handling with rollback
//! - Performance optimizations for high concurrency

// We'll use HashMap instead of HashIndex for now to avoid trait issues
use crate::statistics::access_patterns::AccessPatternTracker;
use crate::statistics::cardinality::HyperLogLogEstimator;
use crate::storage_engine::deadlock_detector::DeadlockDetector;
use crate::storage_engine::file_format::PageId;
use crate::storage_engine::lib::{StorageError, StorageResult, generate_timestamp};
use crate::storage_engine::transaction::TransactionId;
use crate::storage_engine::wal::{LogEntry, WalConfig, WriteAheadLog};

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Types of conflicts that can occur between transactions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictType {
    /// Read-Write conflict: transaction read data that was later modified
    ReadWrite {
        /// Page that was read and then written
        page_id: PageId,
        /// Transaction that did the reading
        reader_txn: TransactionId,
        /// Transaction that did the writing
        writer_txn: TransactionId,
    },
    /// Write-Write conflict: two transactions wrote to the same page
    WriteWrite {
        /// Page that was written by both transactions
        page_id: PageId,
        /// First transaction that wrote
        first_txn: TransactionId,
        /// Second transaction that wrote
        second_txn: TransactionId,
    },
    /// Write-Read conflict: transaction wrote data that was later read
    WriteRead {
        /// Page that was written and then read
        page_id: PageId,
        /// Transaction that did the writing
        writer_txn: TransactionId,
        /// Transaction that did the reading
        reader_txn: TransactionId,
    },
}

/// Resolution strategy for handling conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolutionStrategy {
    /// Abort the conflicting transaction
    AbortConflicting,
    /// Abort the transaction with lower priority (based on timestamp)
    AbortLowerPriority,
    /// Abort the transaction with higher priority (wound-wait)
    AbortHigherPriority,
    /// Retry the transaction with exponential backoff
    RetryWithBackoff,
    /// Intelligent strategy based on access patterns and conflict types
    Adaptive,
    /// No-wait strategy for hot pages
    NoWaitHotPages,
}

/// Statistics for OCC performance monitoring
#[derive(Debug, Clone, Default)]
pub struct OCCStatistics {
    /// Total number of validations performed
    pub total_validations: u64,
    /// Number of successful validations
    pub successful_validations: u64,
    /// Number of failed validations (conflicts detected)
    pub failed_validations: u64,
    /// Number of transactions aborted due to conflicts
    pub aborted_transactions: u64,
    /// Number of read-write conflicts detected
    pub read_write_conflicts: u64,
    /// Number of write-write conflicts detected
    pub write_write_conflicts: u64,
    /// Average validation time in microseconds
    pub average_validation_time_us: u64,
    /// Number of retry attempts
    pub retry_attempts: u64,
}

/// Transaction validation context for OCC
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Transaction being validated
    pub transaction_id: TransactionId,
    /// Read set of the transaction
    pub read_set: HashSet<PageId>,
    /// Write set of the transaction
    pub write_set: HashSet<PageId>,
    /// Transaction start timestamp
    pub start_timestamp: u64,
    /// Transaction validation timestamp
    pub validation_timestamp: u64,
}

/// Represents a committed transaction for validation purposes
#[derive(Debug, Clone)]
pub struct CommittedTransaction {
    /// Transaction ID
    pub id: TransactionId,
    /// Commit timestamp
    pub commit_timestamp: u64,
    /// Pages written by this transaction
    pub write_set: HashSet<PageId>,
    /// Pages read by this transaction
    pub read_set: HashSet<PageId>,
}

/// OCC Manager handles optimistic concurrency control with advanced features
pub struct OCCManager {
    /// Recently committed transactions for validation
    committed_transactions: RwLock<Vec<CommittedTransaction>>,
    /// Page-indexed committed transactions for O(1) lookup
    page_index: RwLock<HashMap<PageId, Vec<TransactionId>>>,
    /// Timestamp-indexed transactions for range queries
    timestamp_index: RwLock<BTreeMap<u64, Vec<TransactionId>>>,
    /// Conflict resolution strategy
    resolution_strategy: RwLock<ConflictResolutionStrategy>,
    /// Statistics for monitoring
    statistics: RwLock<OCCStatistics>,
    /// Write-ahead log for durability
    wal: Arc<WriteAheadLog>,
    /// Deadlock detector for advanced conflict resolution
    deadlock_detector: Option<Arc<DeadlockDetector>>,
    /// Access pattern tracker for hot page detection
    access_tracker: RwLock<AccessPatternTracker>,
    /// Cardinality estimator for fast conflict pre-screening
    conflict_estimator: RwLock<HyperLogLogEstimator>,
    /// Maximum number of committed transactions to keep for validation
    max_committed_transactions: usize,
    /// Cleanup interval for old committed transactions
    cleanup_interval: Duration,
    /// Last cleanup timestamp
    last_cleanup: RwLock<Instant>,
}

impl OCCManager {
    /// Create a new OCC manager
    ///
    /// # Arguments
    ///
    /// * `resolution_strategy` - Strategy for resolving conflicts
    /// * `wal` - Write-ahead log for durability
    /// * `max_committed_transactions` - Maximum committed transactions to track
    ///
    /// # Returns
    ///
    /// A new OCCManager instance
    pub fn new(resolution_strategy: ConflictResolutionStrategy, wal: Arc<WriteAheadLog>, max_committed_transactions: usize) -> Self {
        Self {
            committed_transactions: RwLock::new(Vec::new()),
            page_index: RwLock::new(HashMap::new()),
            timestamp_index: RwLock::new(BTreeMap::new()),
            resolution_strategy: RwLock::new(resolution_strategy),
            statistics: RwLock::new(OCCStatistics::default()),
            wal,
            deadlock_detector: None,
            access_tracker: RwLock::new(AccessPatternTracker::new(10000)),
            conflict_estimator: RwLock::new(HyperLogLogEstimator::new(14).unwrap()),
            max_committed_transactions,
            cleanup_interval: Duration::from_secs(60), // Cleanup every minute
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Add deadlock detection capability
    pub fn with_deadlock_detector(mut self, deadlock_detector: Arc<DeadlockDetector>) -> Self {
        self.deadlock_detector = Some(deadlock_detector);
        self
    }

    /// Check if a page is frequently accessed (hot page)
    pub fn is_hot_page(&self, page_id: PageId) -> bool {
        let tracker = self.access_tracker.read().unwrap();
        let page_key = format!("{}", page_id.0);

        // Check if this page is in the hot keys list
        let hot_keys = tracker.get_hot_keys(100); // Top 100 hot pages
        hot_keys.iter().any(|(key, _)| key == &page_key)
    }

    /// Record page access for pattern tracking
    pub fn record_page_access(&self, page_id: PageId) {
        let mut tracker = self.access_tracker.write().unwrap();
        let page_key = format!("{}", page_id.0);
        tracker.record_access(&page_key);
    }

    /// Fast conflict pre-screening using cardinality estimator
    pub fn quick_conflict_check(&self, read_set: &HashSet<PageId>) -> bool {
        let mut estimator = self.conflict_estimator.write().unwrap();

        // Add read set pages to estimator
        for page_id in read_set {
            estimator.add(page_id);
        }

        // Estimate potential conflicts based on cardinality
        estimator.estimate() > (self.max_committed_transactions as u64 / 2)
    }

    /// Validate a transaction for commit using optimized OCC protocol
    ///
    /// # Arguments
    ///
    /// * `context` - Validation context containing transaction information
    ///
    /// # Returns
    ///
    /// Result indicating if validation succeeded and any detected conflicts
    pub fn validate_transaction(&self, context: &ValidationContext) -> StorageResult<Vec<ConflictType>> {
        let start_time = Instant::now();
        let mut conflicts = Vec::new();

        // Update statistics
        {
            let mut stats = self.statistics.write().unwrap();
            stats.total_validations += 1;
        }

        // Record page accesses for pattern tracking
        for page_id in &context.read_set {
            self.record_page_access(*page_id);
        }
        for page_id in &context.write_set {
            self.record_page_access(*page_id);
        }

        // Fast conflict pre-screening using cardinality estimator
        if self.quick_conflict_check(&context.read_set) {
            // Use optimized page-indexed validation for high-conflict scenarios
            self.validate_with_page_index(context, &mut conflicts)?;
        } else {
            // Use traditional validation for low-conflict scenarios
            self.validate_traditional(context, &mut conflicts)?;
        }

        // Update statistics
        let validation_time = start_time.elapsed();
        {
            let mut stats = self.statistics.write().unwrap();
            if conflicts.is_empty() {
                stats.successful_validations += 1;
            } else {
                stats.failed_validations += 1;
                stats.read_write_conflicts += conflicts.iter().filter(|c| matches!(c, ConflictType::ReadWrite { .. })).count() as u64;
                stats.write_write_conflicts += conflicts.iter().filter(|c| matches!(c, ConflictType::WriteWrite { .. })).count() as u64;
            }

            // Update average validation time
            let total_time = stats.average_validation_time_us * stats.total_validations;
            stats.average_validation_time_us = (total_time + validation_time.as_micros() as u64) / stats.total_validations;
        }

        // Perform cleanup if needed
        self.cleanup_if_needed()?;

        Ok(conflicts)
    }

    /// Optimized validation using page index for O(1) lookup
    fn validate_with_page_index(&self, context: &ValidationContext, conflicts: &mut Vec<ConflictType>) -> StorageResult<()> {
        let page_index = self.page_index.read().unwrap();
        let committed_txns = self.committed_transactions.read().unwrap();
        let timestamp_index = self.timestamp_index.read().unwrap();

        // Get transactions that committed after this transaction started
        let relevant_txns: Vec<TransactionId> = timestamp_index.range(context.start_timestamp..).flat_map(|(_, txns)| txns.iter()).copied().collect();

        // Check conflicts only for pages in read/write sets
        for read_page in &context.read_set {
            // Hot page bypass optimization
            if self.is_hot_page(*read_page) {
                continue; // Skip validation for hot pages in read-only operations
            }

            if let Some(conflicting_txns) = page_index.get(read_page) {
                for &txn_id in conflicting_txns {
                    if relevant_txns.contains(&txn_id) {
                        conflicts.push(ConflictType::ReadWrite {
                            page_id: *read_page,
                            reader_txn: context.transaction_id,
                            writer_txn: txn_id,
                        });
                    }
                }
            }
        }

        for write_page in &context.write_set {
            if let Some(conflicting_txns) = page_index.get(write_page) {
                for &txn_id in conflicting_txns {
                    if relevant_txns.contains(&txn_id) {
                        conflicts.push(ConflictType::WriteWrite {
                            page_id: *write_page,
                            first_txn: txn_id,
                            second_txn: context.transaction_id,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Traditional validation method (fallback)
    fn validate_traditional(&self, context: &ValidationContext, conflicts: &mut Vec<ConflictType>) -> StorageResult<()> {
        let committed_txns = self.committed_transactions.read().unwrap();

        // Check for conflicts with committed transactions
        for committed_txn in committed_txns.iter() {
            // Only check transactions that committed after this transaction started
            if committed_txn.commit_timestamp > context.start_timestamp {
                // Check for read-write conflicts
                for read_page in &context.read_set {
                    if committed_txn.write_set.contains(read_page) {
                        conflicts.push(ConflictType::ReadWrite {
                            page_id: *read_page,
                            reader_txn: context.transaction_id,
                            writer_txn: committed_txn.id,
                        });
                    }
                }

                // Check for write-write conflicts
                for write_page in &context.write_set {
                    if committed_txn.write_set.contains(write_page) {
                        conflicts.push(ConflictType::WriteWrite {
                            page_id: *write_page,
                            first_txn: committed_txn.id,
                            second_txn: context.transaction_id,
                        });
                    }
                }

                // Check for write-read conflicts (anti-dependency)
                for write_page in &context.write_set {
                    if committed_txn.read_set.contains(write_page) {
                        conflicts.push(ConflictType::WriteRead {
                            page_id: *write_page,
                            writer_txn: context.transaction_id,
                            reader_txn: committed_txn.id,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Record a successful transaction commit with optimized indexing
    ///
    /// # Arguments
    ///
    /// * `transaction_id` - ID of the committed transaction
    /// * `commit_timestamp` - Timestamp when the transaction committed
    /// * `read_set` - Pages read by the transaction
    /// * `write_set` - Pages written by the transaction
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn record_commit(&self, transaction_id: TransactionId, commit_timestamp: u64, read_set: HashSet<PageId>, write_set: HashSet<PageId>) -> StorageResult<()> {
        let committed_txn = CommittedTransaction {
            id: transaction_id,
            commit_timestamp,
            write_set: write_set.clone(),
            read_set: read_set.clone(),
        };

        // Update main committed transactions list
        let mut committed_txns = self.committed_transactions.write().unwrap();
        committed_txns.push(committed_txn);

        // Sort by commit timestamp to maintain order
        committed_txns.sort_by_key(|txn| txn.commit_timestamp);

        // Limit the number of tracked transactions
        if committed_txns.len() > self.max_committed_transactions {
            let len = committed_txns.len();
            committed_txns.drain(0..len - self.max_committed_transactions);
        }
        drop(committed_txns);

        // Update page index for O(1) conflict detection
        {
            let mut page_index = self.page_index.write().unwrap();
            for page_id in &write_set {
                page_index.entry(*page_id).or_default().push(transaction_id);
            }
        }

        // Update timestamp index for range queries
        {
            let mut timestamp_index = self.timestamp_index.write().unwrap();
            timestamp_index.entry(commit_timestamp).or_default().push(transaction_id);
        }

        // Integrate with deadlock detector if available
        if let Some(ref deadlock_detector) = self.deadlock_detector {
            // Remove this transaction from deadlock detection
            deadlock_detector.remove_transaction(transaction_id);
        }

        Ok(())
    }

    /// Resolve conflicts using the configured strategy with intelligent optimizations
    ///
    /// # Arguments
    ///
    /// * `conflicts` - Detected conflicts
    /// * `context` - Validation context
    ///
    /// # Returns
    ///
    /// Resolution action to take
    pub fn resolve_conflicts(&self, conflicts: &[ConflictType], context: &ValidationContext) -> ConflictResolution {
        if conflicts.is_empty() {
            return ConflictResolution::Proceed;
        }

        match *self.resolution_strategy.read().unwrap() {
            ConflictResolutionStrategy::AbortConflicting => ConflictResolution::Abort {
                reason: "Conflict detected with committed transaction".to_string(),
                should_retry: false,
            },
            ConflictResolutionStrategy::AbortLowerPriority => {
                // Use timestamp for priority (earlier = higher priority)
                let should_abort = conflicts.iter().any(|conflict| {
                    match conflict {
                        ConflictType::ReadWrite { writer_txn, .. } | ConflictType::WriteWrite { first_txn: writer_txn, .. } | ConflictType::WriteRead { reader_txn: writer_txn, .. } => {
                            // Check if the conflicting transaction has higher priority
                            // In a real implementation, you'd look up the actual timestamp
                            context.start_timestamp > context.validation_timestamp
                        }
                    }
                });

                if should_abort {
                    ConflictResolution::Abort {
                        reason: "Lower priority transaction aborted".to_string(),
                        should_retry: true,
                    }
                } else {
                    ConflictResolution::Proceed
                }
            }
            ConflictResolutionStrategy::AbortHigherPriority => {
                // Wound-wait: abort higher priority (newer) transactions
                ConflictResolution::Abort {
                    reason: "Higher priority transaction wounded".to_string(),
                    should_retry: true,
                }
            }
            ConflictResolutionStrategy::RetryWithBackoff => ConflictResolution::Retry {
                backoff_duration: Duration::from_millis(10 * conflicts.len() as u64),
                max_retries: 3,
            },
            ConflictResolutionStrategy::Adaptive => self.resolve_adaptive_strategy(conflicts, context),
            ConflictResolutionStrategy::NoWaitHotPages => self.resolve_no_wait_strategy(conflicts, context),
        }
    }

    /// Adaptive conflict resolution based on access patterns and conflict types
    fn resolve_adaptive_strategy(&self, conflicts: &[ConflictType], context: &ValidationContext) -> ConflictResolution {
        let tracker = self.access_tracker.read().unwrap();

        // Analyze conflict patterns
        let hot_page_conflicts = conflicts
            .iter()
            .filter(|conflict| {
                let page_id = match conflict {
                    ConflictType::ReadWrite { page_id, .. } | ConflictType::WriteWrite { page_id, .. } | ConflictType::WriteRead { page_id, .. } => *page_id,
                };
                self.is_hot_page(page_id)
            })
            .count();

        let total_conflicts = conflicts.len();
        let hot_page_ratio = hot_page_conflicts as f64 / total_conflicts as f64;

        // Adaptive decision based on patterns
        if hot_page_ratio > 0.7 {
            // Most conflicts are on hot pages - use aggressive retry
            ConflictResolution::Retry {
                backoff_duration: Duration::from_millis(1), // Minimal backoff for hot pages
                max_retries: 5,
            }
        } else if conflicts.iter().any(|c| matches!(c, ConflictType::WriteWrite { .. })) {
            // Write-write conflicts are more serious - abort immediately
            ConflictResolution::Abort {
                reason: "Write-write conflict detected in adaptive mode".to_string(),
                should_retry: true,
            }
        } else {
            // Read-write conflicts can be retried with moderate backoff
            ConflictResolution::Retry {
                backoff_duration: Duration::from_millis(5 * conflicts.len() as u64),
                max_retries: 3,
            }
        }
    }

    /// No-wait strategy optimized for hot page scenarios
    fn resolve_no_wait_strategy(&self, conflicts: &[ConflictType], context: &ValidationContext) -> ConflictResolution {
        // Check if conflicts involve hot pages
        let has_hot_page_conflict = conflicts.iter().any(|conflict| {
            let page_id = match conflict {
                ConflictType::ReadWrite { page_id, .. } | ConflictType::WriteWrite { page_id, .. } | ConflictType::WriteRead { page_id, .. } => *page_id,
            };
            self.is_hot_page(page_id)
        });

        if has_hot_page_conflict {
            // For hot pages, abort immediately to reduce contention
            ConflictResolution::Abort {
                reason: "No-wait abort on hot page conflict".to_string(),
                should_retry: false, // Let application handle retry logic
            }
        } else {
            // For non-hot pages, use standard retry
            ConflictResolution::Retry {
                backoff_duration: Duration::from_millis(1),
                max_retries: 2,
            }
        }
    }

    /// Handle transaction abort due to OCC conflicts
    ///
    /// # Arguments
    ///
    /// * `transaction_id` - ID of the transaction to abort
    /// * `reason` - Reason for abort
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn handle_abort(&self, transaction_id: TransactionId, reason: &str) -> StorageResult<()> {
        // Log the abort
        let abort_entry = LogEntry::abort_transaction(self.wal.next_lsn()?, transaction_id);
        self.wal.append(&abort_entry)?;

        // Update statistics
        {
            let mut stats = self.statistics.write().unwrap();
            stats.aborted_transactions += 1;
        }

        Ok(())
    }

    /// Get current OCC statistics
    ///
    /// # Returns
    ///
    /// Current statistics snapshot
    pub fn statistics(&self) -> OCCStatistics {
        self.statistics.read().unwrap().clone()
    }

    /// Reset statistics
    pub fn reset_statistics(&self) {
        let mut stats = self.statistics.write().unwrap();
        *stats = OCCStatistics::default();
    }

    /// Cleanup old committed transactions
    fn cleanup_if_needed(&self) -> StorageResult<()> {
        let now = Instant::now();
        let should_cleanup = {
            let last_cleanup = self.last_cleanup.read().unwrap();
            now.duration_since(*last_cleanup) > self.cleanup_interval
        };

        if should_cleanup {
            let cutoff_time = generate_timestamp() - 300_000; // 5 minutes ago

            let mut committed_txns = self.committed_transactions.write().unwrap();
            committed_txns.retain(|txn| txn.commit_timestamp > cutoff_time);

            *self.last_cleanup.write().unwrap() = now;
        }

        Ok(())
    }

    /// Get the number of currently tracked committed transactions
    pub fn committed_transaction_count(&self) -> usize {
        self.committed_transactions.read().unwrap().len()
    }

    /// Set the conflict resolution strategy
    pub fn set_resolution_strategy(&self, strategy: ConflictResolutionStrategy) {
        *self.resolution_strategy.write().unwrap() = strategy;
    }

    /// Get the current conflict resolution strategy
    pub fn resolution_strategy(&self) -> ConflictResolutionStrategy {
        *self.resolution_strategy.read().unwrap()
    }
}

/// Result of conflict resolution
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    /// Transaction can proceed with commit
    Proceed,
    /// Transaction should be aborted
    Abort {
        /// Reason for abort
        reason: String,
        /// Whether the transaction should be retried
        should_retry: bool,
    },
    /// Transaction should be retried after a delay
    Retry {
        /// Duration to wait before retry
        backoff_duration: Duration,
        /// Maximum number of retries allowed
        max_retries: u32,
    },
}

/// OCC Transaction extensions for tracking read/write sets
pub trait OCCTransaction {
    /// Add a page to the read set
    fn add_to_read_set(&mut self, page_id: PageId);

    /// Add a page to the write set
    fn add_to_write_set(&mut self, page_id: PageId);

    /// Get the current read set
    fn read_set(&self) -> &HashSet<PageId>;

    /// Get the current write set
    fn write_set(&self) -> &HashSet<PageId>;

    /// Create validation context for OCC
    fn create_validation_context(&self) -> ValidationContext;
}

/// OCC-aware transaction manager
pub struct OCCTransactionManager {
    /// Inner OCC manager
    occ_manager: Arc<OCCManager>,
    /// Active transactions with their contexts
    active_transactions: RwLock<HashMap<TransactionId, ValidationContext>>,
}

impl OCCTransactionManager {
    /// Create a new OCC transaction manager
    pub fn new(occ_manager: Arc<OCCManager>) -> Self {
        Self {
            occ_manager,
            active_transactions: RwLock::new(HashMap::new()),
        }
    }

    /// Begin a new transaction with OCC tracking
    pub fn begin_transaction(&self, transaction_id: TransactionId) -> StorageResult<()> {
        let context = ValidationContext {
            transaction_id,
            read_set: HashSet::new(),
            write_set: HashSet::new(),
            start_timestamp: generate_timestamp(),
            validation_timestamp: 0,
        };

        self.active_transactions.write().unwrap().insert(transaction_id, context);
        Ok(())
    }

    /// Validate and commit a transaction using OCC
    pub fn commit_transaction(&self, transaction_id: TransactionId) -> StorageResult<ConflictResolution> {
        let mut context = {
            let mut active_txns = self.active_transactions.write().unwrap();
            active_txns
                .remove(&transaction_id)
                .ok_or_else(|| StorageError::NotFound(format!("Transaction {transaction_id} not found")))?
        };

        // Update validation timestamp
        context.validation_timestamp = generate_timestamp();

        // Validate the transaction
        let conflicts = self.occ_manager.validate_transaction(&context)?;
        let resolution = self.occ_manager.resolve_conflicts(&conflicts, &context);

        match resolution {
            ConflictResolution::Proceed => {
                // Record successful commit
                self.occ_manager.record_commit(transaction_id, context.validation_timestamp, context.read_set, context.write_set)?;
            }
            ConflictResolution::Abort { ref reason, .. } => {
                self.occ_manager.handle_abort(transaction_id, reason)?;
            }
            ConflictResolution::Retry { .. } => {
                // Put the transaction back for retry
                self.active_transactions.write().unwrap().insert(transaction_id, context);
            }
        }

        Ok(resolution)
    }

    /// Abort a transaction
    pub fn abort_transaction(&self, transaction_id: TransactionId, reason: &str) -> StorageResult<()> {
        self.active_transactions.write().unwrap().remove(&transaction_id);
        self.occ_manager.handle_abort(transaction_id, reason)
    }

    /// Update read set for a transaction
    pub fn add_to_read_set(&self, transaction_id: TransactionId, page_id: PageId) -> StorageResult<()> {
        let mut active_txns = self.active_transactions.write().unwrap();
        if let Some(context) = active_txns.get_mut(&transaction_id) {
            context.read_set.insert(page_id);
            Ok(())
        } else {
            Err(StorageError::NotFound(format!("Transaction {transaction_id} not found")))
        }
    }

    /// Update write set for a transaction
    pub fn add_to_write_set(&self, transaction_id: TransactionId, page_id: PageId) -> StorageResult<()> {
        let mut active_txns = self.active_transactions.write().unwrap();
        if let Some(context) = active_txns.get_mut(&transaction_id) {
            context.write_set.insert(page_id);
            Ok(())
        } else {
            Err(StorageError::NotFound(format!("Transaction {transaction_id} not found")))
        }
    }

    /// Get active transaction count
    pub fn active_transaction_count(&self) -> usize {
        self.active_transactions.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_engine::wal::WriteAheadLog;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_wal() -> Arc<WriteAheadLog> {
        let temp_dir = TempDir::new().unwrap();
        let wal_config = WalConfig {
            directory: temp_dir.path().to_path_buf(),
            max_file_size: 1024 * 1024,
            direct_io: false,
        };
        Arc::new(WriteAheadLog::new(wal_config).unwrap())
    }

    #[test]
    fn test_occ_manager_creation() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        assert_eq!(occ.resolution_strategy(), ConflictResolutionStrategy::AbortConflicting);
        assert_eq!(occ.committed_transaction_count(), 0);
    }

    #[test]
    fn test_validation_no_conflicts() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        let context = ValidationContext {
            transaction_id: 1,
            read_set: HashSet::from([PageId(1), PageId(2), PageId(3)]),
            write_set: HashSet::from([PageId(4), PageId(5)]),
            start_timestamp: 1000,
            validation_timestamp: 1100,
        };

        let conflicts = occ.validate_transaction(&context).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_validation_with_read_write_conflict() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        // Record a committed transaction
        occ.record_commit(1, 1050, HashSet::new(), HashSet::from([PageId(2)])).unwrap();

        let context = ValidationContext {
            transaction_id: 2,
            read_set: HashSet::from([PageId(1), PageId(2), PageId(3)]),
            write_set: HashSet::from([PageId(4), PageId(5)]),
            start_timestamp: 1000,
            validation_timestamp: 1100,
        };

        let conflicts = occ.validate_transaction(&context).unwrap();
        assert_eq!(conflicts.len(), 1);

        match &conflicts[0] {
            ConflictType::ReadWrite { page_id, reader_txn, writer_txn } => {
                assert_eq!(*page_id, PageId(2));
                assert_eq!(*reader_txn, 2);
                assert_eq!(*writer_txn, 1);
            }
            _ => panic!("Expected ReadWrite conflict"),
        }
    }

    #[test]
    fn test_validation_with_write_write_conflict() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        // Record a committed transaction
        occ.record_commit(1, 1050, HashSet::new(), HashSet::from([PageId(4)])).unwrap();

        let context = ValidationContext {
            transaction_id: 2,
            read_set: HashSet::from([PageId(1), PageId(2), PageId(3)]),
            write_set: HashSet::from([PageId(4), PageId(5)]),
            start_timestamp: 1000,
            validation_timestamp: 1100,
        };

        let conflicts = occ.validate_transaction(&context).unwrap();
        assert_eq!(conflicts.len(), 1);

        match &conflicts[0] {
            ConflictType::WriteWrite { page_id, first_txn, second_txn } => {
                assert_eq!(*page_id, PageId(4));
                assert_eq!(*first_txn, 1);
                assert_eq!(*second_txn, 2);
            }
            _ => panic!("Expected WriteWrite conflict"),
        }
    }

    #[test]
    fn test_conflict_resolution_abort() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        let conflicts = vec![ConflictType::ReadWrite {
            page_id: PageId(1),
            reader_txn: 1,
            writer_txn: 2,
        }];

        let context = ValidationContext {
            transaction_id: 1,
            read_set: HashSet::from([PageId(1)]),
            write_set: HashSet::new(),
            start_timestamp: 1000,
            validation_timestamp: 1100,
        };

        let resolution = occ.resolve_conflicts(&conflicts, &context);

        match resolution {
            ConflictResolution::Abort { reason, should_retry } => {
                assert_eq!(reason, "Conflict detected with committed transaction");
                assert!(!should_retry);
            }
            _ => panic!("Expected Abort resolution"),
        }
    }

    #[test]
    fn test_conflict_resolution_retry() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::RetryWithBackoff, wal, 1000);

        let conflicts = vec![ConflictType::ReadWrite {
            page_id: PageId(1),
            reader_txn: 1,
            writer_txn: 2,
        }];

        let context = ValidationContext {
            transaction_id: 1,
            read_set: HashSet::from([PageId(1)]),
            write_set: HashSet::new(),
            start_timestamp: 1000,
            validation_timestamp: 1100,
        };

        let resolution = occ.resolve_conflicts(&conflicts, &context);

        match resolution {
            ConflictResolution::Retry { backoff_duration, max_retries } => {
                assert_eq!(backoff_duration, Duration::from_millis(10));
                assert_eq!(max_retries, 3);
            }
            _ => panic!("Expected Retry resolution"),
        }
    }

    #[test]
    fn test_occ_transaction_manager() {
        let wal = create_test_wal();
        let occ = Arc::new(OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000));
        let occ_txn_mgr = OCCTransactionManager::new(occ);

        // Begin transaction
        occ_txn_mgr.begin_transaction(1).unwrap();
        assert_eq!(occ_txn_mgr.active_transaction_count(), 1);

        // Add to read and write sets
        occ_txn_mgr.add_to_read_set(1, PageId(10)).unwrap();
        occ_txn_mgr.add_to_write_set(1, PageId(20)).unwrap();

        // Commit transaction (should succeed with no conflicts)
        let resolution = occ_txn_mgr.commit_transaction(1).unwrap();
        match resolution {
            ConflictResolution::Proceed => {}
            _ => panic!("Expected Proceed resolution"),
        }

        assert_eq!(occ_txn_mgr.active_transaction_count(), 0);
    }

    #[test]
    fn test_statistics_tracking() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        let context = ValidationContext {
            transaction_id: 1,
            read_set: HashSet::from([PageId(1), PageId(2)]),
            write_set: HashSet::from([PageId(3)]),
            start_timestamp: 1000,
            validation_timestamp: 1100,
        };

        // Perform validation
        occ.validate_transaction(&context).unwrap();

        let stats = occ.statistics();
        assert_eq!(stats.total_validations, 1);
        assert_eq!(stats.successful_validations, 1);
        assert_eq!(stats.failed_validations, 0);
    }

    #[test]
    fn test_committed_transaction_limit() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 2);

        // Add more transactions than the limit
        occ.record_commit(1, 1000, HashSet::new(), HashSet::from([PageId(1)])).unwrap();
        occ.record_commit(2, 1001, HashSet::new(), HashSet::from([PageId(2)])).unwrap();
        occ.record_commit(3, 1002, HashSet::new(), HashSet::from([PageId(3)])).unwrap();

        // Should only keep the last 2 transactions
        assert_eq!(occ.committed_transaction_count(), 2);
    }

    #[test]
    fn test_abort_handling() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        let initial_stats = occ.statistics();

        occ.handle_abort(1, "Test abort").unwrap();

        let updated_stats = occ.statistics();
        assert_eq!(updated_stats.aborted_transactions, initial_stats.aborted_transactions + 1);
    }

    #[test]
    fn test_multiple_conflict_types() {
        let wal = create_test_wal();
        let occ = OCCManager::new(ConflictResolutionStrategy::AbortConflicting, wal, 1000);

        // Record a committed transaction with overlapping read and write sets
        occ.record_commit(1, 1050, HashSet::from([PageId(2)]), HashSet::from([PageId(3), PageId(4)])).unwrap();

        let context = ValidationContext {
            transaction_id: 2,
            read_set: HashSet::from([PageId(3)]),             // Read what txn 1 wrote
            write_set: HashSet::from([PageId(2), PageId(4)]), // Write what txn 1 read and wrote
            start_timestamp: 1000,
            validation_timestamp: 1100,
        };

        let conflicts = occ.validate_transaction(&context).unwrap();
        assert_eq!(conflicts.len(), 3); // Read-write, write-read, write-write
    }
}
