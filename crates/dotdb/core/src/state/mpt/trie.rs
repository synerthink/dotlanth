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

//! Merkle Patricia Trie Implementation
//!
//! This module provides a complete implementation of the Merkle Patricia Trie (MPT),
//! a cryptographically authenticated data structure that can be used to store all
//! (key, value) bindings. The MPT combines the benefits of a Merkle tree and a
//! Patricia trie to provide efficient storage and verification of state data.
//!
//! # Features
//!
//! - Efficient key-value storage and retrieval
//! - Cryptographic verification of state
//! - Support for all standard trie operations (get, put, delete)
//! - Proof generation and verification
//! - Thread-safe operations with RwLock
//!
//! # Performance Considerations
//!
//! - Optimized node storage and retrieval
//! - Efficient path compression
//! - Minimal memory allocations
//! - Thread-safe concurrent access
//! - Efficient proof generation

use crate::state::mpt::lib::{CompactPath, Hash, Key, MPTError, NodeId, TrieResult, Value, common_prefix, key_to_nibbles};
use crate::state::mpt::node::{Node, NodeType};
use crate::state::mpt::proof::{ProofBuilder, StateProof};
use parking_lot::RwLock;
use std::collections::HashMap;

/// Storage interface for MPT nodes
///
/// This trait defines the interface for storing and retrieving nodes in the MPT.
/// Implementations can provide different storage backends (e.g., in-memory, disk-based).
pub trait NodeStorage {
    /// Get a node by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to retrieve
    ///
    /// # Returns
    ///
    /// A Result containing either the node or None if not found
    fn get_node(&self, id: &NodeId) -> TrieResult<Option<Node>>;

    /// Store a node
    ///
    /// # Arguments
    ///
    /// * `node` - The node to store
    fn put_node(&mut self, node: &Node) -> TrieResult<()>;

    /// Delete a node by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to delete
    fn delete_node(&mut self, id: &NodeId) -> TrieResult<()>;

    /// Check if a node exists
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to check
    ///
    /// # Returns
    ///
    /// True if the node exists, false otherwise
    fn contains_node(&self, id: &NodeId) -> bool;
}

/// In-memory storage implementation for testing
///
/// This implementation stores nodes in a HashMap, making it suitable for testing
/// and small-scale usage. For production use, consider implementing a persistent
/// storage backend.
#[derive(Debug, Clone)]
pub struct InMemoryStorage {
    nodes: HashMap<NodeId, Node>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    /// Get the number of stored nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clear all stored nodes
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeStorage for InMemoryStorage {
    fn get_node(&self, id: &NodeId) -> TrieResult<Option<Node>> {
        Ok(self.nodes.get(id).cloned())
    }

    fn put_node(&mut self, node: &Node) -> TrieResult<()> {
        self.nodes.insert(node.id, node.clone());
        Ok(())
    }

    fn delete_node(&mut self, id: &NodeId) -> TrieResult<()> {
        self.nodes.remove(id);
        Ok(())
    }

    fn contains_node(&self, id: &NodeId) -> bool {
        self.nodes.contains_key(id)
    }
}

/// Merkle Patricia Trie implementation
///
/// This is the main trie implementation that provides all the core functionality
/// for storing and retrieving key-value pairs in a cryptographically authenticated
/// manner.
pub struct MerklePatriciaTrie<S: NodeStorage> {
    storage: RwLock<S>,
    root: RwLock<NodeId>,
}

impl<S: NodeStorage + Clone> Clone for MerklePatriciaTrie<S> {
    fn clone(&self) -> Self {
        Self {
            storage: RwLock::new(self.storage.read().clone()),
            root: RwLock::new(*self.root.read()),
        }
    }
}

impl<S: NodeStorage> MerklePatriciaTrie<S> {
    /// Create a new MPT with given storage
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend to use
    ///
    /// # Returns
    ///
    /// A new MPT instance with an empty root node
    pub fn new(storage: S) -> Self {
        let empty_node = Node::new_empty();
        let root_id = empty_node.id;

        let trie = Self {
            storage: RwLock::new(storage),
            root: RwLock::new(root_id),
        };

        // Store the empty root node
        let _ = trie.storage.write().put_node(&empty_node);
        trie
    }

    /// Get the root hash
    ///
    /// # Returns
    ///
    /// The hash of the root node
    pub fn root_hash(&self) -> Hash {
        *self.root.read()
    }

    /// Get value for a key
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Returns
    ///
    /// A Result containing either the value or None if not found
    pub fn get(&self, key: &Key) -> TrieResult<Option<Value>> {
        let root_id = *self.root.read();
        let storage = self.storage.read();
        let key_nibbles = key_to_nibbles(key);
        self.get_recursive(&*storage, root_id, &key_nibbles)
    }

    fn get_recursive(&self, storage: &S, node_id: NodeId, key_nibbles: &[u8]) -> TrieResult<Option<Value>> {
        let node = storage.get_node(&node_id)?.ok_or(MPTError::NodeNotFound(node_id))?;

        match &node.node_type {
            NodeType::Empty => Ok(None),

            NodeType::Leaf { path, value } => {
                if path.nibbles == key_nibbles {
                    Ok(Some(value.clone()))
                } else {
                    Ok(None)
                }
            }

            NodeType::Extension { path, child } => {
                if key_nibbles.len() < path.nibbles.len() {
                    return Ok(None);
                }

                if key_nibbles[..path.nibbles.len()] == path.nibbles[..] {
                    self.get_recursive(storage, *child, &key_nibbles[path.nibbles.len()..])
                } else {
                    Ok(None)
                }
            }

            NodeType::Branch { children, value } => {
                if key_nibbles.is_empty() {
                    Ok(value.clone())
                } else {
                    let nibble = key_nibbles[0] as usize;
                    if nibble >= 16 {
                        return Err(MPTError::PathTraversalError);
                    }

                    match children[nibble] {
                        Some(child_id) => self.get_recursive(storage, child_id, &key_nibbles[1..]),
                        None => Ok(None),
                    }
                }
            }
        }
    }

    /// Insert or update a key-value pair
    pub fn put(&mut self, key: Key, value: Value) -> TrieResult<()> {
        let key_nibbles = key_to_nibbles(&key);
        let root_id = *self.root.read();
        let new_root = self.put_recursive(root_id, &key_nibbles, value)?;
        *self.root.write() = new_root;
        Ok(())
    }

    fn put_recursive(&mut self, node_id: NodeId, key_nibbles: &[u8], value: Value) -> TrieResult<NodeId> {
        let storage = self.storage.read();
        let node = storage.get_node(&node_id)?.ok_or(MPTError::NodeNotFound(node_id))?;
        drop(storage);

        match &node.node_type {
            NodeType::Empty => {
                let path = CompactPath::new(key_nibbles.to_vec(), true);
                let new_node = Node::new_leaf(path, value);
                let new_id = new_node.id;
                self.storage.write().put_node(&new_node)?;
                Ok(new_id)
            }

            NodeType::Leaf { path, value: old_value } => {
                if path.nibbles == key_nibbles {
                    // Update existing leaf
                    let new_node = Node::new_leaf(path.clone(), value);
                    let new_id = new_node.id;
                    self.storage.write().put_node(&new_node)?;
                    Ok(new_id)
                } else {
                    // Split the leaf
                    self.split_leaf_node(&path.nibbles, old_value, key_nibbles, value)
                }
            }

            NodeType::Extension { path, child } => {
                let common_len = common_prefix(&path.nibbles, key_nibbles);

                if common_len == path.nibbles.len() {
                    // Continue down the extension
                    let new_child = self.put_recursive(*child, &key_nibbles[common_len..], value)?;
                    let new_node = Node::new_extension(path.clone(), new_child);
                    let new_id = new_node.id;
                    self.storage.write().put_node(&new_node)?;
                    Ok(new_id)
                } else {
                    // Split the extension
                    self.split_extension_node(path, *child, key_nibbles, value, common_len)
                }
            }

            NodeType::Branch { children, value: branch_value } => {
                if key_nibbles.is_empty() {
                    // Update branch value
                    let new_node = Node::new_branch(*children, Some(value));
                    let new_id = new_node.id;
                    self.storage.write().put_node(&new_node)?;
                    Ok(new_id)
                } else {
                    // Continue down a branch
                    let nibble = key_nibbles[0] as usize;
                    if nibble >= 16 {
                        return Err(MPTError::PathTraversalError);
                    }

                    let child_id = children[nibble].unwrap_or_else(|| Node::new_empty().id);
                    let new_child = self.put_recursive(child_id, &key_nibbles[1..], value)?;
                    let new_node = node.with_branch_child(nibble, Some(new_child))?;
                    let new_id = new_node.id;
                    self.storage.write().put_node(&new_node)?;
                    Ok(new_id)
                }
            }
        }
    }

    fn split_leaf_node(&mut self, old_path: &[u8], old_value: &Value, new_path: &[u8], new_value: Value) -> TrieResult<NodeId> {
        let common_len = common_prefix(old_path, new_path);

        if common_len == 0 {
            // Create branch node directly
            let mut children = [None; 16];

            if old_path.is_empty() {
                // Old leaf becomes branch value
                if !new_path.is_empty() {
                    let nibble = new_path[0] as usize;
                    let new_leaf_path = CompactPath::new(new_path[1..].to_vec(), true);
                    let new_leaf = Node::new_leaf(new_leaf_path, new_value);
                    children[nibble] = Some(new_leaf.id);
                    self.storage.write().put_node(&new_leaf)?;
                }

                let branch = Node::new_branch(children, Some(old_value.clone()));
                let branch_id = branch.id;
                self.storage.write().put_node(&branch)?;
                Ok(branch_id)
            } else {
                let old_nibble = old_path[0] as usize;
                let new_nibble = new_path[0] as usize;

                let old_leaf_path = CompactPath::new(old_path[1..].to_vec(), true);
                let old_leaf = Node::new_leaf(old_leaf_path, old_value.clone());
                children[old_nibble] = Some(old_leaf.id);
                self.storage.write().put_node(&old_leaf)?;

                let new_leaf_path = CompactPath::new(new_path[1..].to_vec(), true);
                let new_leaf = Node::new_leaf(new_leaf_path, new_value);
                children[new_nibble] = Some(new_leaf.id);
                self.storage.write().put_node(&new_leaf)?;

                let branch = Node::new_branch(children, None);
                let branch_id = branch.id;
                self.storage.write().put_node(&branch)?;
                Ok(branch_id)
            }
        } else {
            // Create extension node
            let common_path = CompactPath::new(old_path[..common_len].to_vec(), false);
            let branch_id = self.split_leaf_node(&old_path[common_len..], old_value, &new_path[common_len..], new_value)?;

            let extension = Node::new_extension(common_path, branch_id);
            let extension_id = extension.id;
            self.storage.write().put_node(&extension)?;
            Ok(extension_id)
        }
    }

    fn split_extension_node(&mut self, path: &CompactPath, child: NodeId, key_nibbles: &[u8], value: Value, common_len: usize) -> TrieResult<NodeId> {
        let mut children = [None; 16];

        // Handle the existing extension
        if common_len + 1 == path.nibbles.len() {
            // Extension becomes direct child
            children[path.nibbles[common_len] as usize] = Some(child);
        } else {
            // Create new extension for remaining path
            let remaining_path = CompactPath::new(path.nibbles[common_len + 1..].to_vec(), false);
            let new_extension = Node::new_extension(remaining_path, child);
            children[path.nibbles[common_len] as usize] = Some(new_extension.id);
            self.storage.write().put_node(&new_extension)?;
        }

        // Handle the new key
        if common_len == key_nibbles.len() {
            // New key ends here, becomes branch value
            let branch = Node::new_branch(children, Some(value));
            let branch_id = branch.id;
            self.storage.write().put_node(&branch)?;

            if common_len == 0 {
                Ok(branch_id)
            } else {
                let common_path = CompactPath::new(key_nibbles[..common_len].to_vec(), false);
                let extension = Node::new_extension(common_path, branch_id);
                let extension_id = extension.id;
                self.storage.write().put_node(&extension)?;
                Ok(extension_id)
            }
        } else {
            // Create new leaf for remaining key
            let remaining_key = &key_nibbles[common_len + 1..];
            let new_leaf_path = CompactPath::new(remaining_key.to_vec(), true);
            let new_leaf = Node::new_leaf(new_leaf_path, value);
            children[key_nibbles[common_len] as usize] = Some(new_leaf.id);
            self.storage.write().put_node(&new_leaf)?;

            let branch = Node::new_branch(children, None);
            let branch_id = branch.id;
            self.storage.write().put_node(&branch)?;

            if common_len == 0 {
                Ok(branch_id)
            } else {
                let common_path = CompactPath::new(key_nibbles[..common_len].to_vec(), false);
                let extension = Node::new_extension(common_path, branch_id);
                let extension_id = extension.id;
                self.storage.write().put_node(&extension)?;
                Ok(extension_id)
            }
        }
    }

    /// Delete a key from the trie
    pub fn delete(&mut self, key: &Key) -> TrieResult<bool> {
        let key_nibbles = key_to_nibbles(key);
        let root_id = *self.root.read();

        match self.delete_recursive(root_id, &key_nibbles)? {
            Some(new_root) => {
                *self.root.write() = new_root;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn delete_recursive(&mut self, node_id: NodeId, key_nibbles: &[u8]) -> TrieResult<Option<NodeId>> {
        let storage = self.storage.read();
        let node = storage.get_node(&node_id)?.ok_or(MPTError::NodeNotFound(node_id))?;
        drop(storage);

        match &node.node_type {
            NodeType::Empty => Ok(None),

            NodeType::Leaf { path, .. } => {
                if path.nibbles == key_nibbles {
                    Ok(Some(Node::new_empty().id))
                } else {
                    Ok(None)
                }
            }

            NodeType::Extension { path, child } => {
                if key_nibbles.len() < path.nibbles.len() || key_nibbles[..path.nibbles.len()] != path.nibbles[..] {
                    return Ok(None);
                }

                match self.delete_recursive(*child, &key_nibbles[path.nibbles.len()..])? {
                    Some(new_child) => {
                        let new_node = Node::new_extension(path.clone(), new_child);
                        let new_id = new_node.id;
                        self.storage.write().put_node(&new_node)?;
                        Ok(Some(new_id))
                    }
                    None => Ok(None),
                }
            }

            NodeType::Branch { children, value } => {
                if key_nibbles.is_empty() {
                    if value.is_some() {
                        let new_node = Node::new_branch(*children, None);
                        let new_id = new_node.id;
                        self.storage.write().put_node(&new_node)?;
                        Ok(Some(new_id))
                    } else {
                        Ok(None)
                    }
                } else {
                    let nibble = key_nibbles[0] as usize;
                    if nibble >= 16 {
                        return Err(MPTError::PathTraversalError);
                    }

                    match children[nibble] {
                        Some(child_id) => match self.delete_recursive(child_id, &key_nibbles[1..])? {
                            Some(new_child) => {
                                let new_node = node.with_branch_child(nibble, Some(new_child))?;
                                let new_id = new_node.id;
                                self.storage.write().put_node(&new_node)?;
                                Ok(Some(new_id))
                            }
                            None => {
                                let new_node = node.with_branch_child(nibble, None)?;
                                let new_id = new_node.id;
                                self.storage.write().put_node(&new_node)?;
                                Ok(Some(new_id))
                            }
                        },
                        None => Ok(None),
                    }
                }
            }
        }
    }

    /// Generate a proof for a key
    pub fn get_proof(&self, key: &Key) -> TrieResult<StateProof> {
        let root_id = *self.root.read();
        let storage = self.storage.read();
        let key_nibbles = key_to_nibbles(key);
        let mut proof_builder = ProofBuilder::new();

        let value = self.get_proof_recursive(&*storage, root_id, &key_nibbles, &mut proof_builder)?;
        Ok(proof_builder.build(key.clone(), value, root_id))
    }

    fn get_proof_recursive(&self, storage: &S, node_id: NodeId, key_nibbles: &[u8], proof_builder: &mut ProofBuilder) -> TrieResult<Option<Value>> {
        let node = storage.get_node(&node_id)?.ok_or(MPTError::NodeNotFound(node_id))?;

        proof_builder.add_node(&node);

        match &node.node_type {
            NodeType::Empty => Ok(None),

            NodeType::Leaf { path, value } => {
                if path.nibbles == key_nibbles {
                    Ok(Some(value.clone()))
                } else {
                    Ok(None)
                }
            }

            NodeType::Extension { path, child } => {
                if key_nibbles.len() < path.nibbles.len() || key_nibbles[..path.nibbles.len()] != path.nibbles[..] {
                    return Ok(None);
                }

                self.get_proof_recursive(storage, *child, &key_nibbles[path.nibbles.len()..], proof_builder)
            }

            NodeType::Branch { children, value } => {
                if key_nibbles.is_empty() {
                    Ok(value.clone())
                } else {
                    let nibble = key_nibbles[0] as usize;
                    if nibble >= 16 {
                        return Err(MPTError::PathTraversalError);
                    }

                    match children[nibble] {
                        Some(child_id) => self.get_proof_recursive(storage, child_id, &key_nibbles[1..], proof_builder),
                        None => Ok(None),
                    }
                }
            }
        }
    }

    /// Verify a proof against this trie's root
    pub fn verify_proof(&self, proof: &StateProof) -> TrieResult<bool> {
        let root_id = *self.root.read();
        if proof.root_hash != root_id {
            return Ok(false);
        }
        proof.verify()
    }

    /// Get all keys in the trie
    ///
    /// # Returns
    ///
    /// A Result containing a vector of all keys
    pub fn get_all_keys(&self) -> TrieResult<Vec<Key>> {
        let mut keys = Vec::new();
        let root_id = *self.root.read();
        let storage = self.storage.read();
        self.collect_keys_recursive(&*storage, root_id, Vec::new(), &mut keys)?;
        Ok(keys)
    }

    fn collect_keys_recursive(&self, storage: &S, node_id: NodeId, prefix: Vec<u8>, keys: &mut Vec<Key>) -> TrieResult<()> {
        use crate::state::mpt::lib::nibbles_to_key;
        let node = storage.get_node(&node_id)?.ok_or(MPTError::NodeNotFound(node_id))?;
        match &node.node_type {
            NodeType::Empty => Ok(()),
            NodeType::Leaf { path, .. } => {
                let mut full_key = prefix.clone();
                full_key.extend_from_slice(&path.nibbles);
                keys.push(nibbles_to_key(&full_key));
                Ok(())
            }
            NodeType::Extension { path, child } => {
                let mut new_prefix = prefix.clone();
                new_prefix.extend_from_slice(&path.nibbles);
                self.collect_keys_recursive(storage, *child, new_prefix, keys)
            }
            NodeType::Branch { children, value } => {
                if value.is_some() {
                    // Branch node can have value at this node (empty key)
                    keys.push(nibbles_to_key(&prefix));
                }
                for (i, child_opt) in children.iter().enumerate() {
                    if let Some(child_id) = child_opt {
                        let mut new_prefix = prefix.clone();
                        new_prefix.push(i as u8);
                        self.collect_keys_recursive(storage, *child_id, new_prefix, keys)?;
                    }
                }
                Ok(())
            }
        }
    }

    /// Get the number of key-value pairs in the trie
    ///
    /// # Returns
    ///
    /// The number of key-value pairs
    pub fn len(&self) -> TrieResult<usize> {
        Ok(self.get_all_keys()?.len())
    }

    /// Check if the trie is empty
    ///
    /// # Returns
    ///
    /// True if the trie is empty, false otherwise
    pub fn is_empty(&self) -> bool {
        self.root_hash() == Node::new_empty().id
    }

    /// Set the root node of the trie
    ///
    /// # Arguments
    ///
    /// * `root_hash` - The hash of the new root node
    pub fn set_root(&mut self, root_hash: Hash) {
        *self.root.write() = root_hash;
    }

    /// Add metadata to the trie
    ///
    /// # Arguments
    ///
    /// * `key` - The metadata key
    /// * `value` - The metadata value
    pub fn add_metadata(&mut self, key: String, value: String) -> TrieResult<()> {
        // Store metadata in a special node
        let metadata_key = format!("metadata:{key}");
        self.put(metadata_key.as_bytes().to_vec(), value.as_bytes().to_vec())
    }

    /// Get a clone of the underlying storage
    pub fn get_storage_clone(&self) -> S
    where
        S: Clone,
    {
        self.storage.read().clone()
    }

    /// Get mutable access to the underlying storage for direct operations
    ///
    /// # Returns
    ///
    /// A mutable reference to the storage wrapped in RwLock
    pub fn get_storage_mut(&mut self) -> &mut RwLock<S> {
        &mut self.storage
    }
}

impl MerklePatriciaTrie<InMemoryStorage> {
    /// Create a new in-memory MPT
    ///
    /// # Returns
    ///
    /// A new MPT instance with in-memory storage
    pub fn new_in_memory() -> Self {
        Self::new(InMemoryStorage::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_empty_trie() {
        let trie = MerklePatriciaTrie::new_in_memory();
        assert!(trie.is_empty());
        assert_eq!(trie.len().unwrap(), 0);
        assert_eq!(trie.get(&b"test".to_vec()).unwrap(), None);
    }

    #[test]
    fn test_single_key_value() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        trie.put(b"test".to_vec(), b"value".to_vec()).unwrap();
        assert!(!trie.is_empty());
        assert_eq!(trie.len().unwrap(), 1);
        assert_eq!(trie.get(&b"test".to_vec()).unwrap(), Some(b"value".to_vec()));
    }

    #[test]
    fn test_multiple_keys() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let pairs = vec![(b"key1".to_vec(), b"value1".to_vec()), (b"key2".to_vec(), b"value2".to_vec()), (b"key3".to_vec(), b"value3".to_vec())];

        for (key, value) in pairs.clone() {
            trie.put(key, value).unwrap();
        }

        assert_eq!(trie.len().unwrap(), pairs.len());
        for (key, value) in pairs {
            assert_eq!(trie.get(&key).unwrap(), Some(value));
        }
    }

    #[test]
    fn test_key_update() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let key = b"test".to_vec();
        let value1 = b"value1".to_vec();
        let value2 = b"value2".to_vec();

        trie.put(key.clone(), value1.clone()).unwrap();
        assert_eq!(trie.get(&key).unwrap(), Some(value1));

        trie.put(key.clone(), value2.clone()).unwrap();
        assert_eq!(trie.get(&key).unwrap(), Some(value2));
    }

    #[test]
    fn test_key_deletion() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let key = b"test".to_vec();
        let value = b"value".to_vec();

        trie.put(key.clone(), value).unwrap();
        assert!(trie.delete(&key).unwrap());
        assert_eq!(trie.get(&key).unwrap(), None);
        assert!(trie.is_empty());
    }

    #[test]
    fn test_proof_generation_and_verification() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let key = b"test".to_vec();
        let value = b"value".to_vec();

        trie.put(key.clone(), value.clone()).unwrap();
        let proof = trie.get_proof(&key).unwrap();
        assert!(proof.verify().unwrap());
        assert_eq!(proof.value, Some(value));
    }

    #[test]
    fn test_proof_for_nonexistent_key() {
        let trie = MerklePatriciaTrie::new_in_memory();
        let key = b"nonexistent".to_vec();
        let proof = trie.get_proof(&key).unwrap();
        assert!(!proof.verify().unwrap());
        assert_eq!(proof.value, None);
    }

    #[test]
    fn test_common_prefix_keys() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let pairs = vec![
            (b"test1".to_vec(), b"value1".to_vec()),
            (b"test2".to_vec(), b"value2".to_vec()),
            (b"test3".to_vec(), b"value3".to_vec()),
        ];

        for (key, value) in pairs.clone() {
            trie.put(key, value).unwrap();
        }

        for (key, value) in pairs {
            assert_eq!(trie.get(&key).unwrap(), Some(value));
        }
    }

    #[test]
    fn test_root_hash_consistency() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let initial_hash = trie.root_hash();

        trie.put(b"test1".to_vec(), b"value1".to_vec()).unwrap();
        let hash1 = trie.root_hash();
        assert_ne!(initial_hash, hash1);

        trie.put(b"test2".to_vec(), b"value2".to_vec()).unwrap();
        let hash2 = trie.root_hash();
        assert_ne!(hash1, hash2);

        trie.delete(&b"test1".to_vec()).unwrap();
        let hash3 = trie.root_hash();
        assert_ne!(hash2, hash3);
    }

    #[test]
    fn test_empty_key() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let value = b"value".to_vec();

        trie.put(Vec::new(), value.clone()).unwrap();
        assert_eq!(trie.get(&Vec::new()).unwrap(), Some(value));
    }

    #[test]
    fn test_get_all_keys() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let pairs = vec![(b"key1".to_vec(), b"value1".to_vec()), (b"key2".to_vec(), b"value2".to_vec()), (b"key3".to_vec(), b"value3".to_vec())];

        for (key, value) in pairs.clone() {
            trie.put(key, value).unwrap();
        }

        let keys: HashSet<_> = trie.get_all_keys().unwrap().into_iter().collect();
        let expected_keys: HashSet<_> = pairs.into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys, expected_keys);
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let trie = Arc::new(Mutex::new(MerklePatriciaTrie::new_in_memory()));
        let mut handles = vec![];

        for i in 0..10 {
            let trie_clone = Arc::clone(&trie);
            handles.push(thread::spawn(move || {
                let key = format!("key{}", i).into_bytes();
                let value = format!("value{}", i).into_bytes();
                trie_clone.lock().unwrap().put(key, value).unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(trie.lock().unwrap().len().unwrap(), 10);
    }

    #[test]
    fn test_large_key_value_pairs() {
        let mut trie = MerklePatriciaTrie::new_in_memory();
        let key = vec![0u8; 1024]; // 1KB key
        let value = vec![1u8; 1024 * 1024]; // 1MB value

        trie.put(key.clone(), value.clone()).unwrap();
        assert_eq!(trie.get(&key).unwrap(), Some(value));
    }

    #[test]
    fn test_storage_operations() {
        let mut storage = InMemoryStorage::new();
        let node = Node::new_empty();

        assert!(!storage.contains_node(&node.id));
        storage.put_node(&node).unwrap();
        assert!(storage.contains_node(&node.id));
        assert_eq!(storage.get_node(&node.id).unwrap(), Some(node.clone()));
        storage.delete_node(&node.id).unwrap();
        assert!(!storage.contains_node(&node.id));
    }
}
