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

//! State Merkle Tree implementation
//!
//! This module provides a cryptographic structure for efficient verification
//! of state integrity through a Merkle tree.

use crate::vm::state_management::lib::{Error, Result, StateKey, StateValue};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

/// Type alias for state hash values (32 bytes SHA-256)
pub type StateHash = [u8; 32];

/// A node in the Merkle tree
#[derive(Debug, Clone)]
pub struct MerkleNode {
    /// Hash of this node
    pub hash: StateHash,
    /// Left child reference, if any
    pub left: Option<Box<MerkleNode>>,
    /// Right child reference, if any
    pub right: Option<Box<MerkleNode>>,
    /// Key for leaf nodes, None for internal nodes
    pub key: Option<StateKey>,
    /// Value for leaf nodes, None for internal nodes
    pub value: Option<StateValue>,
}

impl MerkleNode {
    /// Creates a new leaf node
    pub fn new_leaf(key: StateKey, value: StateValue) -> Result<Self> {
        let hash = compute_leaf_hash(&key, &value);

        Ok(Self {
            hash,
            left: None,
            right: None,
            key: Some(key),
            value: Some(value),
        })
    }

    /// Creates a new internal node from two child nodes
    pub fn new_internal(left: MerkleNode, right: MerkleNode) -> Self {
        let hash = compute_internal_hash(&left.hash, &right.hash);

        Self {
            hash,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            key: None,
            value: None,
        }
    }

    /// Checks if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.key.is_some() && self.value.is_some()
    }
}

/// Merkle proof for a specific key
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// Key being proven
    pub key: StateKey,
    /// Value being proven
    pub value: Option<StateValue>,
    /// List of sibling hashes along the path to the root
    pub siblings: Vec<(bool, StateHash)>, // (is_right, hash)
}

impl MerkleProof {
    /// Verify the proof against a given root hash
    pub fn verify(&self, root_hash: &StateHash) -> bool {
        let mut current_hash = match &self.value {
            Some(value) => compute_leaf_hash(&self.key, value),
            None => return false, // Cannot verify absence proofs with this implementation
        };

        for (is_right, sibling_hash) in &self.siblings {
            if *is_right {
                // Sibling is on the right, current is on the left
                current_hash = compute_internal_hash(&current_hash, sibling_hash);
            } else {
                // Sibling is on the left, current is on the right
                current_hash = compute_internal_hash(sibling_hash, &current_hash);
            }
        }

        // The final hash should match the root hash
        current_hash == *root_hash
    }
}

/// Merkle tree for state verification
pub struct MerkleTree {
    /// Root node of the tree
    root: Option<MerkleNode>,
    /// Cache of leaf nodes for faster lookups
    leaves: HashMap<StateKey, MerkleNode>,
}

/// Merkle Tree implementation for state integrity verification.
/// Provides cryptographic proofs and hash-based consistency checks.
impl MerkleTree {
    /// Creates a new empty Merkle tree
    pub fn new() -> Self {
        Self { root: None, leaves: HashMap::new() }
    }

    /// Builds a Merkle tree from a state map with:
    /// - **Leaf Hashing**: SHA-256 hashes of key-value pairs
    /// - **Balanced Tree**: Auto-balances odd node counts
    /// - **Proof Generation**: Path proofs for individual keys
    ///
    /// # Arguments
    /// - `state`: BTreeMap of key-value pairs
    ///
    /// # Returns
    /// - `MerkleTree`: Constructed tree
    /// - `Error`: On empty state or hashing failures
    pub fn build(state: &BTreeMap<StateKey, StateValue>) -> Result<Self> {
        if state.is_empty() {
            return Ok(Self::new());
        }

        let mut leaves = HashMap::new();
        let mut nodes = Vec::new();

        // Create leaf nodes
        for (key, value) in state {
            let leaf = MerkleNode::new_leaf(key.clone(), value.clone())?;
            leaves.insert(key.clone(), leaf.clone());
            nodes.push(leaf);
        }

        // Build the tree bottom-up
        while nodes.len() > 1 {
            let mut next_level = Vec::new();

            // Process pairs of nodes
            for chunk in nodes.chunks(2) {
                if chunk.len() == 2 {
                    // Create an internal node from two children
                    let internal = MerkleNode::new_internal(chunk[0].clone(), chunk[1].clone());
                    next_level.push(internal);
                } else {
                    // Odd number of nodes, pass the last one up
                    next_level.push(chunk[0].clone());
                }
            }

            nodes = next_level;
        }

        // The last remaining node is the root
        Ok(Self {
            root: if nodes.is_empty() { None } else { Some(nodes.remove(0)) },
            leaves,
        })
    }

    /// Gets the root hash of the tree
    pub fn root_hash(&self) -> Option<StateHash> {
        self.root.as_ref().map(|node| node.hash)
    }

    /// Gets the leaf node for a specific key
    pub fn get_leaf(&self, key: &StateKey) -> Option<&MerkleNode> {
        self.leaves.get(key)
    }

    /// Generates a Merkle proof for a specific key
    pub fn generate_proof(&self, key: &StateKey) -> Result<MerkleProof> {
        let root = self.root.as_ref().ok_or_else(|| Error::MerkleError("Empty tree".to_string()))?;

        let leaf = self.leaves.get(key).ok_or_else(|| Error::NotFound)?;

        let mut siblings = Vec::new();

        // Since we're using a binary tree built bottom-up from sorted keys,
        // we need to find the path from root to leaf
        if !self.find_path_to_leaf(root, key, &mut siblings)? {
            return Err(Error::MerkleError("Failed to find path to leaf".to_string()));
        }

        Ok(MerkleProof {
            key: key.clone(),
            value: leaf.value.clone(),
            siblings,
        })
    }

    /// Helper function to find path from root to leaf and collect siblings
    fn find_path_to_leaf(&self, node: &MerkleNode, key: &StateKey, siblings: &mut Vec<(bool, StateHash)>) -> Result<bool> {
        // If this is a leaf node, check if it's the one we're looking for
        if node.is_leaf() {
            return Ok(node.key.as_ref().unwrap() == key);
        }

        // This is an internal node, check left subtree first
        if let Some(left) = &node.left {
            if self.find_path_to_leaf(left, key, siblings)? {
                // Key found in left subtree, add right sibling if it exists
                if let Some(right) = &node.right {
                    siblings.push((true, right.hash));
                }
                return Ok(true);
            }
        }

        // Check right subtree
        if let Some(right) = &node.right {
            if self.find_path_to_leaf(right, key, siblings)? {
                // Key found in right subtree, add left sibling
                if let Some(left) = &node.left {
                    siblings.push((false, left.hash));
                }
                return Ok(true);
            }
        }

        // Key not found in this subtree
        Ok(false)
    }

    /// Updates the tree with a new state (rebuilds the tree)
    pub fn update(&mut self, state: &BTreeMap<StateKey, StateValue>) -> Result<()> {
        let new_tree = Self::build(state)?;
        *self = new_tree;
        Ok(())
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes the hash for a leaf node
fn compute_leaf_hash(key: &StateKey, value: &StateValue) -> StateHash {
    let mut hasher = Sha256::new();
    // Prefix with 0 to distinguish from internal nodes
    hasher.update([0u8]);
    hasher.update(key.as_bytes());
    hasher.update(value.as_bytes());

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Computes the hash for an internal node
fn compute_internal_hash(left: &StateHash, right: &StateHash) -> StateHash {
    let mut hasher = Sha256::new();
    // Prefix with 1 to distinguish from leaf nodes
    hasher.update([1u8]);
    hasher.update(left);
    hasher.update(right);

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::state_management::lib::{StateKey, StateValue};

    fn setup_test_state() -> BTreeMap<StateKey, StateValue> {
        let mut state = BTreeMap::new();
        state.insert(StateKey::from_string("key1"), StateValue::from_string("value1"));
        state.insert(StateKey::from_string("key2"), StateValue::from_string("value2"));
        state.insert(StateKey::from_string("key3"), StateValue::from_string("value3"));
        state.insert(StateKey::from_string("key4"), StateValue::from_string("value4"));
        state
    }

    #[test]
    fn test_build_tree() {
        let state = setup_test_state();
        let tree = MerkleTree::build(&state).unwrap();

        // Tree should have a root
        assert!(tree.root.is_some());

        // All keys should be in the leaves
        for key in state.keys() {
            assert!(tree.get_leaf(key).is_some());
        }
    }

    #[test]
    fn test_leaf_hash() {
        let key = StateKey::from_string("test_key");
        let value = StateValue::from_string("test_value");

        let hash1 = compute_leaf_hash(&key, &value);
        let hash2 = compute_leaf_hash(&key, &value);

        // Same input should produce same hash
        assert_eq!(hash1, hash2);

        // Different input should produce different hash
        let diff_value = StateValue::from_string("different_value");
        let hash3 = compute_leaf_hash(&key, &diff_value);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_internal_hash() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];

        let internal1 = compute_internal_hash(&hash1, &hash2);
        let internal2 = compute_internal_hash(&hash1, &hash2);

        // Same input should produce same hash
        assert_eq!(internal1, internal2);

        // Order matters
        let internal3 = compute_internal_hash(&hash2, &hash1);
        assert_ne!(internal1, internal3);
    }

    #[test]
    fn test_proof_generation_and_verification() {
        let state = setup_test_state();
        let tree = MerkleTree::build(&state).unwrap();
        let root_hash = tree.root_hash().unwrap();

        // Generate and verify proof for each key
        for key in state.keys() {
            let proof = tree.generate_proof(key).unwrap();
            assert!(proof.verify(&root_hash));

            // Verification should fail with a different root hash
            let wrong_hash = [0u8; 32];
            assert!(!proof.verify(&wrong_hash));
        }
    }

    #[test]
    fn test_tree_update() {
        let state = setup_test_state();
        let mut tree = MerkleTree::build(&state).unwrap();
        let original_root = tree.root_hash().unwrap();

        // Update with a modified state
        let mut new_state = state.clone();
        new_state.insert(StateKey::from_string("key5"), StateValue::from_string("value5"));
        tree.update(&new_state).unwrap();

        // Root should change
        let new_root = tree.root_hash().unwrap();
        assert_ne!(original_root, new_root);

        // New key should be in the leaves
        assert!(tree.get_leaf(&StateKey::from_string("key5")).is_some());
    }
}
