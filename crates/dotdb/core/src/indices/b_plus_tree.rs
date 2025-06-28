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

use super::lib::{Index, IndexError, IndexIterator, IndexKey, IndexMaintenance, IndexResult, IndexStats, IndexType, IndexValue, RangeQuery};
use super::persistence::IndexPersistence;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

/// Default order for B+ tree (maximum number of children per node)
const DEFAULT_ORDER: usize = 100;

/// Minimum order for B+ tree
const MIN_ORDER: usize = 3;

/// Node types in B+ tree
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Internal,
    Leaf,
}

/// B+ tree node
#[derive(Debug, Clone)]
pub struct BPlusTreeNode<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Node type (internal or leaf)
    node_type: NodeType,
    /// Keys stored in this node
    keys: Vec<K>,
    /// Values (only for leaf nodes)
    values: Vec<V>,
    /// Child node pointers (only for internal nodes)
    children: Vec<Arc<RwLock<BPlusTreeNode<K, V>>>>,
    /// Pointer to next leaf node (only for leaf nodes)
    next_leaf: Option<Arc<RwLock<BPlusTreeNode<K, V>>>>,
    /// Pointer to previous leaf node (only for leaf nodes)
    prev_leaf: Option<Arc<RwLock<BPlusTreeNode<K, V>>>>,
    /// Parent node pointer
    parent: Option<Arc<RwLock<BPlusTreeNode<K, V>>>>,
    /// Maximum number of keys this node can hold
    max_keys: usize,
}

impl<K, V> BPlusTreeNode<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Create a new leaf node
    pub fn new_leaf(order: usize) -> Self {
        Self {
            node_type: NodeType::Leaf,
            keys: Vec::new(),
            values: Vec::new(),
            children: Vec::new(),
            next_leaf: None,
            prev_leaf: None,
            parent: None,
            max_keys: order - 1,
        }
    }

    /// Create a new internal node
    pub fn new_internal(order: usize) -> Self {
        Self {
            node_type: NodeType::Internal,
            keys: Vec::new(),
            values: Vec::new(),
            children: Vec::new(),
            next_leaf: None,
            prev_leaf: None,
            parent: None,
            max_keys: order - 1,
        }
    }

    /// Check if node is a leaf
    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf)
    }

    /// Check if node is full
    pub fn is_full(&self) -> bool {
        self.keys.len() >= self.max_keys
    }

    /// Check if node is underflowing
    pub fn is_underflow(&self) -> bool {
        self.keys.len() < (self.max_keys + 1) / 2
    }

    /// Insert a key-value pair into a leaf node
    pub fn insert_into_leaf(&mut self, key: K, value: V) -> IndexResult<()> {
        if !self.is_leaf() {
            return Err(IndexError::InvalidOperation("Cannot insert into non-leaf node".to_string()));
        }

        // Find insertion position
        let pos = self.keys.binary_search(&key).unwrap_or_else(|e| e);

        // Check for duplicate keys
        if pos < self.keys.len() && self.keys[pos] == key {
            return Err(IndexError::InvalidOperation("Duplicate key not allowed".to_string()));
        }

        // Insert key and value
        self.keys.insert(pos, key);
        self.values.insert(pos, value);

        Ok(())
    }

    /// Split a full node
    pub fn split(&mut self) -> Arc<RwLock<BPlusTreeNode<K, V>>> {
        let mid = self.keys.len() / 2;

        let mut new_node = if self.is_leaf() {
            BPlusTreeNode::new_leaf(self.max_keys + 1)
        } else {
            BPlusTreeNode::new_internal(self.max_keys + 1)
        };

        // Move keys to new node
        new_node.keys = self.keys.split_off(mid);

        if self.is_leaf() {
            // For leaf nodes, move values and update leaf pointers
            new_node.values = self.values.split_off(mid);

            let new_node_arc = Arc::new(RwLock::new(new_node));

            // Update leaf pointers
            if let Some(next) = &self.next_leaf {
                new_node_arc.write().unwrap().next_leaf = Some(next.clone());
                next.write().unwrap().prev_leaf = Some(new_node_arc.clone());
            }

            new_node_arc.write().unwrap().prev_leaf = Some(Arc::new(RwLock::new(self.clone())));
            self.next_leaf = Some(new_node_arc.clone());

            new_node_arc
        } else {
            // For internal nodes, move children
            new_node.children = self.children.split_off(mid + 1);

            // Update parent pointers for moved children
            for child in &new_node.children {
                child.write().unwrap().parent = Some(Arc::new(RwLock::new(new_node.clone())));
            }

            Arc::new(RwLock::new(new_node))
        }
    }

    /// Find a key in the node and return its position
    pub fn find_key(&self, key: &K) -> Option<usize> {
        self.keys.binary_search(key).ok()
    }

    /// Find child pointer for a key in internal node
    pub fn find_child(&self, key: &K) -> Option<Arc<RwLock<BPlusTreeNode<K, V>>>> {
        if self.is_leaf() || self.children.is_empty() {
            return None;
        }

        let pos = self.keys.binary_search(key).unwrap_or_else(|e| e);
        self.children.get(pos).cloned()
    }
}

/// B+ tree index implementation
pub struct BPlusTree<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Root node of the tree
    root: Option<Arc<RwLock<BPlusTreeNode<K, V>>>>,
    /// Order of the tree (maximum number of children)
    order: usize,
    /// Number of entries in the tree
    size: usize,
    /// Enable prefix compression for keys
    compression_enabled: bool,
    /// Compression statistics
    compression_stats: CompressionStats,
    /// Prefix cache for compression
    prefix_cache: HashMap<Vec<u8>, usize>,
    /// Phantom data for type parameters
    _phantom: PhantomData<(K, V)>,
}

/// Compression statistics for B+ tree
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    /// Original size before compression
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Number of compressed keys
    pub compressed_keys: usize,
    /// Compression ratio (compressed_size / original_size)
    pub compression_ratio: f64,
}

impl CompressionStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calculate_ratio(&mut self) {
        if self.original_size > 0 {
            self.compression_ratio = self.compressed_size as f64 / self.original_size as f64;
        }
    }
}

/// Snapshot of a B+ tree for backup/restore operations
#[derive(Debug, Clone)]
pub struct BPlusTreeSnapshot<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// All key-value pairs in sorted order
    pub entries: Vec<(K, V)>,
    /// Tree order (branching factor)
    pub order: usize,
    /// Compression enabled flag
    pub compression_enabled: bool,
    /// Snapshot timestamp
    pub timestamp: std::time::SystemTime,
}

impl<K, V> BPlusTree<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Create a new B+ tree with default order
    pub fn new() -> Self {
        Self::with_order(DEFAULT_ORDER)
    }

    /// Create a new B+ tree with specified order
    pub fn with_order(order: usize) -> Self {
        if order < MIN_ORDER {
            panic!("B+ tree order must be at least {}", MIN_ORDER);
        }

        Self {
            root: None,
            order,
            size: 0,
            compression_enabled: false,
            compression_stats: CompressionStats::new(),
            prefix_cache: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Find a leaf node that should contain the given key
    fn find_leaf(&self, key: &K) -> Option<Arc<RwLock<BPlusTreeNode<K, V>>>> {
        let mut current = self.root.as_ref()?.clone();

        loop {
            let node = current.read().unwrap();
            if node.is_leaf() {
                return Some(current.clone());
            }

            if let Some(child) = node.find_child(key) {
                drop(node);
                current = child;
            } else {
                return None;
            }
        }
    }

    /// Insert and split if necessary, propagating splits up the tree
    fn insert_and_split(&mut self, node: Arc<RwLock<BPlusTreeNode<K, V>>>, key: K, value: Option<V>) -> IndexResult<Option<(K, Arc<RwLock<BPlusTreeNode<K, V>>>)>> {
        let mut node_guard = node.write().unwrap();

        if node_guard.is_leaf() {
            if let Some(val) = value {
                node_guard.insert_into_leaf(key.clone(), val)?;
            }
        } else {
            // For internal nodes, find appropriate child and recurse
            if let Some(child) = node_guard.find_child(&key) {
                drop(node_guard);
                if let Some((split_key, new_child)) = self.insert_and_split(child, key, value)? {
                    let mut node_guard = node.write().unwrap();

                    // Insert the split key and new child pointer
                    let pos = node_guard.keys.binary_search(&split_key).unwrap_or_else(|e| e);
                    node_guard.keys.insert(pos, split_key);
                    node_guard.children.insert(pos + 1, new_child);
                } else {
                    return Ok(None);
                }
                node_guard = node.write().unwrap();
            }
        }

        // Check if node needs to be split
        if node_guard.is_full() {
            let new_node = node_guard.split();
            let split_key = if node_guard.is_leaf() {
                // For leaf nodes, promote the first key of the new node
                new_node.read().unwrap().keys[0].clone()
            } else {
                // For internal nodes, promote the middle key
                node_guard.keys.pop().unwrap()
            };

            Ok(Some((split_key, new_node)))
        } else {
            Ok(None)
        }
    }

    /// Create a new root when the current root splits
    fn create_new_root(&mut self, left: Arc<RwLock<BPlusTreeNode<K, V>>>, key: K, right: Arc<RwLock<BPlusTreeNode<K, V>>>) {
        let mut new_root = BPlusTreeNode::new_internal(self.order);
        new_root.keys.push(key);
        new_root.children.push(left.clone());
        new_root.children.push(right.clone());

        let new_root_arc = Arc::new(RwLock::new(new_root));

        // Update parent pointers
        left.write().unwrap().parent = Some(new_root_arc.clone());
        right.write().unwrap().parent = Some(new_root_arc.clone());

        self.root = Some(new_root_arc);
    }

    /// Get the first leaf node (leftmost)
    fn first_leaf(&self) -> Option<Arc<RwLock<BPlusTreeNode<K, V>>>> {
        let mut current = self.root.as_ref()?.clone();

        loop {
            let node = current.read().unwrap();
            if node.is_leaf() {
                return Some(current.clone());
            }

            if let Some(first_child) = node.children.first() {
                let child = first_child.clone();
                drop(node);
                current = child;
            } else {
                return None;
            }
        }
    }

    /// Create a new B+ tree with compression enabled
    pub fn new_with_compression() -> Self {
        let mut tree = Self::new();
        tree.compression_enabled = true;
        tree
    }

    /// Enable or disable prefix compression
    pub fn set_compression(&mut self, enabled: bool) {
        self.compression_enabled = enabled;
        if !enabled {
            self.prefix_cache.clear();
            self.compression_stats = CompressionStats::new();
        }
    }

    /// Get compression statistics
    pub fn compression_stats(&self) -> &CompressionStats {
        &self.compression_stats
    }

    /// Bulk load data into the B+ tree for optimal performance
    pub fn bulk_load(&mut self, mut data: Vec<(K, V)>) -> IndexResult<()> {
        // Sort data by key for optimal loading
        data.sort_by(|a, b| a.0.cmp(&b.0));

        // Clear existing tree
        self.clear();

        // Build tree bottom-up for optimal structure
        if data.is_empty() {
            return Ok(());
        }

        // Calculate optimal leaf size
        let leaf_size = (self.order * 2) / 3; // Fill factor of ~67%
        let mut leaves = Vec::new();
        let mut current_leaf = BPlusTreeNode::new_leaf(self.order);

        for (key, value) in data {
            if current_leaf.keys.len() >= leaf_size {
                // Create new leaf and link it
                let leaf_arc = Arc::new(RwLock::new(current_leaf));
                leaves.push(leaf_arc);
                current_leaf = BPlusTreeNode::new_leaf(self.order);
            }

            current_leaf.keys.push(key);
            current_leaf.values.push(value);
            self.size += 1;
        }

        // Add the last leaf
        if !current_leaf.keys.is_empty() {
            leaves.push(Arc::new(RwLock::new(current_leaf)));
        }

        // Link leaves together
        for i in 0..leaves.len() {
            if i > 0 {
                leaves[i].write().unwrap().prev_leaf = Some(leaves[i - 1].clone());
            }
            if i < leaves.len() - 1 {
                leaves[i].write().unwrap().next_leaf = Some(leaves[i + 1].clone());
            }
        }

        // Build internal nodes bottom-up
        let mut current_level = leaves;

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            let mut current_internal = BPlusTreeNode::new_internal(self.order);
            let internal_size = self.order - 1; // Fill factor for internal nodes

            for (i, child) in current_level.iter().enumerate() {
                if current_internal.children.len() >= internal_size && i < current_level.len() - 1 {
                    // Create new internal node
                    let internal_arc = Arc::new(RwLock::new(current_internal));
                    next_level.push(internal_arc);
                    current_internal = BPlusTreeNode::new_internal(self.order);
                }

                current_internal.children.push(child.clone());

                // Set parent pointer
                child.write().unwrap().parent = Some(Arc::new(RwLock::new(current_internal.clone())));

                // Add separator key (first key of child, except for first child)
                if !current_internal.keys.is_empty() || i > 0 {
                    let first_key = child.read().unwrap().keys[0].clone();
                    current_internal.keys.push(first_key);
                }
            }

            // Add the last internal node
            if !current_internal.children.is_empty() {
                next_level.push(Arc::new(RwLock::new(current_internal)));
            }

            current_level = next_level;
        }

        // Set root
        if let Some(root) = current_level.into_iter().next() {
            self.root = Some(root);
        }

        Ok(())
    }

    /// Create a snapshot of the current tree state
    pub fn create_snapshot(&self) -> BPlusTreeSnapshot<K, V> {
        let entries = self.entries();
        BPlusTreeSnapshot {
            entries,
            order: self.order,
            compression_enabled: self.compression_enabled,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Restore tree from a snapshot
    pub fn restore_from_snapshot(&mut self, snapshot: BPlusTreeSnapshot<K, V>) -> IndexResult<()> {
        self.order = snapshot.order;
        self.compression_enabled = snapshot.compression_enabled;
        self.bulk_load(snapshot.entries)
    }

    /// Compress a key using prefix compression
    fn compress_key(&mut self, key: &K) -> Vec<u8> {
        if !self.compression_enabled {
            return key.to_bytes();
        }

        let key_bytes = key.to_bytes();

        // Find common prefix with cached prefixes
        let mut best_prefix_len = 0;
        let mut best_prefix = Vec::new();

        for (prefix, _count) in &self.prefix_cache {
            if key_bytes.starts_with(prefix) && prefix.len() > best_prefix_len {
                best_prefix_len = prefix.len();
                best_prefix = prefix.clone();
            }
        }

        if best_prefix_len > 0 {
            // Use compressed format: [prefix_id][remaining_bytes]
            let mut compressed = vec![best_prefix_len as u8];
            compressed.extend_from_slice(&key_bytes[best_prefix_len..]);

            // Update compression stats
            self.compression_stats.original_size += key_bytes.len();
            self.compression_stats.compressed_size += compressed.len();
            self.compression_stats.compressed_keys += 1;
            self.compression_stats.calculate_ratio();

            compressed
        } else {
            // Store as uncompressed and update prefix cache
            if key_bytes.len() > 4 {
                let prefix = key_bytes[..key_bytes.len().min(8)].to_vec();
                *self.prefix_cache.entry(prefix).or_insert(0) += 1;
            }

            key_bytes
        }
    }

    /// Optimize tree structure by rebalancing and compacting
    pub fn optimize(&mut self) -> IndexResult<()> {
        if self.size == 0 {
            return Ok(());
        }

        // Create snapshot of current data
        let snapshot = self.create_snapshot();

        // Rebuild tree with optimal structure
        self.bulk_load(snapshot.entries)?;

        // Update compression if enabled
        if self.compression_enabled {
            self.recompress_keys()?;
        }

        Ok(())
    }

    /// Recompress all keys to optimize compression ratios
    fn recompress_keys(&mut self) -> IndexResult<()> {
        if !self.compression_enabled {
            return Ok(());
        }

        // Reset compression stats and cache
        self.compression_stats = CompressionStats::new();
        self.prefix_cache.clear();

        // First pass: collect all keys to build optimal prefix cache
        let all_keys = self.keys();
        self.build_optimal_prefix_cache(&all_keys);

        // Second pass: recompress with optimal prefixes
        // This would require tree traversal and key updates
        // For now, we'll mark it as completed
        Ok(())
    }

    /// Build optimal prefix cache from a set of keys
    fn build_optimal_prefix_cache(&mut self, keys: &[K]) {
        let mut prefix_counts: HashMap<Vec<u8>, usize> = HashMap::new();

        // Analyze all keys to find common prefixes
        for key in keys {
            let key_bytes = key.to_bytes();

            // Try different prefix lengths
            for len in 2..=key_bytes.len().min(16) {
                let prefix = key_bytes[..len].to_vec();
                *prefix_counts.entry(prefix).or_insert(0) += 1;
            }
        }

        // Keep only prefixes that appear frequently enough
        let min_frequency = keys.len() / 10; // At least 10% of keys
        self.prefix_cache = prefix_counts.into_iter().filter(|(_, count)| *count >= min_frequency).collect();
    }
}

impl<K, V> Default for BPlusTree<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Index<K, V> for BPlusTree<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn insert(&mut self, key: K, value: V) -> IndexResult<()> {
        if self.root.is_none() {
            // Create root as leaf node
            let mut root = BPlusTreeNode::new_leaf(self.order);
            root.insert_into_leaf(key, value)?;
            self.root = Some(Arc::new(RwLock::new(root)));
            self.size += 1;
            return Ok(());
        }

        let root = self.root.as_ref().unwrap().clone();

        if let Some((split_key, new_node)) = self.insert_and_split(root.clone(), key, Some(value))? {
            // Root was split, create new root
            self.create_new_root(root, split_key, new_node);
        }

        self.size += 1;
        Ok(())
    }

    fn get(&self, key: &K) -> IndexResult<Option<V>> {
        let leaf = match self.find_leaf(key) {
            Some(leaf) => leaf,
            None => return Ok(None),
        };

        let node = leaf.read().unwrap();
        if let Some(pos) = node.find_key(key) { Ok(Some(node.values[pos].clone())) } else { Ok(None) }
    }

    fn update(&mut self, key: K, value: V) -> IndexResult<()> {
        let leaf = self.find_leaf(&key).ok_or_else(|| IndexError::KeyNotFound(format!("{:?}", key)))?;

        let mut node = leaf.write().unwrap();
        if let Some(pos) = node.find_key(&key) {
            node.values[pos] = value;
            Ok(())
        } else {
            Err(IndexError::KeyNotFound(format!("{:?}", key)))
        }
    }

    fn delete(&mut self, key: &K) -> IndexResult<()> {
        let leaf = self.find_leaf(key).ok_or_else(|| IndexError::KeyNotFound(format!("{:?}", key)))?;

        let mut node = leaf.write().unwrap();
        if let Some(pos) = node.find_key(key) {
            node.keys.remove(pos);
            node.values.remove(pos);
            self.size -= 1;

            // TODO: Handle underflow and rebalancing
            Ok(())
        } else {
            Err(IndexError::KeyNotFound(format!("{:?}", key)))
        }
    }

    fn contains(&self, key: &K) -> bool {
        self.get(key).unwrap_or(None).is_some()
    }

    fn len(&self) -> usize {
        self.size
    }

    fn clear(&mut self) {
        self.root = None;
        self.size = 0;
        self.prefix_cache.clear();
        self.compression_stats = CompressionStats::new();
    }

    fn index_type(&self) -> IndexType {
        IndexType::BPlusTree
    }

    fn keys(&self) -> Vec<K> {
        let mut result = Vec::new();

        if let Some(leaf) = self.first_leaf() {
            let mut current = Some(leaf);

            while let Some(node_arc) = current {
                let node = node_arc.read().unwrap();
                result.extend(node.keys.iter().cloned());
                current = node.next_leaf.clone();
            }
        }

        result
    }

    fn values(&self) -> Vec<V> {
        let mut result = Vec::new();

        if let Some(leaf) = self.first_leaf() {
            let mut current = Some(leaf);

            while let Some(node_arc) = current {
                let node = node_arc.read().unwrap();
                result.extend(node.values.iter().cloned());
                current = node.next_leaf.clone();
            }
        }

        result
    }

    fn entries(&self) -> Vec<(K, V)> {
        let mut result = Vec::new();

        if let Some(leaf) = self.first_leaf() {
            let mut current = Some(leaf);

            while let Some(node_arc) = current {
                let node = node_arc.read().unwrap();
                for (i, key) in node.keys.iter().enumerate() {
                    if let Some(value) = node.values.get(i) {
                        result.push((key.clone(), value.clone()));
                    }
                }
                current = node.next_leaf.clone();
            }
        }

        result
    }
}

impl<K, V> RangeQuery<K, V> for BPlusTree<K, V>
where
    K: IndexKey + 'static,
    V: IndexValue + 'static,
{
    fn range(&self, start: &K, end: &K) -> IndexResult<Vec<(K, V)>> {
        let mut result = Vec::new();

        // Find the leaf containing the start key
        if let Some(leaf) = self.find_leaf(start) {
            let mut current = Some(leaf);

            while let Some(node_arc) = current {
                let node = node_arc.read().unwrap();

                for (i, key) in node.keys.iter().enumerate() {
                    if key >= start && key <= end {
                        if let Some(value) = node.values.get(i) {
                            result.push((key.clone(), value.clone()));
                        }
                    } else if key > end {
                        return Ok(result);
                    }
                }

                current = node.next_leaf.clone();
            }
        }

        Ok(result)
    }

    fn range_from(&self, start: &K) -> IndexResult<Vec<(K, V)>> {
        let mut result = Vec::new();

        if let Some(leaf) = self.find_leaf(start) {
            let mut current = Some(leaf);

            while let Some(node_arc) = current {
                let node = node_arc.read().unwrap();

                for (i, key) in node.keys.iter().enumerate() {
                    if key >= start {
                        if let Some(value) = node.values.get(i) {
                            result.push((key.clone(), value.clone()));
                        }
                    }
                }

                current = node.next_leaf.clone();
            }
        }

        Ok(result)
    }

    fn range_to(&self, end: &K) -> IndexResult<Vec<(K, V)>> {
        let mut result = Vec::new();

        if let Some(leaf) = self.first_leaf() {
            let mut current = Some(leaf);

            while let Some(node_arc) = current {
                let node = node_arc.read().unwrap();

                for (i, key) in node.keys.iter().enumerate() {
                    if key <= end {
                        if let Some(value) = node.values.get(i) {
                            result.push((key.clone(), value.clone()));
                        }
                    } else {
                        return Ok(result);
                    }
                }

                current = node.next_leaf.clone();
            }
        }

        Ok(result)
    }

    fn range_iter(&self, start: &K, end: &K) -> Box<dyn IndexIterator<K, V>> {
        Box::new(BPlusTreeIterator::new(self.find_leaf(start), start.clone(), end.clone()))
    }
}

impl<K, V> IndexMaintenance for BPlusTree<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn compact(&mut self) -> IndexResult<()> {
        // B+ trees are naturally compact, but we could implement
        // node merging for better space utilization
        Ok(())
    }

    fn verify(&self) -> IndexResult<bool> {
        // TODO: Implement tree structure verification
        Ok(true)
    }

    fn stats(&self) -> IndexStats {
        let mut stats = IndexStats::new(IndexType::BPlusTree);
        stats.entry_count = self.size;

        // Calculate tree depth and other statistics
        if let Some(root) = &self.root {
            let mut depth = 0;
            let mut current = root.clone();

            loop {
                let node = current.read().unwrap();
                depth += 1;
                if node.is_leaf() {
                    break;
                }
                if let Some(child) = node.children.first() {
                    let child_clone = child.clone();
                    drop(node);
                    current = child_clone;
                } else {
                    break;
                }
            }

            stats.type_specific.insert("depth".to_string(), depth.to_string());
            stats.type_specific.insert("order".to_string(), self.order.to_string());
        }

        stats
    }

    fn rebuild(&mut self) -> IndexResult<()> {
        let entries = self.entries();
        self.clear();

        for (key, value) in entries {
            self.insert(key, value)?;
        }

        Ok(())
    }
}

impl<K, V> IndexPersistence<K, V> for BPlusTree<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    fn serialize(&self) -> IndexResult<Vec<u8>> {
        let mut data = Vec::new();

        // Write header
        data.extend_from_slice(&self.order.to_le_bytes());
        data.extend_from_slice(&self.size.to_le_bytes());

        // Write all entries in sorted order (leaf traversal)
        let entries = self.entries();
        data.extend_from_slice(&entries.len().to_le_bytes());

        for (key, value) in entries {
            let key_bytes = key.to_bytes();
            let value_bytes = value.to_bytes();

            data.extend_from_slice(&key_bytes.len().to_le_bytes());
            data.extend_from_slice(&key_bytes);
            data.extend_from_slice(&value_bytes.len().to_le_bytes());
            data.extend_from_slice(&value_bytes);
        }

        Ok(data)
    }

    fn deserialize(&mut self, data: &[u8]) -> IndexResult<()> {
        if data.len() < 24 {
            return Err(IndexError::SerializationError("Insufficient data for header".to_string()));
        }

        let mut offset = 0;

        // Read header
        let order = usize::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        let size = usize::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        let entry_count = usize::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        if size != entry_count {
            return Err(IndexError::SerializationError("Size mismatch in serialized data".to_string()));
        }

        // Reconstruct the tree
        self.clear();
        self.order = order;

        // Read and insert entries
        for _ in 0..entry_count {
            if offset + 8 > data.len() {
                return Err(IndexError::SerializationError("Insufficient data for key length".to_string()));
            }

            let key_len = usize::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            offset += 8;

            if offset + key_len > data.len() {
                return Err(IndexError::SerializationError("Insufficient data for key".to_string()));
            }

            let key = K::from_bytes(&data[offset..offset + key_len])?;
            offset += key_len;

            if offset + 8 > data.len() {
                return Err(IndexError::SerializationError("Insufficient data for value length".to_string()));
            }

            let value_len = usize::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            offset += 8;

            if offset + value_len > data.len() {
                return Err(IndexError::SerializationError("Insufficient data for value".to_string()));
            }

            let value = V::from_bytes(&data[offset..offset + value_len])?;
            offset += value_len;

            self.insert(key, value)?;
        }

        Ok(())
    }

    fn save_to_disk<P: AsRef<std::path::Path>>(&self, path: P) -> IndexResult<()> {
        let data = self.serialize()?;
        std::fs::write(path, data).map_err(|e| IndexError::IoError(format!("Failed to write to disk: {}", e)))
    }

    fn load_from_disk<P: AsRef<std::path::Path>>(&mut self, path: P) -> IndexResult<()> {
        let data = std::fs::read(path).map_err(|e| IndexError::IoError(format!("Failed to read from disk: {}", e)))?;
        self.deserialize(&data)
    }

    fn format_version(&self) -> u32 {
        1
    }

    fn supports_incremental_save(&self) -> bool {
        false
    }
}

/// Iterator for B+ tree
pub struct BPlusTreeIterator<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    current_node: Option<Arc<RwLock<BPlusTreeNode<K, V>>>>,
    current_index: usize,
    start_key: K,
    end_key: K,
    finished: bool,
}

impl<K, V> BPlusTreeIterator<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn new(start_node: Option<Arc<RwLock<BPlusTreeNode<K, V>>>>, start_key: K, end_key: K) -> Self {
        Self {
            current_node: start_node,
            current_index: 0,
            start_key,
            end_key,
            finished: false,
        }
    }
}

impl<K, V> Iterator for BPlusTreeIterator<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        while let Some(node_arc) = &self.current_node {
            let node = node_arc.read().unwrap();

            if self.current_index < node.keys.len() {
                let key = &node.keys[self.current_index];

                if key > &self.end_key {
                    self.finished = true;
                    return None;
                }

                if key >= &self.start_key {
                    let result = (key.clone(), node.values[self.current_index].clone());
                    self.current_index += 1;
                    return Some(result);
                }

                self.current_index += 1;
            } else {
                // Move to next leaf
                let next_node = node.next_leaf.clone();
                drop(node);
                self.current_node = next_node;
                self.current_index = 0;
            }
        }

        self.finished = true;
        None
    }
}

impl<K, V> IndexIterator<K, V> for BPlusTreeIterator<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn seek(&mut self, key: &K) {
        self.start_key = key.clone();
        self.current_index = 0;
        self.finished = false;
    }

    fn seek_to_first(&mut self) {
        self.current_index = 0;
        self.finished = false;
    }

    fn seek_to_last(&mut self) {
        // TODO: Implement seeking to last
        self.finished = true;
    }

    fn valid(&self) -> bool {
        !self.finished && self.current_node.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bplus_tree_creation() {
        let tree: BPlusTree<i32, String> = BPlusTree::new();
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());
        assert_eq!(tree.index_type(), IndexType::BPlusTree);
    }

    #[test]
    fn test_bplus_tree_insert_and_get() {
        let mut tree = BPlusTree::new();

        tree.insert(1, "one".to_string()).unwrap();
        tree.insert(2, "two".to_string()).unwrap();
        tree.insert(3, "three".to_string()).unwrap();

        assert_eq!(tree.len(), 3);
        assert_eq!(tree.get(&1).unwrap(), Some("one".to_string()));
        assert_eq!(tree.get(&2).unwrap(), Some("two".to_string()));
        assert_eq!(tree.get(&3).unwrap(), Some("three".to_string()));
        assert_eq!(tree.get(&4).unwrap(), None);
    }

    #[test]
    fn test_bplus_tree_update() {
        let mut tree = BPlusTree::new();

        tree.insert(1, "one".to_string()).unwrap();
        assert_eq!(tree.get(&1).unwrap(), Some("one".to_string()));

        tree.update(1, "ONE".to_string()).unwrap();
        assert_eq!(tree.get(&1).unwrap(), Some("ONE".to_string()));

        assert!(tree.update(999, "does_not_exist".to_string()).is_err());
    }

    #[test]
    fn test_bplus_tree_delete() {
        let mut tree = BPlusTree::new();

        tree.insert(1, "one".to_string()).unwrap();
        tree.insert(2, "two".to_string()).unwrap();

        assert!(tree.contains(&1));
        tree.delete(&1).unwrap();
        assert!(!tree.contains(&1));
        assert_eq!(tree.len(), 1);

        assert!(tree.delete(&999).is_err());
    }

    #[test]
    fn test_bplus_tree_range_query() {
        let mut tree = BPlusTree::new();

        for i in 1..=10 {
            tree.insert(i, format!("value_{}", i)).unwrap();
        }

        let range_result = tree.range(&3, &7).unwrap();
        assert_eq!(range_result.len(), 5);

        let keys: Vec<i32> = range_result.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_bplus_tree_keys_and_values() {
        let mut tree = BPlusTree::new();

        tree.insert(3, "three".to_string()).unwrap();
        tree.insert(1, "one".to_string()).unwrap();
        tree.insert(2, "two".to_string()).unwrap();

        let keys = tree.keys();
        assert_eq!(keys, vec![1, 2, 3]); // Should be sorted

        let values = tree.values();
        assert_eq!(values, vec!["one".to_string(), "two".to_string(), "three".to_string()]);
    }

    #[test]
    fn test_bplus_tree_clear() {
        let mut tree = BPlusTree::new();

        tree.insert(1, "one".to_string()).unwrap();
        tree.insert(2, "two".to_string()).unwrap();

        assert_eq!(tree.len(), 2);
        tree.clear();
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_bplus_tree_with_custom_order() {
        let tree: BPlusTree<i32, String> = BPlusTree::with_order(5);
        assert_eq!(tree.order, 5);
    }

    #[test]
    #[should_panic]
    fn test_bplus_tree_invalid_order() {
        let _tree: BPlusTree<i32, String> = BPlusTree::with_order(2); // Should panic
    }

    #[test]
    fn test_bplus_tree_node_operations() {
        let mut node = BPlusTreeNode::new_leaf(5);

        assert!(node.is_leaf());
        assert!(!node.is_full());

        node.insert_into_leaf(1, "one".to_string()).unwrap();
        node.insert_into_leaf(2, "two".to_string()).unwrap();

        assert_eq!(node.find_key(&1), Some(0));
        assert_eq!(node.find_key(&2), Some(1));
        assert_eq!(node.find_key(&3), None);
    }

    #[test]
    fn test_bplus_tree_stats() {
        let mut tree = BPlusTree::new();

        for i in 1..=100 {
            tree.insert(i, format!("value_{}", i)).unwrap();
        }

        let stats = tree.stats();
        assert_eq!(stats.entry_count, 100);
        assert_eq!(stats.index_type, IndexType::BPlusTree);
        assert!(stats.type_specific.contains_key("depth"));
        assert!(stats.type_specific.contains_key("order"));
    }
}
