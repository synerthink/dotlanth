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

//! Deadlock Detection and Resolution
//!
//! This module implements a wait-for graph based deadlock detection system.
//! It identifies cycles in the transaction dependency graph and resolves
//! deadlocks by aborting the youngest transaction in the cycle.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::storage_engine::file_format::PageId;
use crate::storage_engine::lib::{StorageError, StorageResult};
use crate::storage_engine::transaction::TransactionId;

/// Represents a wait-for relationship between transactions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitForEdge {
    /// Transaction that is waiting
    pub waiter: TransactionId,
    /// Transaction that is being waited for
    pub holder: TransactionId,
    /// Resource being waited for
    pub resource: PageId,
    /// When this wait relationship was established
    pub wait_start_time: Instant,
}

impl WaitForEdge {
    /// Create a new wait-for edge
    pub fn new(waiter: TransactionId, holder: TransactionId, resource: PageId) -> Self {
        Self {
            waiter,
            holder,
            resource,
            wait_start_time: Instant::now(),
        }
    }

    /// Get the duration this transaction has been waiting
    pub fn wait_duration(&self) -> Duration {
        self.wait_start_time.elapsed()
    }
}

/// A cycle in the wait-for graph representing a deadlock
#[derive(Debug, Clone)]
pub struct DeadlockCycle {
    /// Transactions involved in the deadlock
    pub transactions: Vec<TransactionId>,
    /// Resources involved in the deadlock
    pub resources: Vec<PageId>,
    /// When this deadlock was detected
    pub detection_time: Instant,
}

impl DeadlockCycle {
    /// Get the youngest transaction in the cycle (highest transaction ID)
    pub fn youngest_transaction(&self) -> TransactionId {
        *self.transactions.iter().max().unwrap_or(&0)
    }

    /// Get the oldest transaction in the cycle (lowest transaction ID)
    pub fn oldest_transaction(&self) -> TransactionId {
        *self.transactions.iter().min().unwrap_or(&0)
    }

    /// Check if a transaction is part of this deadlock
    pub fn contains_transaction(&self, txn_id: TransactionId) -> bool {
        self.transactions.contains(&txn_id)
    }
}

/// Policy for resolving deadlocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlockResolutionPolicy {
    /// Abort the youngest transaction (highest ID)
    AbortYoungest,
    /// Abort the oldest transaction (lowest ID)
    AbortOldest,
    /// Abort the transaction with the least resources
    AbortLeastResources,
    /// Abort the transaction that has waited the longest
    AbortLongestWaiting,
}

/// Statistics about deadlock detection
#[derive(Debug, Clone, Default)]
pub struct DeadlockStatistics {
    /// Total number of deadlocks detected
    pub total_deadlocks_detected: u64,
    /// Total number of transactions aborted due to deadlocks
    pub total_transactions_aborted: u64,
    /// Number of currently active wait-for edges
    pub active_wait_edges: usize,
    /// Average deadlock detection time in microseconds
    pub average_detection_time_us: u64,
    /// Maximum wait time before deadlock detection
    pub max_wait_time_ms: u64,
}

/// Wait-for graph for deadlock detection
pub struct WaitForGraph {
    /// Edges in the wait-for graph
    edges: HashMap<TransactionId, Vec<WaitForEdge>>,
    /// Reverse mapping: transaction -> transactions waiting for it
    waiting_for: HashMap<TransactionId, HashSet<TransactionId>>,
    /// Transaction metadata for deadlock resolution
    transaction_metadata: HashMap<TransactionId, TransactionMetadata>,
}

/// Metadata about a transaction for deadlock resolution
#[derive(Debug, Clone)]
struct TransactionMetadata {
    /// When the transaction started
    start_time: Instant,
    /// Number of resources held by this transaction
    resources_held: usize,
    /// Total wait time for this transaction
    total_wait_time: Duration,
}

impl WaitForGraph {
    /// Create a new wait-for graph
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            waiting_for: HashMap::new(),
            transaction_metadata: HashMap::new(),
        }
    }

    /// Add a wait-for edge to the graph
    pub fn add_edge(&mut self, edge: WaitForEdge) {
        // Add to forward mapping
        self.edges.entry(edge.waiter).or_default().push(edge.clone());

        // Add to reverse mapping
        self.waiting_for.entry(edge.holder).or_default().insert(edge.waiter);

        // Update metadata
        self.transaction_metadata.entry(edge.waiter).or_insert_with(|| TransactionMetadata {
            start_time: Instant::now(),
            resources_held: 0,
            total_wait_time: Duration::default(),
        });
    }

    /// Remove a wait-for edge from the graph
    pub fn remove_edge(&mut self, waiter: TransactionId, holder: TransactionId) {
        // Remove from forward mapping
        if let Some(edges) = self.edges.get_mut(&waiter) {
            edges.retain(|edge| edge.holder != holder);
            if edges.is_empty() {
                self.edges.remove(&waiter);
            }
        }

        // Remove from reverse mapping
        if let Some(waiters) = self.waiting_for.get_mut(&holder) {
            waiters.remove(&waiter);
            if waiters.is_empty() {
                self.waiting_for.remove(&holder);
            }
        }
    }

    /// Remove all edges involving a transaction
    pub fn remove_transaction(&mut self, txn_id: TransactionId) {
        // Remove all edges where this transaction is waiting
        if let Some(edges) = self.edges.remove(&txn_id) {
            for edge in edges {
                if let Some(waiters) = self.waiting_for.get_mut(&edge.holder) {
                    waiters.remove(&txn_id);
                    if waiters.is_empty() {
                        self.waiting_for.remove(&edge.holder);
                    }
                }
            }
        }

        // Remove all edges where other transactions are waiting for this one
        if let Some(waiters) = self.waiting_for.remove(&txn_id) {
            for waiter in waiters {
                if let Some(edges) = self.edges.get_mut(&waiter) {
                    edges.retain(|edge| edge.holder != txn_id);
                    if edges.is_empty() {
                        self.edges.remove(&waiter);
                    }
                }
            }
        }

        // Remove metadata
        self.transaction_metadata.remove(&txn_id);
    }

    /// Detect cycles in the wait-for graph using DFS
    pub fn detect_deadlocks(&self) -> Vec<DeadlockCycle> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut current_path = Vec::new();
        let mut deadlocks = Vec::new();

        for &txn_id in self.edges.keys() {
            if !visited.contains(&txn_id) {
                self.dfs_detect_cycle(txn_id, &mut visited, &mut recursion_stack, &mut current_path, &mut deadlocks);
            }
        }

        deadlocks
    }

    /// Depth-first search to detect cycles
    fn dfs_detect_cycle(
        &self,
        txn_id: TransactionId,
        visited: &mut HashSet<TransactionId>,
        recursion_stack: &mut HashSet<TransactionId>,
        current_path: &mut Vec<TransactionId>,
        deadlocks: &mut Vec<DeadlockCycle>,
    ) {
        visited.insert(txn_id);
        recursion_stack.insert(txn_id);
        current_path.push(txn_id);

        if let Some(edges) = self.edges.get(&txn_id) {
            for edge in edges {
                if !visited.contains(&edge.holder) {
                    self.dfs_detect_cycle(edge.holder, visited, recursion_stack, current_path, deadlocks);
                } else if recursion_stack.contains(&edge.holder) {
                    // Found a cycle
                    if let Some(cycle_start) = current_path.iter().position(|&x| x == edge.holder) {
                        let cycle_transactions = current_path[cycle_start..].to_vec();
                        let cycle_resources = self.get_resources_in_cycle(&cycle_transactions);

                        let deadlock = DeadlockCycle {
                            transactions: cycle_transactions,
                            resources: cycle_resources,
                            detection_time: Instant::now(),
                        };

                        deadlocks.push(deadlock);
                    }
                }
            }
        }

        current_path.pop();
        recursion_stack.remove(&txn_id);
    }

    /// Get resources involved in a cycle
    fn get_resources_in_cycle(&self, transactions: &[TransactionId]) -> Vec<PageId> {
        let mut resources = HashSet::new();

        for &txn_id in transactions {
            if let Some(edges) = self.edges.get(&txn_id) {
                for edge in edges {
                    if transactions.contains(&edge.holder) {
                        resources.insert(edge.resource);
                    }
                }
            }
        }

        resources.into_iter().collect()
    }

    /// Update transaction metadata
    pub fn update_transaction_metadata(&mut self, txn_id: TransactionId, resources_held: usize) {
        if let Some(metadata) = self.transaction_metadata.get_mut(&txn_id) {
            metadata.resources_held = resources_held;
        }
    }

    /// Get transaction with least resources in a cycle
    pub fn get_least_resources_transaction(&self, transactions: &[TransactionId]) -> TransactionId {
        transactions
            .iter()
            .min_by_key(|&&txn_id| self.transaction_metadata.get(&txn_id).map(|m| m.resources_held).unwrap_or(0))
            .copied()
            .unwrap_or(transactions[0])
    }

    /// Get transaction with longest wait time in a cycle
    pub fn get_longest_waiting_transaction(&self, transactions: &[TransactionId]) -> TransactionId {
        transactions
            .iter()
            .max_by_key(|&&txn_id| self.transaction_metadata.get(&txn_id).map(|m| m.total_wait_time).unwrap_or_default())
            .copied()
            .unwrap_or(transactions[0])
    }

    /// Get number of active edges
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }

    /// Check if the graph has any edges
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl Default for WaitForGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Deadlock detector that monitors the wait-for graph
pub struct DeadlockDetector {
    /// Wait-for graph
    wait_for_graph: RwLock<WaitForGraph>,
    /// Detection policy
    resolution_policy: DeadlockResolutionPolicy,
    /// Statistics
    statistics: Mutex<DeadlockStatistics>,
    /// Detection interval
    detection_interval: Duration,
    /// Maximum wait time before forced deadlock check
    max_wait_time: Duration,
}

impl DeadlockDetector {
    /// Create a new deadlock detector
    pub fn new(resolution_policy: DeadlockResolutionPolicy, detection_interval: Duration, max_wait_time: Duration) -> Self {
        Self {
            wait_for_graph: RwLock::new(WaitForGraph::new()),
            resolution_policy,
            statistics: Mutex::new(DeadlockStatistics::default()),
            detection_interval,
            max_wait_time,
        }
    }

    /// Add a wait-for relationship
    pub fn add_wait_edge(&self, waiter: TransactionId, holder: TransactionId, resource: PageId) {
        let edge = WaitForEdge::new(waiter, holder, resource);
        let mut graph = self.wait_for_graph.write().unwrap();
        graph.add_edge(edge);

        // Update statistics
        let mut stats = self.statistics.lock().unwrap();
        stats.active_wait_edges = graph.edge_count();
    }

    /// Remove a wait-for relationship
    pub fn remove_wait_edge(&self, waiter: TransactionId, holder: TransactionId) {
        let mut graph = self.wait_for_graph.write().unwrap();
        graph.remove_edge(waiter, holder);

        // Update statistics
        let mut stats = self.statistics.lock().unwrap();
        stats.active_wait_edges = graph.edge_count();
    }

    /// Remove all wait relationships for a transaction
    pub fn remove_transaction(&self, txn_id: TransactionId) {
        let mut graph = self.wait_for_graph.write().unwrap();
        graph.remove_transaction(txn_id);

        // Update statistics
        let mut stats = self.statistics.lock().unwrap();
        stats.active_wait_edges = graph.edge_count();
    }

    /// Run deadlock detection and return transactions to abort
    pub fn detect_and_resolve_deadlocks(&self) -> StorageResult<Vec<TransactionId>> {
        let detection_start = Instant::now();
        let graph = self.wait_for_graph.read().unwrap();
        let deadlocks = graph.detect_deadlocks();
        let detection_time = detection_start.elapsed();

        if deadlocks.is_empty() {
            return Ok(Vec::new());
        }

        let mut transactions_to_abort = Vec::new();
        let mut stats = self.statistics.lock().unwrap();

        for deadlock in deadlocks {
            stats.total_deadlocks_detected += 1;

            // Choose transaction to abort based on policy
            let victim = self.choose_victim(&deadlock, &graph);
            transactions_to_abort.push(victim);
            stats.total_transactions_aborted += 1;
        }

        // Update detection time statistics
        let detection_time_us = detection_time.as_micros() as u64;
        if stats.average_detection_time_us == 0 {
            stats.average_detection_time_us = detection_time_us;
        } else {
            stats.average_detection_time_us = (stats.average_detection_time_us + detection_time_us) / 2;
        }

        Ok(transactions_to_abort)
    }

    /// Choose victim transaction based on resolution policy
    fn choose_victim(&self, deadlock: &DeadlockCycle, graph: &WaitForGraph) -> TransactionId {
        match self.resolution_policy {
            DeadlockResolutionPolicy::AbortYoungest => deadlock.youngest_transaction(),
            DeadlockResolutionPolicy::AbortOldest => deadlock.oldest_transaction(),
            DeadlockResolutionPolicy::AbortLeastResources => graph.get_least_resources_transaction(&deadlock.transactions),
            DeadlockResolutionPolicy::AbortLongestWaiting => graph.get_longest_waiting_transaction(&deadlock.transactions),
        }
    }

    /// Check for deadlocks involving long-waiting transactions
    pub fn check_long_waiting_transactions(&self) -> StorageResult<Vec<TransactionId>> {
        let graph = self.wait_for_graph.read().unwrap();
        let mut long_waiters = Vec::new();

        for edges in graph.edges.values() {
            for edge in edges {
                if edge.wait_duration() > self.max_wait_time {
                    long_waiters.push(edge.waiter);
                }
            }
        }

        if !long_waiters.is_empty() {
            // Force deadlock detection for these transactions
            self.detect_and_resolve_deadlocks()
        } else {
            Ok(Vec::new())
        }
    }

    /// Update transaction metadata for better deadlock resolution
    pub fn update_transaction_metadata(&self, txn_id: TransactionId, resources_held: usize) {
        let mut graph = self.wait_for_graph.write().unwrap();
        graph.update_transaction_metadata(txn_id, resources_held);
    }

    /// Get current deadlock statistics
    pub fn get_statistics(&self) -> DeadlockStatistics {
        let stats = self.statistics.lock().unwrap();
        let mut result = stats.clone();

        // Update max wait time
        let graph = self.wait_for_graph.read().unwrap();
        let max_wait_ms = graph
            .edges
            .values()
            .flat_map(|edges| edges.iter())
            .map(|edge| edge.wait_duration().as_millis() as u64)
            .max()
            .unwrap_or(0);

        result.max_wait_time_ms = max_wait_ms;
        result
    }

    /// Get detection interval
    pub fn detection_interval(&self) -> Duration {
        self.detection_interval
    }

    /// Get maximum wait time
    pub fn max_wait_time(&self) -> Duration {
        self.max_wait_time
    }

    /// Check if deadlock detection is needed
    pub fn should_detect_deadlocks(&self) -> bool {
        let graph = self.wait_for_graph.read().unwrap();
        !graph.is_empty()
    }
}

impl Default for DeadlockDetector {
    fn default() -> Self {
        Self::new(
            DeadlockResolutionPolicy::AbortYoungest,
            Duration::from_millis(100), // Check every 100ms
            Duration::from_secs(5),     // Max wait time of 5 seconds
        )
    }
}

/// Deadlock detection service that runs periodically
pub struct DeadlockDetectionService {
    /// The deadlock detector
    detector: Arc<DeadlockDetector>,
    /// Callback to abort transactions
    abort_callback: Box<dyn Fn(TransactionId) -> StorageResult<()> + Send + Sync>,
    /// Whether the service is running
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl DeadlockDetectionService {
    /// Create a new deadlock detection service
    pub fn new<F>(detector: Arc<DeadlockDetector>, abort_callback: F) -> Self
    where
        F: Fn(TransactionId) -> StorageResult<()> + Send + Sync + 'static,
    {
        Self {
            detector,
            abort_callback: Box::new(abort_callback),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the deadlock detection service
    pub fn start(&self) -> StorageResult<()> {
        if self.is_running.load(std::sync::atomic::Ordering::Acquire) {
            return Err(StorageError::InvalidOperation("Service already running".to_string()));
        }

        self.is_running.store(true, std::sync::atomic::Ordering::Release);

        let detector = self.detector.clone();
        let is_running = self.is_running.clone();
        let detection_interval = detector.detection_interval();

        std::thread::spawn(move || {
            while is_running.load(std::sync::atomic::Ordering::Acquire) {
                if detector.should_detect_deadlocks() {
                    if let Ok(victims) = detector.detect_and_resolve_deadlocks() {
                        for victim in victims {
                            // In a real implementation, we would call the abort callback here
                            eprintln!("Deadlock detected: aborting transaction {victim}");
                        }
                    }

                    // Also check for long-waiting transactions
                    if let Ok(long_waiters) = detector.check_long_waiting_transactions() {
                        for waiter in long_waiters {
                            eprintln!("Long-waiting transaction detected: {waiter}");
                        }
                    }
                }

                std::thread::sleep(detection_interval);
            }
        });

        Ok(())
    }

    /// Stop the deadlock detection service
    pub fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Release);
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_for_edge() {
        let edge = WaitForEdge::new(1, 2, PageId(100));
        assert_eq!(edge.waiter, 1);
        assert_eq!(edge.holder, 2);
        assert_eq!(edge.resource, PageId(100));
        assert!(edge.wait_duration().as_nanos() > 0);
    }

    #[test]
    fn test_deadlock_cycle() {
        let cycle = DeadlockCycle {
            transactions: vec![1, 3, 2],
            resources: vec![PageId(100), PageId(200)],
            detection_time: Instant::now(),
        };

        assert_eq!(cycle.youngest_transaction(), 3);
        assert_eq!(cycle.oldest_transaction(), 1);
        assert!(cycle.contains_transaction(2));
        assert!(!cycle.contains_transaction(4));
    }

    #[test]
    fn test_wait_for_graph_basic_operations() {
        let mut graph = WaitForGraph::new();

        // Add edges
        graph.add_edge(WaitForEdge::new(1, 2, PageId(100)));
        graph.add_edge(WaitForEdge::new(2, 3, PageId(200)));

        assert_eq!(graph.edge_count(), 2);
        assert!(!graph.is_empty());

        // Remove edge
        graph.remove_edge(1, 2);
        assert_eq!(graph.edge_count(), 1);

        // Remove transaction
        graph.remove_transaction(2);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.is_empty());
    }

    #[test]
    fn test_simple_deadlock_detection() {
        let mut graph = WaitForGraph::new();

        // Create a simple cycle: 1 -> 2 -> 3 -> 1
        graph.add_edge(WaitForEdge::new(1, 2, PageId(100)));
        graph.add_edge(WaitForEdge::new(2, 3, PageId(200)));
        graph.add_edge(WaitForEdge::new(3, 1, PageId(300)));

        let deadlocks = graph.detect_deadlocks();
        assert_eq!(deadlocks.len(), 1);

        let deadlock = &deadlocks[0];
        assert_eq!(deadlock.transactions.len(), 3);
        assert!(deadlock.contains_transaction(1));
        assert!(deadlock.contains_transaction(2));
        assert!(deadlock.contains_transaction(3));
    }

    #[test]
    fn test_no_deadlock_detection() {
        let mut graph = WaitForGraph::new();

        // Create a non-cyclic wait chain: 1 -> 2 -> 3
        graph.add_edge(WaitForEdge::new(1, 2, PageId(100)));
        graph.add_edge(WaitForEdge::new(2, 3, PageId(200)));

        let deadlocks = graph.detect_deadlocks();
        assert_eq!(deadlocks.len(), 0);
    }

    #[test]
    fn test_deadlock_detector() {
        let detector = DeadlockDetector::new(DeadlockResolutionPolicy::AbortYoungest, Duration::from_millis(10), Duration::from_secs(1));

        // Add wait edges to create a cycle
        detector.add_wait_edge(1, 2, PageId(100));
        detector.add_wait_edge(2, 3, PageId(200));
        detector.add_wait_edge(3, 1, PageId(300));

        // Detect deadlocks
        let victims = detector.detect_and_resolve_deadlocks().unwrap();
        assert_eq!(victims.len(), 1);
        assert_eq!(victims[0], 3); // Should abort youngest (highest ID)

        // Check statistics
        let stats = detector.get_statistics();
        assert_eq!(stats.total_deadlocks_detected, 1);
        assert_eq!(stats.total_transactions_aborted, 1);
    }

    #[test]
    fn test_deadlock_resolution_policies() {
        let mut graph = WaitForGraph::new();

        // Setup cycle with different resource counts
        graph.add_edge(WaitForEdge::new(1, 2, PageId(100)));
        graph.add_edge(WaitForEdge::new(2, 3, PageId(200)));
        graph.add_edge(WaitForEdge::new(3, 1, PageId(300)));

        // Update metadata
        graph.update_transaction_metadata(1, 5); // Most resources
        graph.update_transaction_metadata(2, 1); // Least resources
        graph.update_transaction_metadata(3, 3); // Medium resources

        let deadlock = DeadlockCycle {
            transactions: vec![1, 2, 3],
            resources: vec![PageId(100), PageId(200), PageId(300)],
            detection_time: Instant::now(),
        };

        // Test different policies
        assert_eq!(deadlock.youngest_transaction(), 3);
        assert_eq!(deadlock.oldest_transaction(), 1);
        assert_eq!(graph.get_least_resources_transaction(&deadlock.transactions), 2);
    }

    #[test]
    fn test_long_waiting_transactions() {
        let detector = DeadlockDetector::new(
            DeadlockResolutionPolicy::AbortYoungest,
            Duration::from_millis(10),
            Duration::from_millis(1), // Very short max wait time for testing
        );

        // Add a wait edge
        detector.add_wait_edge(1, 2, PageId(100));

        // Wait a bit to exceed max wait time
        std::thread::sleep(Duration::from_millis(2));

        // Check long waiting transactions
        let long_waiters = detector.check_long_waiting_transactions().unwrap();
        // Since there's no actual cycle, should return empty
        assert_eq!(long_waiters.len(), 0);
    }

    #[test]
    fn test_deadlock_detection_service() {
        let detector = Arc::new(DeadlockDetector::new(DeadlockResolutionPolicy::AbortYoungest, Duration::from_millis(10), Duration::from_secs(1)));

        let service = DeadlockDetectionService::new(detector.clone(), |_txn_id| Ok(()));

        assert!(!service.is_running());

        service.start().unwrap();
        assert!(service.is_running());

        // Should fail to start again
        assert!(service.start().is_err());

        service.stop();
        // Give it a moment to stop
        std::thread::sleep(Duration::from_millis(20));
    }

    #[test]
    fn test_deadlock_statistics() {
        let detector = DeadlockDetector::new(DeadlockResolutionPolicy::AbortYoungest, Duration::from_millis(10), Duration::from_secs(1));

        let initial_stats = detector.get_statistics();
        assert_eq!(initial_stats.total_deadlocks_detected, 0);
        assert_eq!(initial_stats.active_wait_edges, 0);

        // Add some wait edges
        detector.add_wait_edge(1, 2, PageId(100));
        detector.add_wait_edge(2, 3, PageId(200));

        let stats = detector.get_statistics();
        assert_eq!(stats.active_wait_edges, 2);

        // Create deadlock and detect
        detector.add_wait_edge(3, 1, PageId(300));
        let _victims = detector.detect_and_resolve_deadlocks().unwrap();

        let final_stats = detector.get_statistics();
        assert_eq!(final_stats.total_deadlocks_detected, 1);
        assert!(final_stats.average_detection_time_us > 0);
    }

    #[test]
    fn test_complex_deadlock_scenario() {
        let mut graph = WaitForGraph::new();

        // Create multiple overlapping cycles
        // Cycle 1: 1 -> 2 -> 1
        graph.add_edge(WaitForEdge::new(1, 2, PageId(100)));
        graph.add_edge(WaitForEdge::new(2, 1, PageId(200)));

        // Cycle 2: 3 -> 4 -> 5 -> 3
        graph.add_edge(WaitForEdge::new(3, 4, PageId(300)));
        graph.add_edge(WaitForEdge::new(4, 5, PageId(400)));
        graph.add_edge(WaitForEdge::new(5, 3, PageId(500)));

        let deadlocks = graph.detect_deadlocks();
        assert!(deadlocks.len() >= 2); // Should detect both cycles
    }
}
