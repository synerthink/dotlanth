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

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Error type for Snapshot management operations.
#[derive(Debug, PartialEq)]
pub enum SnapshotError {
    /// The specified snapshot ID does not exist.
    SnapshotNotFound,
    /// An error occurred during snapshot capture or restoration.
    OperationFailed(String),
    /// Operation not implemented.
    NotImplemented,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotError::SnapshotNotFound => write!(f, "Snapshot not found"),
            SnapshotError::OperationFailed(msg) => write!(f, "Operation failed: {msg}"),
            SnapshotError::NotImplemented => write!(f, "NotImplemented"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// A Snapshot wraps a snapshot identifier along with a clone of state data.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot<T>
where
    T: Clone,
{
    /// Unique identifier for the snapshot.
    pub id: u32,
    /// The snapshot’s state.
    pub state: T,
}

/// Trait for managing state snapshots. T represents the type of state to capture.
/// T must implement Clone so that snapshots hold independent copies.
pub trait SnapshotManager<T>
where
    T: Clone,
{
    /// Captures a snapshot of the given state and returns a unique snapshot ID.
    ///
    /// # Arguments
    ///
    /// * `state` - A reference to the current state.
    ///
    /// # Returns
    ///
    /// * On success, returns the unique ID of the snapshot.
    fn capture_snapshot(&mut self, state: &T) -> Result<u32, SnapshotError>;

    /// Restores and returns the state associated with the given snapshot ID.
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - The unique identifier of a previously captured snapshot.
    ///
    /// # Returns
    ///
    /// * On success, returns a clone of the state captured in the snapshot.
    fn restore_snapshot(&self, snapshot_id: u32) -> Result<T, SnapshotError>;

    /// Cleans up (deletes) snapshots that are considered outdated.
    ///
    /// The criteria for retention are determined by the implementation.
    ///
    /// # Returns
    ///
    /// * The number of snapshots that were removed.
    fn cleanup_snapshots(&mut self) -> Result<usize, SnapshotError>;
}

/// DefaultSnapshotManager is our production implementation of the SnapshotManager trait.
/// Its methods are left unimplemented (using unimplemented!()) so that TDD tests drive its development.
pub struct DefaultSnapshotManager<T>
where
    T: Clone,
{
    /// Stores snapshots keyed by their unique ID.
    snapshots: Arc<Mutex<BTreeMap<u32, Snapshot<T>>>>,
    /// A simple counter to generate unique snapshot IDs.
    next_id: u32,
    /// A retention threshold – for example, we might only keep the last N snapshots.
    retention_limit: usize,
}

impl<T> DefaultSnapshotManager<T>
where
    T: Clone,
{
    /// Creates a new instance of DefaultSnapshotManager with a given retention limit.
    pub fn new(retention_limit: usize) -> Self {
        DefaultSnapshotManager {
            snapshots: Arc::new(Mutex::new(BTreeMap::new())),
            next_id: 1,
            retention_limit,
        }
    }
}

impl<T> SnapshotManager<T> for DefaultSnapshotManager<T>
where
    T: Clone,
{
    fn capture_snapshot(&mut self, state: &T) -> Result<u32, SnapshotError> {
        let mut snapshots_guard = self.snapshots.lock().map_err(|_| SnapshotError::OperationFailed("Mutex poisoned".into()))?;
        let snapshot_id = self.next_id;
        let snapshot = Snapshot {
            id: snapshot_id,
            state: state.clone(),
        };
        snapshots_guard.insert(snapshot_id, snapshot);
        self.next_id += 1;
        Ok(snapshot_id)
    }

    fn restore_snapshot(&self, snapshot_id: u32) -> Result<T, SnapshotError> {
        let snapshots_guard = self.snapshots.lock().map_err(|_| SnapshotError::OperationFailed("Mutex poisoned".into()))?;
        if let Some(snapshot) = snapshots_guard.get(&snapshot_id) {
            Ok(snapshot.state.clone())
        } else {
            Err(SnapshotError::SnapshotNotFound)
        }
    }

    fn cleanup_snapshots(&mut self) -> Result<usize, SnapshotError> {
        let mut snapshots_guard = self.snapshots.lock().map_err(|_| SnapshotError::OperationFailed("Mutex poisoned".into()))?;
        let total = snapshots_guard.len();
        if total <= self.retention_limit {
            return Ok(0);
        }
        let remove_count = total - self.retention_limit;
        // Since BTreeMap keys are ordered, we remove the smallest (oldest) snapshots.
        let keys_to_remove: Vec<u32> = snapshots_guard.keys().cloned().take(remove_count).collect();
        for key in keys_to_remove {
            snapshots_guard.remove(&key);
        }
        Ok(remove_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A helper test state structure. For demonstration, we use a simple struct.
    #[derive(Debug, Clone, PartialEq)]
    struct TestState {
        pub counter: i32,
        pub info: String,
    }

    /// For testing purposes, we will simulate expected behavior.
    /// The tests below target the production code (DefaultSnapshotManager) and define
    /// the expected outcomes. Since the production code methods are unimplemented,
    /// these tests will initially fail, driving their implementation.
    #[test]
    fn test_capture_and_restore_snapshot() {
        let initial_state = TestState { counter: 100, info: "Initial".into() };

        // Create the snapshot manager with a retention limit of 10.
        let mut manager = DefaultSnapshotManager::new(10);

        // Capture a snapshot of the initial state.
        let snapshot_id = manager.capture_snapshot(&initial_state).expect("Snapshot capture should eventually succeed");
        // Attempt to restore the snapshot.
        let restored_state = manager.restore_snapshot(snapshot_id).expect("Snapshot restoration should succeed");
        // The restored state should equal the initial state.
        assert_eq!(restored_state, initial_state, "Restored state must match initial state");
    }

    #[test]
    fn test_restore_nonexistent_snapshot() {
        let manager = DefaultSnapshotManager::<TestState>::new(10);
        // Attempt to restore snapshot ID 999 which does not exist.
        let result = manager.restore_snapshot(999);
        assert!(result.is_err(), "Restoring a non-existent snapshot should yield an error");
        assert_eq!(result.err().unwrap(), SnapshotError::SnapshotNotFound);
    }

    #[test]
    fn test_cleanup_snapshots_retention_policy() {
        let mut manager = DefaultSnapshotManager::new(3); // Retention limit of 3 snapshots.
        let state = TestState { counter: 42, info: "Snapshot".into() };

        // Capture 5 snapshots.
        for _ in 0..5 {
            let _ = manager.capture_snapshot(&state).expect("Snapshot capture should eventually succeed");
        }
        // Cleanup should remove snapshots in excess of the retention limit.
        let removed = manager.cleanup_snapshots().expect("Cleanup should eventually succeed");
        // For example, if 5 snapshots were captured and limit is 3, then 2 should be removed.
        assert_eq!(removed, 2, "Cleanup should remove snapshots over the retention limit");

        // Check that only 3 snapshots remain by trying to restore those IDs.
        // (Here we assume that snapshot IDs are consecutive starting from 1, and that the cleanup
        // mechanism removes the oldest snapshots.)
        for snapshot_id in 3..=5 {
            let _restored = manager.restore_snapshot(snapshot_id).expect("Snapshot within retention limit should be restorable");
        }
        // Snapshot ID 1 and 2 should no longer be available.
        for snapshot_id in 1..=2 {
            let result = manager.restore_snapshot(snapshot_id);
            assert!(result.is_err(), "Old snapshots removed by cleanup should not be restorable");
            assert_eq!(result.err().unwrap(), SnapshotError::SnapshotNotFound);
        }
    }

    #[test]
    fn test_snapshot_independence() {
        let initial_state = TestState { counter: 10, info: "State A".into() };
        let mut manager = DefaultSnapshotManager::new(10);

        // Capture a snapshot.
        let snapshot_id = manager.capture_snapshot(&initial_state).expect("Snapshot capture should eventually succeed");
        // Modify state.
        let modified_state = TestState { counter: 20, info: "State B".into() };
        // Capture a second snapshot.
        let snapshot_id2 = manager.capture_snapshot(&modified_state).expect("Second snapshot capture should eventually succeed");

        // Restore the first snapshot and ensure it is independent of the second.
        let restored_state1 = manager.restore_snapshot(snapshot_id).expect("Restoring first snapshot should succeed");
        assert_eq!(restored_state1, initial_state, "Restored state must equal the state at snapshot time");

        // Restore the second snapshot.
        let restored_state2 = manager.restore_snapshot(snapshot_id2).expect("Restoring second snapshot should succeed");
        assert_eq!(restored_state2, modified_state, "Restored state must equal the modified state at snapshot time");
    }

    /// Additional test: Simulate concurrent snapshot captures.
    /// (Note: This test is a basic simulation; in real-world use, more extensive concurrency tests would be performed.)
    #[test]
    fn test_concurrent_snapshot_capture() {
        use std::thread;

        let state = TestState {
            counter: 7,
            info: "Concurrent".into(),
        };
        let mut manager = DefaultSnapshotManager::new(10);

        // Wrap manager in an Arc<Mutex<>> so that it can be shared across threads.
        let manager = Arc::new(Mutex::new(manager));
        let mut handles = vec![];

        // Launch 5 threads to concurrently capture snapshots.
        for _ in 0..5 {
            let m = Arc::clone(&manager);
            let state_clone = state.clone();
            let handle = thread::spawn(move || {
                let mut mgr = m.lock().unwrap();
                mgr.capture_snapshot(&state_clone).expect("Concurrent snapshot capture should eventually succeed")
            });
            handles.push(handle);
        }

        // Collect results.
        let mut snapshot_ids = vec![];
        for handle in handles {
            snapshot_ids.push(handle.join().unwrap());
        }

        // Verify that each snapshot id is unique and snapshots can be restored.
        let mgr = manager.lock().unwrap();
        for id in snapshot_ids {
            let restored = mgr.restore_snapshot(id).expect("Restored snapshot should equal the original state");
            assert_eq!(restored, state, "Concurrent snapshot restoration must match original state");
        }
    }
}
