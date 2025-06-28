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

//! State Difference Implementation
//!
//! This module provides functionality for computing and managing differences between
//! two Merkle Patricia Trie states. It enables efficient state comparison, change
//! tracking, and state synchronization.
//!
//! # Features
//!
//! - State change tracking (additions, modifications, removals)
//! - Efficient diff computation between states
//! - Forward and reverse diff application
//! - Change summarization and querying
//!
//! # Performance Considerations
//!
//! - Efficient key-value comparison using HashMaps
//! - Minimal memory allocations during diff computation
//! - Optimized change tracking and application
//! - Thread-safe operations

use crate::state::mpt::trie::NodeStorage;
use crate::state::mpt::{Hash, Key, MerklePatriciaTrie, TrieResult, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents different types of changes in state
///
/// This enum captures all possible state changes that can occur between
/// two versions of a Merkle Patricia Trie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateChange {
    /// Key was added with a new value
    Added { key: Key, value: Value },
    /// Key was modified from old_value to new_value
    Modified { key: Key, old_value: Value, new_value: Value },
    /// Key was removed with its last value
    Removed { key: Key, old_value: Value },
}

/// Represents the difference between two state trees
///
/// This struct captures all changes between two states, including the root hashes
/// of both states and a list of all changes that occurred.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateDiff {
    /// Changes organized by type for efficient querying
    pub changes: Vec<StateChange>,
    /// Root hash of the source state
    pub from_root: Hash,
    /// Root hash of the target state
    pub to_root: Hash,
}

impl StateDiff {
    /// Create a new empty state diff
    ///
    /// # Arguments
    ///
    /// * `from_root` - The root hash of the source state
    /// * `to_root` - The root hash of the target state
    ///
    /// # Returns
    ///
    /// A new empty StateDiff instance
    pub fn new(from_root: Hash, to_root: Hash) -> Self {
        Self {
            changes: Vec::new(),
            from_root,
            to_root,
        }
    }

    /// Add a change to the diff
    ///
    /// # Arguments
    ///
    /// * `change` - The state change to add
    pub fn add_change(&mut self, change: StateChange) {
        self.changes.push(change);
    }

    /// Get all added keys
    ///
    /// # Returns
    ///
    /// A vector of references to keys that were added
    pub fn added_keys(&self) -> Vec<&Key> {
        self.changes
            .iter()
            .filter_map(|change| match change {
                StateChange::Added { key, .. } => Some(key),
                _ => None,
            })
            .collect()
    }

    /// Get all modified keys
    ///
    /// # Returns
    ///
    /// A vector of references to keys that were modified
    pub fn modified_keys(&self) -> Vec<&Key> {
        self.changes
            .iter()
            .filter_map(|change| match change {
                StateChange::Modified { key, .. } => Some(key),
                _ => None,
            })
            .collect()
    }

    /// Get all removed keys
    ///
    /// # Returns
    ///
    /// A vector of references to keys that were removed
    pub fn removed_keys(&self) -> Vec<&Key> {
        self.changes
            .iter()
            .filter_map(|change| match change {
                StateChange::Removed { key, .. } => Some(key),
                _ => None,
            })
            .collect()
    }

    /// Check if the diff is empty (no changes)
    ///
    /// # Returns
    ///
    /// True if there are no changes, false otherwise
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Get the total number of changes
    ///
    /// # Returns
    ///
    /// The number of changes in the diff
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// Get changes by type counts
    ///
    /// # Returns
    ///
    /// A tuple containing (added_count, modified_count, removed_count)
    pub fn change_summary(&self) -> (usize, usize, usize) {
        let mut added = 0;
        let mut modified = 0;
        let mut removed = 0;

        for change in &self.changes {
            match change {
                StateChange::Added { .. } => added += 1,
                StateChange::Modified { .. } => modified += 1,
                StateChange::Removed { .. } => removed += 1,
            }
        }

        (added, modified, removed)
    }

    /// Apply this diff to a target trie (forward application)
    ///
    /// # Arguments
    ///
    /// * `from_trie` - The source trie to copy unchanged keys from
    /// * `trie` - The target trie to apply changes to
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    pub fn apply_to<S: NodeStorage>(&self, from_trie: &MerklePatriciaTrie<S>, trie: &mut MerklePatriciaTrie<S>) -> TrieResult<()> {
        use std::collections::HashSet;

        // 1. Collect keys of changes to be applied
        let mut changed_keys = HashSet::new();
        for change in &self.changes {
            changed_keys.insert(change.key().clone());
        }

        // 2. Get all keys from from_trie
        let from_keys = from_trie.get_all_keys()?;

        // 3. Copy unchanged keys
        for key in from_keys {
            if !changed_keys.contains(&key)
                && let Some(value) = from_trie.get(&key)?
            {
                trie.put(key, value)?;
            }
        }

        // 4. Apply changes from diff
        for change in &self.changes {
            match change {
                StateChange::Added { key, value } => {
                    trie.put(key.clone(), value.clone())?;
                }
                StateChange::Modified { key, new_value, .. } => {
                    trie.put(key.clone(), new_value.clone())?;
                }
                StateChange::Removed { key, .. } => {
                    trie.delete(key)?;
                }
            }
        }
        Ok(())
    }

    /// Create reverse diff (to undo this diff)
    ///
    /// # Returns
    ///
    /// A new StateDiff that would undo the changes in this diff
    pub fn reverse(&self) -> StateDiff {
        let mut reversed_changes = Vec::with_capacity(self.changes.len());

        for change in &self.changes {
            let reversed_change = match change {
                StateChange::Added { key, value } => StateChange::Removed {
                    key: key.clone(),
                    old_value: value.clone(),
                },
                StateChange::Modified { key, old_value, new_value } => StateChange::Modified {
                    key: key.clone(),
                    old_value: new_value.clone(),
                    new_value: old_value.clone(),
                },
                StateChange::Removed { key, old_value } => StateChange::Added {
                    key: key.clone(),
                    value: old_value.clone(),
                },
            };
            reversed_changes.push(reversed_change);
        }

        // Reverse the order of changes for proper undo sequence
        reversed_changes.reverse();

        StateDiff {
            changes: reversed_changes,
            from_root: self.to_root,
            to_root: self.from_root,
        }
    }

    /// Get all changes for a specific key
    ///
    /// # Arguments
    ///
    /// * `key` - The key to find changes for
    ///
    /// # Returns
    ///
    /// A vector of references to changes affecting the specified key
    pub fn changes_for_key(&self, key: &Key) -> Vec<&StateChange> {
        self.changes.iter().filter(|change| change.key() == key).collect()
    }

    /// Check if a key was changed in this diff
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Returns
    ///
    /// True if the key was changed, false otherwise
    pub fn has_change_for_key(&self, key: &Key) -> bool {
        self.changes.iter().any(|change| change.key() == key)
    }
}

impl StateChange {
    /// Get the key associated with this change
    ///
    /// # Returns
    ///
    /// A reference to the key that was changed
    pub fn key(&self) -> &Key {
        match self {
            StateChange::Added { key, .. } => key,
            StateChange::Modified { key, .. } => key,
            StateChange::Removed { key, .. } => key,
        }
    }

    /// Get the new value if this change introduces one
    ///
    /// # Returns
    ///
    /// Some reference to the new value if this change introduces one, None otherwise
    pub fn get_new_value(&self) -> Option<&Value> {
        match self {
            StateChange::Added { value, .. } => Some(value),
            StateChange::Modified { new_value, .. } => Some(new_value),
            StateChange::Removed { .. } => None,
        }
    }

    /// Get the old value if this change removes/modifies one
    ///
    /// # Returns
    ///
    /// Some reference to the old value if this change removes/modifies one, None otherwise
    pub fn get_old_value(&self) -> Option<&Value> {
        match self {
            StateChange::Added { .. } => None,
            StateChange::Modified { old_value, .. } => Some(old_value),
            StateChange::Removed { old_value, .. } => Some(old_value),
        }
    }
}

/// State diff computer for comparing two MPT states
///
/// This struct provides functionality to compute the difference between
/// two Merkle Patricia Trie states efficiently.
pub struct StateDiffComputer;

impl StateDiffComputer {
    /// Compute the difference between two tries
    ///
    /// # Arguments
    ///
    /// * `from_trie` - The source trie
    /// * `to_trie` - The target trie
    ///
    /// # Returns
    ///
    /// A Result containing the computed StateDiff or an error
    pub fn compute_diff<S: NodeStorage>(from_trie: &MerklePatriciaTrie<S>, to_trie: &MerklePatriciaTrie<S>) -> TrieResult<StateDiff> {
        let mut diff = StateDiff::new(from_trie.root_hash(), to_trie.root_hash());
        let from_keys = from_trie.get_all_keys()?;
        let to_keys = to_trie.get_all_keys()?;

        // Create sets for efficient lookup
        let from_key_set: HashSet<_> = from_keys.into_iter().collect();
        let to_key_set: HashSet<_> = to_keys.into_iter().collect();

        // Find added and modified keys
        for key in to_key_set.iter() {
            let to_value = to_trie.get(key)?;
            let from_value = from_trie.get(key)?;

            match (from_value, to_value) {
                (None, Some(value)) => {
                    diff.add_change(StateChange::Added {
                        key: key.clone(),
                        value: value.clone(),
                    });
                }
                (Some(old_value), Some(new_value)) if old_value != new_value => {
                    diff.add_change(StateChange::Modified {
                        key: key.clone(),
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });
                }
                _ => {}
            }
        }

        // Find removed keys
        for key in from_key_set.iter() {
            if !to_key_set.contains(key)
                && let Some(old_value) = from_trie.get(key)?
            {
                diff.add_change(StateChange::Removed {
                    key: key.clone(),
                    old_value: old_value.clone(),
                });
            }
        }

        Ok(diff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mpt::MerklePatriciaTrie;
    use crate::state::mpt::trie::InMemoryStorage;

    #[test]
    fn test_state_change_creation() {
        let key = vec![1, 2, 3];
        let value = vec![4, 5, 6];
        let old_value = vec![7, 8, 9];
        let new_value = vec![10, 11, 12];

        let added = StateChange::Added {
            key: key.clone(),
            value: value.clone(),
        };
        assert_eq!(added.key(), &key);
        assert_eq!(added.get_new_value(), Some(&value));
        assert_eq!(added.get_old_value(), None);

        let modified = StateChange::Modified {
            key: key.clone(),
            old_value: old_value.clone(),
            new_value: new_value.clone(),
        };
        assert_eq!(modified.key(), &key);
        assert_eq!(modified.get_new_value(), Some(&new_value));
        assert_eq!(modified.get_old_value(), Some(&old_value));

        let removed = StateChange::Removed {
            key: key.clone(),
            old_value: old_value.clone(),
        };
        assert_eq!(removed.key(), &key);
        assert_eq!(removed.get_new_value(), None);
        assert_eq!(removed.get_old_value(), Some(&old_value));
    }

    #[test]
    fn test_state_diff_creation() {
        let mut diff = StateDiff::new([1; 32], [2; 32]);
        assert!(diff.is_empty());
        assert_eq!(diff.change_count(), 0);
        assert_eq!(diff.change_summary(), (0, 0, 0));

        diff.add_change(StateChange::Added { key: vec![1], value: vec![2] });
        assert!(!diff.is_empty());
        assert_eq!(diff.change_count(), 1);
        assert_eq!(diff.change_summary(), (1, 0, 0));
    }

    #[test]
    fn test_state_diff_operations() {
        let mut diff = StateDiff::new([1; 32], [2; 32]);
        let key1 = vec![1];
        let key2 = vec![2];
        let value1 = vec![3];
        let value2 = vec![4];

        diff.add_change(StateChange::Added {
            key: key1.clone(),
            value: value1.clone(),
        });
        diff.add_change(StateChange::Modified {
            key: key2.clone(),
            old_value: value1.clone(),
            new_value: value2.clone(),
        });

        assert_eq!(diff.added_keys(), vec![&key1]);
        assert_eq!(diff.modified_keys(), vec![&key2]);
        assert_eq!(diff.removed_keys(), Vec::<&Key>::new());
    }

    #[test]
    fn test_diff_reverse() {
        let mut diff = StateDiff::new([1; 32], [2; 32]);
        let key = vec![1];
        let old_value = vec![2];
        let new_value = vec![3];

        diff.add_change(StateChange::Modified {
            key: key.clone(),
            old_value: old_value.clone(),
            new_value: new_value.clone(),
        });

        let reversed = diff.reverse();
        assert_eq!(reversed.from_root, diff.to_root);
        assert_eq!(reversed.to_root, diff.from_root);

        if let StateChange::Modified {
            key: reversed_key,
            old_value: reversed_old,
            new_value: reversed_new,
        } = &reversed.changes[0]
        {
            assert_eq!(reversed_key, &key);
            assert_eq!(reversed_old, &new_value);
            assert_eq!(reversed_new, &old_value);
        } else {
            assert!(false, "Expected Modified change");
        }
    }

    #[test]
    fn test_state_change_key_access() {
        let key = vec![1];
        let value = vec![2];

        let added = StateChange::Added {
            key: key.clone(),
            value: value.clone(),
        };
        assert_eq!(added.key(), &key);

        let modified = StateChange::Modified {
            key: key.clone(),
            old_value: value.clone(),
            new_value: value.clone(),
        };
        assert_eq!(modified.key(), &key);

        let removed = StateChange::Removed {
            key: key.clone(),
            old_value: value.clone(),
        };
        assert_eq!(removed.key(), &key);
    }

    #[test]
    fn test_diff_computation() {
        let mut from_trie = MerklePatriciaTrie::new_in_memory();
        let mut to_trie = MerklePatriciaTrie::new_in_memory();

        // Add some initial state
        from_trie.put(vec![1], vec![1]).unwrap();
        from_trie.put(vec![2], vec![2]).unwrap();

        // Create target state with changes
        to_trie.put(vec![1], vec![1]).unwrap(); // unchanged
        to_trie.put(vec![2], vec![3]).unwrap(); // modified
        to_trie.put(vec![3], vec![3]).unwrap(); // added

        let diff = StateDiffComputer::compute_diff(&from_trie, &to_trie).unwrap();
        assert_eq!(diff.change_count(), 2);
        assert_eq!(diff.change_summary(), (1, 1, 0));
    }

    #[test]
    fn test_diff_application() {
        let mut from_trie = MerklePatriciaTrie::new_in_memory();
        let mut to_trie = MerklePatriciaTrie::new_in_memory();
        let mut target_trie = MerklePatriciaTrie::new_in_memory();

        // Create source state
        from_trie.put(vec![1], vec![1]).unwrap();
        from_trie.put(vec![2], vec![2]).unwrap();

        // Create target state
        to_trie.put(vec![1], vec![1]).unwrap();
        to_trie.put(vec![2], vec![3]).unwrap();
        to_trie.put(vec![3], vec![3]).unwrap();

        // Compute and apply diff
        let diff = StateDiffComputer::compute_diff(&from_trie, &to_trie).unwrap();
        diff.apply_to(&from_trie, &mut target_trie).unwrap();

        // Verify target state matches
        assert_eq!(target_trie.get(&vec![1]).unwrap(), Some(vec![1]));
        assert_eq!(target_trie.get(&vec![2]).unwrap(), Some(vec![3]));
        assert_eq!(target_trie.get(&vec![3]).unwrap(), Some(vec![3]));
    }

    #[test]
    fn test_changes_for_key() {
        let mut diff = StateDiff::new([1; 32], [2; 32]);
        let key = vec![1];
        let value1 = vec![2];
        let value2 = vec![3];

        diff.add_change(StateChange::Modified {
            key: key.clone(),
            old_value: value1.clone(),
            new_value: value2.clone(),
        });

        let changes = diff.changes_for_key(&key);
        assert_eq!(changes.len(), 1);
        assert!(diff.has_change_for_key(&key));
        assert!(!diff.has_change_for_key(&vec![2]));
    }

    #[test]
    fn test_large_state_diff() {
        let mut from_trie = MerklePatriciaTrie::new_in_memory();
        let mut to_trie = MerklePatriciaTrie::new_in_memory();

        // Create large state
        for i in 0..1000 {
            let key = vec![(i >> 8) as u8, (i & 0xFF) as u8]; // 2-byte unique key
            from_trie.put(key.clone(), vec![i as u8]).unwrap();
            if i % 2 == 0 {
                to_trie.put(key, vec![(i + 1) as u8]).unwrap();
            } else {
                to_trie.put(key, vec![i as u8]).unwrap();
            }
        }

        let diff = StateDiffComputer::compute_diff(&from_trie, &to_trie).unwrap();
        assert_eq!(diff.change_summary().1, 500); // 500 modifications
    }
}
