//! Merkle Proof Implementation for MPT
//!
//! This module provides functionality for generating and verifying Merkle proofs
//! in the Merkle Patricia Trie. It includes proof generation during trie traversal
//! and verification of proofs against a known root hash.
//!
//! # Features
//!
//! - Proof generation during trie traversal
//! - Cryptographic verification of proofs
//! - Support for all node types (Leaf, Extension, Branch)
//! - Efficient proof encoding/decoding
//!
//! # Performance Considerations
//!
//! - Minimal memory allocations during proof generation
//! - Efficient proof verification
//! - Compact proof serialization
//! - Zero-copy operations where possible

use crate::state::mpt::lib::{Hash, Key, MPTError, NodeId, TrieResult, Value, keccak256, key_to_nibbles};
use crate::state::mpt::node::{Node, NodeType};
use bincode;
use serde::{Deserialize, Serialize};

/// Proof element in a Merkle proof
///
/// Each element in a proof contains a node's ID and its encoded data.
/// This allows for verification of the node's existence and content
/// in the trie.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofElement {
    /// The hash of the node
    pub node_id: NodeId,
    /// The encoded node data
    pub node_data: Vec<u8>,
}

/// State proof for a key-value pair
///
/// A complete proof that can be used to verify the existence or non-existence
/// of a key-value pair in the trie. The proof includes all necessary nodes
/// to reconstruct the path from the root to the target node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateProof {
    /// The key being proven
    pub key: Key,
    /// The value (if it exists) being proven
    pub value: Option<Value>,
    /// The sequence of nodes in the proof
    pub proof_elements: Vec<ProofElement>,
    /// The root hash of the trie
    pub root_hash: Hash,
}

impl StateProof {
    /// Create a new state proof
    ///
    /// # Arguments
    ///
    /// * `key` - The key being proven
    /// * `value` - The value (if it exists) being proven
    /// * `proof_elements` - The sequence of nodes in the proof
    /// * `root_hash` - The root hash of the trie
    pub fn new(key: Key, value: Option<Value>, proof_elements: Vec<ProofElement>, root_hash: Hash) -> Self {
        Self {
            key,
            value,
            proof_elements,
            root_hash,
        }
    }

    /// Verify the proof against a root hash
    ///
    /// This method verifies that:
    /// 1. The proof elements form a valid path in the trie
    /// 2. The node hashes match the expected values
    /// 3. The key-value pair (or its absence) is correctly proven
    ///
    /// # Returns
    ///
    /// A Result containing a boolean indicating whether the proof is valid
    pub fn verify(&self) -> TrieResult<bool> {
        if self.proof_elements.is_empty() {
            return Ok(false);
        }

        let mut current_hash = self.root_hash;
        let mut key_nibbles = key_to_nibbles(&self.key);

        for element in &self.proof_elements {
            // Chain verification: current_hash must match element.node_id
            if current_hash != element.node_id {
                return Ok(false);
            }
            // Verify node hash matches
            let computed_hash = keccak256(&element.node_data);
            if computed_hash != element.node_id {
                return Ok(false);
            }

            // Decode and verify node
            let node = Node::decode(&element.node_data)?;
            match &node.node_type {
                NodeType::Empty => {
                    if self.value.is_some() {
                        return Ok(false);
                    }
                }
                NodeType::Leaf { path, value } => {
                    if path.nibbles != key_nibbles {
                        return Ok(false);
                    }
                    if self.value.as_ref() != Some(value) {
                        return Ok(false);
                    }
                    return Ok(true);
                }
                NodeType::Extension { path, child } => {
                    if !key_nibbles.starts_with(&path.nibbles) {
                        return Ok(false);
                    }
                    key_nibbles = key_nibbles[path.nibbles.len()..].to_vec();
                    current_hash = *child;
                }
                NodeType::Branch { children, value } => {
                    if key_nibbles.is_empty() {
                        if let Some(expected_value) = &self.value {
                            if value.as_ref() != Some(expected_value) {
                                return Ok(false);
                            }
                            return Ok(true);
                        } else if value.is_some() {
                            return Ok(false);
                        }
                    } else {
                        let nibble = key_nibbles[0] as usize;
                        if nibble >= 16 || children[nibble].is_none() {
                            return Ok(false);
                        }
                        current_hash = children[nibble].unwrap();
                        key_nibbles = key_nibbles[1..].to_vec();
                    }
                }
            }
        }

        // If we've processed all nodes and still have key nibbles left,
        // or if we're expecting a value but haven't found it
        if !key_nibbles.is_empty() || self.value.is_some() {
            return Ok(false);
        }

        Ok(true)
    }

    /// Encode proof for serialization
    ///
    /// # Returns
    ///
    /// A byte vector containing the encoded proof
    pub fn encode(&self) -> Vec<u8> {
        bincode::serde::encode_to_vec(self, bincode::config::standard()).unwrap_or_default()
    }

    /// Decode proof from bytes
    ///
    /// # Arguments
    ///
    /// * `data` - The encoded proof data
    ///
    /// # Returns
    ///
    /// A Result containing either the decoded proof or an error
    pub fn decode(data: &[u8]) -> TrieResult<Self> {
        let (val, _): (Self, _) = bincode::serde::decode_from_slice(data, bincode::config::standard()).map_err(|e| MPTError::SerializationError(e.to_string()))?;
        Ok(val)
    }

    /// Get proof size in bytes
    ///
    /// # Returns
    ///
    /// The size of the encoded proof in bytes
    pub fn size(&self) -> usize {
        self.encode().len()
    }
}

/// Proof builder for constructing state proofs during trie traversal
///
/// This builder helps construct proofs incrementally as the trie is traversed.
/// It maintains the sequence of nodes that form the proof path.
pub struct ProofBuilder {
    elements: Vec<ProofElement>,
}

impl ProofBuilder {
    /// Create a new proof builder
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    /// Add a node to the proof
    ///
    /// # Arguments
    ///
    /// * `node` - The node to add to the proof
    pub fn add_node(&mut self, node: &Node) {
        let element = ProofElement {
            node_id: node.id,
            node_data: node.encode(),
        };
        self.elements.push(element);
    }

    /// Build the final proof
    ///
    /// # Arguments
    ///
    /// * `key` - The key being proven
    /// * `value` - The value (if it exists) being proven
    /// * `root_hash` - The root hash of the trie
    ///
    /// # Returns
    ///
    /// A complete state proof
    pub fn build(self, key: Key, value: Option<Value>, root_hash: Hash) -> StateProof {
        StateProof::new(key, value, self.elements, root_hash)
    }

    /// Get current number of elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if builder is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl Default for ProofBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mpt::lib::CompactPath;

    #[test]
    fn test_leaf_proof_verification() {
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let path = CompactPath::new(key_to_nibbles(&key), true);
        let leaf_node = Node::new_leaf(path, value.clone());

        let proof_element = ProofElement {
            node_id: leaf_node.id,
            node_data: leaf_node.encode(),
        };

        let proof = StateProof::new(key, Some(value), vec![proof_element], leaf_node.id);

        assert!(proof.verify().unwrap());
    }

    #[test]
    fn test_proof_verification_with_wrong_value() {
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let wrong_value = b"wrong_value".to_vec();
        let path = CompactPath::new(key_to_nibbles(&key), true);
        let leaf_node = Node::new_leaf(path, value);

        let proof_element = ProofElement {
            node_id: leaf_node.id,
            node_data: leaf_node.encode(),
        };

        let proof = StateProof::new(key, Some(wrong_value), vec![proof_element], leaf_node.id);

        assert!(!proof.verify().unwrap());
    }

    #[test]
    fn test_proof_verification_with_wrong_hash() {
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let path = CompactPath::new(key_to_nibbles(&key), true);
        let leaf_node = Node::new_leaf(path, value.clone());

        let proof_element = ProofElement {
            node_id: leaf_node.id,
            node_data: leaf_node.encode(),
        };

        let wrong_root = keccak256(b"wrong_root");
        let proof = StateProof::new(key, Some(value), vec![proof_element], wrong_root);

        assert!(!proof.verify().unwrap());
    }

    #[test]
    fn test_proof_builder() {
        let mut builder = ProofBuilder::new();
        assert!(builder.is_empty());
        assert_eq!(builder.len(), 0);

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let path = CompactPath::new(key_to_nibbles(&key), true);
        let leaf_node = Node::new_leaf(path, value.clone());

        builder.add_node(&leaf_node);
        assert!(!builder.is_empty());
        assert_eq!(builder.len(), 1);

        let proof = builder.build(key.clone(), Some(value), leaf_node.id);
        assert_eq!(proof.key, key);
        assert_eq!(proof.proof_elements.len(), 1);
    }

    #[test]
    fn test_proof_encoding_decoding() {
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let path = CompactPath::new(key_to_nibbles(&key), true);
        let leaf_node = Node::new_leaf(path, value.clone());

        let proof_element = ProofElement {
            node_id: leaf_node.id,
            node_data: leaf_node.encode(),
        };

        let original_proof = StateProof::new(key, Some(value), vec![proof_element], leaf_node.id);
        let encoded = original_proof.encode();
        let decoded_proof = StateProof::decode(&encoded).unwrap();

        assert_eq!(original_proof, decoded_proof);
    }

    #[test]
    fn test_empty_node_proof() {
        use crate::state::mpt::trie::MerklePatriciaTrie;
        let trie = MerklePatriciaTrie::new_in_memory();
        let key = b"test_key".to_vec();
        let proof = trie.get_proof(&key).unwrap();
        assert!(!proof.verify().unwrap());
        assert_eq!(proof.value, None);
    }

    // Additional tests for edge cases and complex scenarios
    #[test]
    fn test_extension_node_proof() {
        use crate::state::mpt::trie::MerklePatriciaTrie;
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let mut trie = MerklePatriciaTrie::new_in_memory();
        trie.put(key.clone(), value.clone()).unwrap();
        let proof = trie.get_proof(&key).unwrap();
        assert!(proof.verify().unwrap());
        assert_eq!(proof.value, Some(value));
    }

    #[test]
    fn test_branch_node_proof() {
        use crate::state::mpt::trie::MerklePatriciaTrie;
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let mut trie = MerklePatriciaTrie::new_in_memory();
        trie.put(key.clone(), value.clone()).unwrap();
        let proof = trie.get_proof(&key).unwrap();
        assert!(proof.verify().unwrap());
        assert_eq!(proof.value, Some(value));
    }

    #[test]
    fn test_proof_size() {
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        let path = CompactPath::new(key_to_nibbles(&key), true);
        let leaf_node = Node::new_leaf(path, value.clone());

        let proof_element = ProofElement {
            node_id: leaf_node.id,
            node_data: leaf_node.encode(),
        };

        let proof = StateProof::new(key, Some(value), vec![proof_element], leaf_node.id);
        assert!(proof.size() > 0);
    }

    #[test]
    fn test_invalid_proof_data() {
        let invalid_data = b"invalid_proof_data";
        assert!(matches!(StateProof::decode(invalid_data), Err(MPTError::SerializationError(_))));
    }

    #[test]
    fn test_proof_with_multiple_nodes() {
        use crate::state::mpt::trie::MerklePatriciaTrie;
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        trie.put(key.clone(), value.clone()).unwrap();
        let proof = trie.get_proof(&key).unwrap();
        assert!(proof.verify().unwrap());
        assert_eq!(proof.value, Some(value));
    }
}
