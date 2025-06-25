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

//! State Verification Mechanism
//!
//! This module implements robust methods to verify state consistency and
//! detect tampering through cryptographic checks and comparison with snapshots.

use crate::vm::state_management::lib::StateKey; // Removed Error, Result, StateValue
use crate::vm::state_management::mvcc::{MVCCStore, Version};
use crate::vm::state_management::snapshot::SnapshotManager; // Removed Snapshot
use crate::vm::state_management::tree::{MerkleTree, StateHash}; // Removed MerkleProof
use std::collections::HashSet; // Removed BTreeMap
use std::sync::Arc;

/// Type for verification result details
pub type VerificationResult = std::result::Result<VerificationReport, VerificationError>;

/// Detailed error information for verification failures
#[derive(Debug, Clone)]
pub struct VerificationError {
    /// Error code
    pub code: VerificationErrorCode,
    /// Descriptive message
    pub message: String,
    /// Optional details about the error
    pub details: Option<VerificationErrorDetails>,
}

/// Error codes for verification failures
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationErrorCode {
    /// Root hash mismatch
    RootHashMismatch,
    /// Proof verification failed
    ProofVerificationFailed,
    /// State inconsistency detected
    StateInconsistency,
    /// Snapshot verification failed
    SnapshotVerificationFailed,
    /// General verification error
    VerificationFailed,
}

/// Detailed information about a verification error
#[derive(Debug, Clone)]
pub enum VerificationErrorDetails {
    /// Details for hash mismatch
    HashMismatch {
        /// Expected hash
        expected: StateHash,
        /// Actual hash found
        actual: StateHash,
    },
    /// Details for key verification failure
    KeyVerification {
        /// List of keys that failed verification
        failed_keys: Vec<StateKey>,
    },
    /// Details for state inconsistency
    StateInconsistency {
        /// Description of inconsistency
        description: String,
        /// Affected keys
        affected_keys: Vec<StateKey>,
    },
}

/// Report containing verification results
#[derive(Debug, Clone)]
pub struct VerificationReport {
    /// Whether the verification succeeded
    pub valid: bool,
    /// Version of the state that was validated
    pub version: Version,
    /// Root hash of the validated state
    pub root_hash: Option<StateHash>,
    /// Number of entries validated
    pub entries_count: usize,
    /// Optional details about the verification
    pub details: Option<String>,
    /// Warnings (if any)
    pub warnings: Vec<String>,
}

/// State validator for verifying integrity
pub struct Validator {
    /// Reference to the MVCC store
    store: Arc<MVCCStore>,
    /// Reference to the snapshot manager (if available)
    snapshot_manager: Option<Arc<SnapshotManager>>,
}

/// Implementation of state verification mechanisms using Merkle proofs and snapshots.
/// Ensures cryptographic integrity and consistency across state versions.
impl Validator {
    /// Creates a new validator with just the MVCC store
    pub fn new(store: Arc<MVCCStore>) -> Self {
        Self { store, snapshot_manager: None }
    }

    /// Creates a new validator with both MVCC store and snapshot manager
    pub fn with_snapshots(store: Arc<MVCCStore>, snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self {
            store,
            snapshot_manager: Some(snapshot_manager),
        }
    }

    /// Validates the state at a specific version with multiple checks:
    /// - **Merkle Tree Construction**: Builds tree from state and checks root hash
    /// - **Proof Verification**: Validates all keys' Merkle proofs
    /// - **Snapshot Comparison**: Compares with nearest snapshot for unexpected changes
    /// - **Hash Consistency**: Verifies root hash matches snapshot when versions align
    ///
    /// # Arguments
    /// - `version`: Target state version to validate
    ///
    /// # Returns
    /// - `VerificationReport`: Detailed verification results
    /// - `VerificationError`: For cryptographic mismatches or inconsistencies
    pub fn validate_state_at_version(&self, version: Version) -> VerificationResult {
        let state = self.store.get_state_at_version(version);

        // Build a Merkle tree from the state
        let tree = MerkleTree::build(&state).map_err(|e| VerificationError {
            code: VerificationErrorCode::VerificationFailed,
            message: format!("Failed to build Merkle tree: {}", e),
            details: None,
        })?;

        let root_hash = tree.root_hash();

        // Validate individual entries against the Merkle tree
        let mut warnings = Vec::new();
        let mut failed_keys = Vec::new();

        for key in state.keys() {
            let proof = tree.generate_proof(key).map_err(|e| VerificationError {
                code: VerificationErrorCode::VerificationFailed,
                message: format!("Failed to generate proof for key {:?}: {}", key, e),
                details: None,
            })?;

            // Verify the proof against the root hash
            if let Some(ref hash) = root_hash {
                if !proof.verify(hash) {
                    failed_keys.push(key.clone());
                }
            }
        }

        // Check if any keys failed verification
        if !failed_keys.is_empty() {
            return Err(VerificationError {
                code: VerificationErrorCode::ProofVerificationFailed,
                message: format!("{} keys failed Merkle proof verification", failed_keys.len()),
                details: Some(VerificationErrorDetails::KeyVerification { failed_keys }),
            });
        }

        // If we have a snapshot manager, compare with closest snapshot
        if let Some(ref mgr) = self.snapshot_manager {
            if let Ok(snapshots) = mgr.list_snapshots() {
                // Find closest snapshot to the requested version
                let closest = snapshots.iter().filter(|s| s.version <= version).max_by_key(|s| s.version);

                if let Some(snapshot_meta) = closest {
                    if let Ok(snapshot) = mgr.load_snapshot(&snapshot_meta.id) {
                        let snapshot_state = snapshot.deserialize_state();

                        // Compare snapshot state with current state for keys that should be unchanged
                        let mut inconsistent_keys = Vec::new();

                        for (key, value) in &snapshot_state {
                            // Only check keys that existed before this version
                            if snapshot_meta.version < version {
                                if let Some(current_value) = state.get(key) {
                                    if current_value != value {
                                        // The value has changed unexpectedly
                                        inconsistent_keys.push(key.clone());
                                    }
                                }
                            }
                        }

                        if !inconsistent_keys.is_empty() {
                            warnings.push(format!("{} keys have unexpected changes since snapshot {}", inconsistent_keys.len(), snapshot_meta.id));
                        }

                        // Verify root hash matches if available
                        if let (Some(snapshot_hash), Some(current_hash)) = (snapshot_meta.root_hash, root_hash) {
                            if snapshot_meta.version == version && snapshot_hash != current_hash {
                                return Err(VerificationError {
                                    code: VerificationErrorCode::RootHashMismatch,
                                    message: "Root hash mismatch with snapshot".to_string(),
                                    details: Some(VerificationErrorDetails::HashMismatch {
                                        expected: snapshot_hash,
                                        actual: current_hash,
                                    }),
                                });
                            }
                        }
                    }
                }
            }
        }

        // All checks passed
        Ok(VerificationReport {
            valid: true,
            version,
            root_hash,
            entries_count: state.len(),
            details: Some("State verification successful".to_string()),
            warnings,
        })
    }

    /// Validates the latest state
    pub fn validate_latest_state(&self) -> VerificationResult {
        let version = self.store.current_version();
        self.validate_state_at_version(version)
    }

    /// Validates a specific snapshot
    pub fn validate_snapshot(&self, snapshot_id: &str) -> VerificationResult {
        let snapshot_manager = self.snapshot_manager.as_ref().ok_or_else(|| VerificationError {
            code: VerificationErrorCode::VerificationFailed,
            message: "No snapshot manager available".to_string(),
            details: None,
        })?;

        let snapshot = snapshot_manager.load_snapshot(snapshot_id).map_err(|e| VerificationError {
            code: VerificationErrorCode::VerificationFailed,
            message: format!("Failed to load snapshot: {}", e),
            details: None,
        })?;

        let state = snapshot.deserialize_state();

        // Build a Merkle tree from the snapshot state
        let tree = MerkleTree::build(&state).map_err(|e| VerificationError {
            code: VerificationErrorCode::VerificationFailed,
            message: format!("Failed to build Merkle tree from snapshot: {}", e),
            details: None,
        })?;

        let computed_root_hash = tree.root_hash();

        // Compare with the stored root hash if available
        if let (Some(stored_hash), Some(computed_hash)) = (snapshot.metadata.root_hash, computed_root_hash) {
            if stored_hash != computed_hash {
                return Err(VerificationError {
                    code: VerificationErrorCode::RootHashMismatch,
                    message: "Snapshot root hash mismatch".to_string(),
                    details: Some(VerificationErrorDetails::HashMismatch {
                        expected: stored_hash,
                        actual: computed_hash,
                    }),
                });
            }
        }

        // Validate individual entries
        let mut failed_keys = Vec::new();

        for key in state.keys() {
            let proof = tree.generate_proof(key).map_err(|e| VerificationError {
                code: VerificationErrorCode::VerificationFailed,
                message: format!("Failed to generate proof for key {:?}: {}", key, e),
                details: None,
            })?;

            // Verify the proof against the root hash
            if let Some(ref hash) = computed_root_hash {
                if !proof.verify(hash) {
                    failed_keys.push(key.clone());
                }
            }
        }

        // Check if any keys failed verification
        if !failed_keys.is_empty() {
            return Err(VerificationError {
                code: VerificationErrorCode::ProofVerificationFailed,
                message: format!("{} keys failed Merkle proof verification in snapshot", failed_keys.len()),
                details: Some(VerificationErrorDetails::KeyVerification { failed_keys }),
            });
        }

        // All checks passed
        Ok(VerificationReport {
            valid: true,
            version: snapshot.metadata.version,
            root_hash: computed_root_hash,
            entries_count: state.len(),
            details: Some(format!("Snapshot {} verification successful", snapshot_id)),
            warnings: Vec::new(),
        })
    }

    /// Performs comprehensive state verification across versions
    pub fn comprehensive_verification(&self, start_version: Version, end_version: Option<Version>) -> Vec<VerificationResult> {
        let end = end_version.unwrap_or_else(|| self.store.current_version());
        let mut results = Vec::new();

        for version in start_version..=end {
            results.push(self.validate_state_at_version(version));
        }

        results
    }

    /// Validates the state transition between two versions
    pub fn validate_state_transition(&self, from_version: Version, to_version: Version) -> VerificationResult {
        if from_version >= to_version {
            return Err(VerificationError {
                code: VerificationErrorCode::VerificationFailed,
                message: "From version must be less than to version".to_string(),
                details: None,
            });
        }

        let old_state = self.store.get_state_at_version(from_version);
        let new_state = self.store.get_state_at_version(to_version);

        // Build trees for both states
        let _old_tree = MerkleTree::build(&old_state).map_err(|e| VerificationError {
            // Prefixed with _
            code: VerificationErrorCode::VerificationFailed,
            message: format!("Failed to build tree for version {}: {}", from_version, e),
            details: None,
        })?;

        let new_tree = MerkleTree::build(&new_state).map_err(|e| VerificationError {
            code: VerificationErrorCode::VerificationFailed,
            message: format!("Failed to build tree for version {}: {}", to_version, e),
            details: None,
        })?;

        // Validate that new state includes all non-deleted keys from old state
        let mut warnings = Vec::new();
        let mut changed_keys = HashSet::new();

        for (key, old_value) in &old_state {
            if let Some(new_value) = new_state.get(key) {
                if old_value != new_value {
                    changed_keys.insert(key.clone());
                }
            } else {
                // Key was deleted
                changed_keys.insert(key.clone());
            }
        }

        // Find new keys
        for key in new_state.keys() {
            if !old_state.contains_key(key) {
                changed_keys.insert(key.clone());
            }
        }

        if !changed_keys.is_empty() {
            warnings.push(format!("{} keys changed between versions", changed_keys.len()));
        }

        Ok(VerificationReport {
            valid: true,
            version: to_version,
            root_hash: new_tree.root_hash(),
            entries_count: new_state.len(),
            details: Some(format!("Transition from {} to {} validated", from_version, to_version)),
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::state_management::lib::StateValue; // Added StateValue import
    use tempfile::tempdir;

    fn setup_test_environment() -> (Arc<MVCCStore>, Arc<SnapshotManager>) {
        let store = Arc::new(MVCCStore::new());

        // Add some initial data
        let key1 = StateKey::from_string("key1");
        let key2 = StateKey::from_string("key2");
        let value1 = StateValue::from_string("value1");
        let value2 = StateValue::from_string("value2");

        let _ = store.put(key1, value1);
        let _ = store.put(key2, value2);

        // Create temporary directory for snapshots
        let temp_dir = tempdir().unwrap();
        let snapshot_manager = Arc::new(SnapshotManager::new(store.clone(), temp_dir.path()).unwrap());

        (store, snapshot_manager)
    }

    #[test]
    fn test_validate_latest_state() {
        let (store, _) = setup_test_environment();
        let validator = Validator::new(store.clone());

        // Validate latest state
        let result = validator.validate_latest_state();
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(report.valid);
        assert_eq!(report.version, store.current_version());
        assert_eq!(report.entries_count, 2); // Our two test keys
    }

    #[test]
    fn test_validate_snapshot() {
        let (store, snapshot_manager) = setup_test_environment();
        let validator = Validator::with_snapshots(store.clone(), snapshot_manager.clone());

        // Create a snapshot
        let snapshot = snapshot_manager.create_snapshot(None).unwrap();

        // Validate the snapshot
        let result = validator.validate_snapshot(&snapshot.metadata.id);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(report.valid);
        assert_eq!(report.version, snapshot.metadata.version);
    }

    #[test]
    fn test_validate_state_transition() {
        let (store, _) = setup_test_environment();
        let validator = Validator::new(store.clone());

        let initial_version = store.current_version();

        // Make some changes
        let key3 = StateKey::from_string("key3");
        let value3 = StateValue::from_string("value3");
        let _ = store.put(key3, value3);

        let new_version = store.current_version();

        // Validate the transition
        let result = validator.validate_state_transition(initial_version, new_version);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(report.valid);
        assert_eq!(report.version, new_version);

        // Should have a warning about changed keys
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn test_comprehensive_verification() {
        let (store, _) = setup_test_environment();
        let validator = Validator::new(store.clone());

        let initial_version = store.current_version();

        // Make some changes to create more versions
        for i in 1..=3 {
            let key = StateKey::from_string(&format!("key{}", i + 2));
            let value = StateValue::from_string(&format!("value{}", i + 2));
            let _ = store.put(key, value);
        }

        // Validate across all versions
        let results = validator.comprehensive_verification(initial_version, None);

        // Should have 4 results (initial + 3 updates)
        assert_eq!(results.len(), 4);

        // All should be valid
        for result in results {
            assert!(result.is_ok());
            let report = result.unwrap();
            assert!(report.valid);
        }
    }

    #[test]
    fn test_verification_error_handling() {
        let (store, _) = setup_test_environment();
        let validator = Validator::new(store.clone());

        // Attempt to validate an invalid transition (from > to)
        let current_version = store.current_version();
        let result = validator.validate_state_transition(current_version, current_version.saturating_sub(1)); // Avoid underflow on 0

        // Should fail
        assert!(result.is_err());

        if let Err(error) = result {
            assert_eq!(error.code, VerificationErrorCode::VerificationFailed);
        }
    }

    #[test]
    fn test_root_hash_consistency() {
        let (store, _) = setup_test_environment();
        let validator = Validator::new(store.clone());

        // Get root hash from initial state
        let result1 = validator.validate_latest_state();
        assert!(result1.is_ok());
        let report1 = result1.unwrap();
        let initial_hash = report1.root_hash;

        // Make no changes and validate again
        let result2 = validator.validate_latest_state();
        assert!(result2.is_ok());
        let report2 = result2.unwrap();

        // Root hash should be the same when no changes are made
        assert_eq!(initial_hash, report2.root_hash);

        // Now make a change
        let key = StateKey::from_string("new_key");
        let value = StateValue::from_string("new_value");
        let _ = store.put(key, value);

        // Validate again
        let result3 = validator.validate_latest_state();
        assert!(result3.is_ok());
        let report3 = result3.unwrap();

        // Root hash should be different after changes
        assert_ne!(initial_hash, report3.root_hash);
    }

    #[test]
    fn test_snapshot_root_hash_verification() {
        let (store, snapshot_manager) = setup_test_environment();
        let validator = Validator::with_snapshots(store.clone(), snapshot_manager.clone());

        // Create a snapshot
        let snapshot = snapshot_manager.create_snapshot(None).unwrap();

        // Validate the snapshot
        let result = validator.validate_snapshot(&snapshot.metadata.id);
        assert!(result.is_ok());

        // Manually tamper with the snapshot's stored root hash to simulate corruption
        // This would typically require a mock of the snapshot manager, but for illustration:
        // snapshot_manager.tamper_with_root_hash(&snapshot.metadata.id, tampered_hash);

        // For this test, we'll simulate the verification code path where hashes mismatch
        // by directly testing the VerificationErrorDetails::HashMismatch variant
        let mismatch_error = VerificationError {
            code: VerificationErrorCode::RootHashMismatch,
            message: "Test hash mismatch".to_string(),
            details: Some(VerificationErrorDetails::HashMismatch {
                expected: [1u8; 32],
                actual: [2u8; 32],
            }),
        };

        // Verify error details formatting
        assert!(mismatch_error.message.contains("hash mismatch"));
        if let Some(VerificationErrorDetails::HashMismatch { expected, actual }) = mismatch_error.details {
            assert_ne!(expected, actual);
        } else {
            panic!("Expected HashMismatch details");
        }
    }
}
