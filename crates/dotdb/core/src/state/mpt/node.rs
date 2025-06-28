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

//! Node implementation for Merkle Patricia Trie
//!
//! This module provides the core node types and operations for the Merkle Patricia Trie.
//! It includes implementations for different node types (Empty, Leaf, Extension, Branch)
//! and their associated operations.
//!
//! # Node Types
//!
//! - `Empty`: Represents an empty node in the trie
//! - `Leaf`: Stores a key-value pair with a path
//! - `Extension`: Compresses paths with single children
//! - `Branch`: Stores up to 16 children and an optional value
//!
//! # Performance Considerations
//!
//! - Efficient node encoding/decoding using bincode
//! - Optimized node ID calculation
//! - Minimal memory usage through path compression
//! - Fast node type checking using pattern matching

use crate::state::mpt::lib::{CompactPath, MPTError, NodeId, TrieResult, Value, keccak256};
use bincode;
use serde::{Deserialize, Serialize};

/// Types of nodes in the MPT
///
/// Each node type serves a specific purpose in the trie structure:
/// - `Empty`: Represents an empty or non-existent node
/// - `Leaf`: Stores actual key-value data
/// - `Extension`: Compresses paths with single children for efficiency
/// - `Branch`: Handles multiple children for different nibble values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    /// Leaf node: contains a key-value pair
    Leaf { path: CompactPath, value: Value },
    /// Extension node: path compression for single-child branches
    Extension { path: CompactPath, child: NodeId },
    /// Branch node: up to 16 children (for each nibble)
    Branch { children: [Option<NodeId>; 16], value: Option<Value> },
    /// Empty node
    Empty,
}

/// Node in the MPT
///
/// Each node contains a unique identifier (hash) and its type-specific data.
/// The node ID is calculated based on the node's content to ensure
/// cryptographic verification of the trie structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier (hash) of the node
    pub id: NodeId,
    /// Type-specific data of the node
    pub node_type: NodeType,
}

impl Node {
    /// Create a new empty node
    ///
    /// # Returns
    ///
    /// A new empty node with a pre-calculated ID
    pub fn new_empty() -> Self {
        let node_type = NodeType::Empty;
        let id = Self::calculate_id(&node_type);
        Self { id, node_type }
    }

    /// Create a new leaf node
    ///
    /// # Arguments
    ///
    /// * `path` - The compact path for the leaf
    /// * `value` - The value to store in the leaf
    ///
    /// # Returns
    ///
    /// A new leaf node with the given path and value
    pub fn new_leaf(path: CompactPath, value: Value) -> Self {
        let node_type = NodeType::Leaf { path, value };
        let id = Self::calculate_id(&node_type);
        Self { id, node_type }
    }

    /// Create a new extension node
    ///
    /// # Arguments
    ///
    /// * `path` - The compact path for the extension
    /// * `child` - The ID of the child node
    ///
    /// # Returns
    ///
    /// A new extension node with the given path and child
    pub fn new_extension(path: CompactPath, child: NodeId) -> Self {
        let node_type = NodeType::Extension { path, child };
        let id = Self::calculate_id(&node_type);
        Self { id, node_type }
    }

    /// Create a new branch node
    ///
    /// # Arguments
    ///
    /// * `children` - Array of optional child node IDs
    /// * `value` - Optional value to store in the branch
    ///
    /// # Returns
    ///
    /// A new branch node with the given children and value
    pub fn new_branch(children: [Option<NodeId>; 16], value: Option<Value>) -> Self {
        let node_type = NodeType::Branch { children, value };
        let id = Self::calculate_id(&node_type);
        Self { id, node_type }
    }

    /// Calculate node ID based on its content
    ///
    /// # Arguments
    ///
    /// * `node_type` - The node type to calculate ID for
    ///
    /// # Returns
    ///
    /// A 32-byte hash representing the node's ID
    fn calculate_id(node_type: &NodeType) -> NodeId {
        let encoded = Self::encode_node_type(node_type);
        keccak256(&encoded)
    }

    /// Encode node for hashing and storage
    ///
    /// # Returns
    ///
    /// A byte vector containing the encoded node data
    pub fn encode(&self) -> Vec<u8> {
        Self::encode_node_type(&self.node_type)
    }

    /// Encode node type for hashing and storage
    ///
    /// # Arguments
    ///
    /// * `node_type` - The node type to encode
    ///
    /// # Returns
    ///
    /// A byte vector containing the encoded node type data
    fn encode_node_type(node_type: &NodeType) -> Vec<u8> {
        bincode::serde::encode_to_vec(node_type, bincode::config::standard()).unwrap_or_default()
    }

    /// Decode node from bytes
    ///
    /// # Arguments
    ///
    /// * `data` - The encoded node data
    ///
    /// # Returns
    ///
    /// A Result containing either the decoded node or an error
    pub fn decode(data: &[u8]) -> TrieResult<Node> {
        let (node_type, _): (NodeType, _) = bincode::serde::decode_from_slice(data, bincode::config::standard()).map_err(|e| MPTError::SerializationError(e.to_string()))?;

        let id = Self::calculate_id(&node_type);
        Ok(Node { id, node_type })
    }

    /// Check if node is empty
    pub fn is_empty(&self) -> bool {
        matches!(self.node_type, NodeType::Empty)
    }

    /// Check if node is leaf
    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf { .. })
    }

    /// Check if node is extension
    pub fn is_extension(&self) -> bool {
        matches!(self.node_type, NodeType::Extension { .. })
    }

    /// Check if node is branch
    pub fn is_branch(&self) -> bool {
        matches!(self.node_type, NodeType::Branch { .. })
    }

    /// Get value from leaf or branch node
    ///
    /// # Returns
    ///
    /// An optional reference to the node's value
    pub fn get_value(&self) -> Option<&Value> {
        match &self.node_type {
            NodeType::Leaf { value, .. } => Some(value),
            NodeType::Branch { value, .. } => value.as_ref(),
            _ => None,
        }
    }

    /// Get path from leaf or extension node
    ///
    /// # Returns
    ///
    /// An optional reference to the node's path
    pub fn get_path(&self) -> Option<&CompactPath> {
        match &self.node_type {
            NodeType::Leaf { path, .. } => Some(path),
            NodeType::Extension { path, .. } => Some(path),
            _ => None,
        }
    }

    /// Get child from extension node
    ///
    /// # Returns
    ///
    /// An optional node ID of the extension's child
    pub fn get_extension_child(&self) -> Option<NodeId> {
        match &self.node_type {
            NodeType::Extension { child, .. } => Some(*child),
            _ => None,
        }
    }

    /// Get children from branch node
    ///
    /// # Returns
    ///
    /// An optional reference to the branch's children array
    pub fn get_branch_children(&self) -> Option<&[Option<NodeId>; 16]> {
        match &self.node_type {
            NodeType::Branch { children, .. } => Some(children),
            _ => None,
        }
    }

    /// Clone node with new value (for branch nodes)
    ///
    /// # Arguments
    ///
    /// * `new_value` - The new value to set
    ///
    /// # Returns
    ///
    /// A Result containing either the new node or an error
    pub fn with_branch_value(&self, new_value: Option<Value>) -> TrieResult<Node> {
        match &self.node_type {
            NodeType::Branch { children, .. } => Ok(Node::new_branch(*children, new_value)),
            _ => Err(MPTError::InvalidNodeType),
        }
    }

    /// Clone node with updated child (for branch nodes)
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the child to update (0-15)
    /// * `child` - The new child node ID
    ///
    /// # Returns
    ///
    /// A Result containing either the new node or an error
    pub fn with_branch_child(&self, index: usize, child: Option<NodeId>) -> TrieResult<Node> {
        if index >= 16 {
            return Err(MPTError::InvalidNodeType);
        }

        match &self.node_type {
            NodeType::Branch { children, value } => {
                let mut new_children = *children;
                new_children[index] = child;
                Ok(Node::new_branch(new_children, value.clone()))
            }
            _ => Err(MPTError::InvalidNodeType),
        }
    }

    /// Calculate the size of the node in bytes
    ///
    /// # Returns
    ///
    /// The size of the node in bytes
    pub fn size_bytes(&self) -> u64 {
        match &self.node_type {
            NodeType::Empty => 0,
            NodeType::Leaf { path, value } => path.nibbles.len() as u64 + value.len() as u64,
            NodeType::Extension { path, child } => path.nibbles.len() as u64 + child.len() as u64,
            NodeType::Branch { children, value } => children.iter().filter_map(|c| c.as_ref()).map(|c| c.len()).sum::<usize>() as u64 + value.as_ref().map_or(0, |v| v.len() as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_node() {
        let node = Node::new_empty();
        assert!(node.is_empty());
        assert!(!node.is_leaf());
        assert!(!node.is_extension());
        assert!(!node.is_branch());
        assert_eq!(node.get_value(), None);
    }

    #[test]
    fn test_leaf_node() {
        let path = CompactPath::new(vec![1, 2, 3], true);
        let value = b"test_value".to_vec();
        let node = Node::new_leaf(path.clone(), value.clone());

        assert!(!node.is_empty());
        assert!(node.is_leaf());
        assert!(!node.is_extension());
        assert!(!node.is_branch());
        assert_eq!(node.get_value(), Some(&value));
        assert_eq!(node.get_path(), Some(&path));
    }

    #[test]
    fn test_extension_node() {
        let path = CompactPath::new(vec![4, 5, 6], false);
        let child_id = keccak256(b"child");
        let node = Node::new_extension(path.clone(), child_id);

        assert!(!node.is_empty());
        assert!(!node.is_leaf());
        assert!(node.is_extension());
        assert!(!node.is_branch());
        assert_eq!(node.get_path(), Some(&path));
        assert_eq!(node.get_extension_child(), Some(child_id));
    }

    #[test]
    fn test_branch_node() {
        let mut children = [None; 16];
        children[0] = Some(keccak256(b"child0"));
        children[5] = Some(keccak256(b"child5"));
        let value = Some(b"branch_value".to_vec());

        let node = Node::new_branch(children, value.clone());

        assert!(!node.is_empty());
        assert!(!node.is_leaf());
        assert!(!node.is_extension());
        assert!(node.is_branch());
        assert_eq!(node.get_value(), value.as_ref());
        assert_eq!(node.get_branch_children(), Some(&children));
    }

    #[test]
    fn test_node_encoding_decoding() {
        let path = CompactPath::new(vec![1, 2, 3], true);
        let value = b"test_value".to_vec();
        let original_node = Node::new_leaf(path, value);

        let encoded = original_node.encode();
        let decoded_node = Node::decode(&encoded).unwrap();

        assert_eq!(original_node, decoded_node);
    }

    #[test]
    fn test_branch_node_with_child_update() {
        let mut children = [None; 16];
        children[0] = Some(keccak256(b"child0"));
        let node = Node::new_branch(children, None);

        let new_child = keccak256(b"new_child");
        let updated_node = node.with_branch_child(5, Some(new_child)).unwrap();

        let updated_children = updated_node.get_branch_children().unwrap();
        assert_eq!(updated_children[0], Some(keccak256(b"child0")));
        assert_eq!(updated_children[5], Some(new_child));
    }

    #[test]
    fn test_node_id_consistency() {
        let path = CompactPath::new(vec![1, 2, 3], true);
        let value = b"test_value".to_vec();

        let node1 = Node::new_leaf(path.clone(), value.clone());
        let node2 = Node::new_leaf(path, value);

        assert_eq!(node1.id, node2.id);
    }

    // Additional tests for edge cases and error conditions
    #[test]
    fn test_branch_node_invalid_index() {
        let children = [None; 16];
        let node = Node::new_branch(children, None);

        assert!(matches!(node.with_branch_child(16, Some(keccak256(b"child"))), Err(MPTError::InvalidNodeType)));
    }

    #[test]
    fn test_branch_node_value_update() {
        let children = [None; 16];
        let node = Node::new_branch(children, None);
        let new_value = Some(b"new_value".to_vec());

        let updated_node = node.with_branch_value(new_value.clone()).unwrap();
        assert_eq!(updated_node.get_value(), new_value.as_ref());
    }

    #[test]
    fn test_invalid_node_type_operations() {
        let node = Node::new_empty();

        assert!(matches!(node.with_branch_value(Some(vec![1, 2, 3])), Err(MPTError::InvalidNodeType)));
        assert!(matches!(node.with_branch_child(0, Some(keccak256(b"child"))), Err(MPTError::InvalidNodeType)));
    }

    #[test]
    fn test_node_serialization_consistency() {
        let path = CompactPath::new(vec![1, 2, 3], true);
        let value = b"test_value".to_vec();
        let node = Node::new_leaf(path, value);

        let encoded1 = node.encode();
        let encoded2 = node.encode();
        assert_eq!(encoded1, encoded2);

        let decoded1 = Node::decode(&encoded1).unwrap();
        let decoded2 = Node::decode(&encoded2).unwrap();
        assert_eq!(decoded1, decoded2);
        assert_eq!(decoded1.id, node.id);
    }

    #[test]
    fn test_branch_node_multiple_updates() {
        let mut children = [None; 16];
        let node = Node::new_branch(children, None);

        // Update multiple children
        let child1 = keccak256(b"child1");
        let child2 = keccak256(b"child2");
        let child3 = keccak256(b"child3");

        let node1 = node.with_branch_child(1, Some(child1)).unwrap();
        let node2 = node1.with_branch_child(2, Some(child2)).unwrap();
        let node3 = node2.with_branch_child(3, Some(child3)).unwrap();

        let final_children = node3.get_branch_children().unwrap();
        assert_eq!(final_children[1], Some(child1));
        assert_eq!(final_children[2], Some(child2));
        assert_eq!(final_children[3], Some(child3));
    }
}
