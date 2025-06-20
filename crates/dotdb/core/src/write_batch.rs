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

use crate::indices::{BPlusTree, CompositeIndex, CompositeKey, HashIndex, Index, IndexError, IndexKey, IndexMaintenance, IndexPersistenceManager, IndexResult, IndexStats, IndexType, IndexValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

/// Write operation types
#[derive(Debug, Clone, PartialEq)]
pub enum WriteOperation<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Insert a new key-value pair
    Insert { key: K, value: V },
    /// Update an existing key with a new value
    Update { key: K, old_value: V, new_value: V },
    /// Delete a key
    Delete { key: K, value: V },
}

/// Batch of write operations with automatic index maintenance
pub struct WriteBatch<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    /// List of operations in this batch
    operations: Vec<WriteOperation<K, V>>,
    /// Maximum number of operations allowed in a batch
    max_operations: usize,
    /// Whether to automatically maintain indices
    auto_maintain_indices: bool,
}

impl<K, V> WriteBatch<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    /// Create a new write batch
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            max_operations: 1000, // Default limit
            auto_maintain_indices: true,
        }
    }

    /// Create a new write batch with custom configuration
    pub fn with_config(max_operations: usize, auto_maintain_indices: bool) -> Self {
        Self {
            operations: Vec::new(),
            max_operations,
            auto_maintain_indices,
        }
    }

    /// Add an insert operation to the batch
    pub fn insert(&mut self, key: K, value: V) -> IndexResult<()> {
        if self.operations.len() >= self.max_operations {
            return Err(IndexError::InvalidOperation("Batch is full".to_string()));
        }

        self.operations.push(WriteOperation::Insert { key, value });
        Ok(())
    }

    /// Add an update operation to the batch
    pub fn update(&mut self, key: K, old_value: V, new_value: V) -> IndexResult<()> {
        if self.operations.len() >= self.max_operations {
            return Err(IndexError::InvalidOperation("Batch is full".to_string()));
        }

        self.operations.push(WriteOperation::Update { key, old_value, new_value });
        Ok(())
    }

    /// Add a delete operation to the batch
    pub fn delete(&mut self, key: K, value: V) -> IndexResult<()> {
        if self.operations.len() >= self.max_operations {
            return Err(IndexError::InvalidOperation("Batch is full".to_string()));
        }

        self.operations.push(WriteOperation::Delete { key, value });
        Ok(())
    }

    /// Get the number of operations in the batch
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Check if the batch is full
    pub fn is_full(&self) -> bool {
        self.operations.len() >= self.max_operations
    }

    /// Clear all operations from the batch
    pub fn clear(&mut self) {
        self.operations.clear();
    }

    /// Get read-only access to operations
    pub fn operations(&self) -> &[WriteOperation<K, V>] {
        &self.operations
    }

    /// Apply all operations in the batch to the given indices
    pub fn apply_to_indices(&self, indices: &mut IndexManager<K, V>) -> IndexResult<BatchResult> {
        let mut result = BatchResult::new();

        for (op_index, operation) in self.operations.iter().enumerate() {
            let op_result = match operation {
                WriteOperation::Insert { key, value } => indices.insert_to_all(key.clone(), value.clone()),
                WriteOperation::Update { key, new_value, .. } => indices.update_in_all(key.clone(), new_value.clone()),
                WriteOperation::Delete { key, .. } => indices.delete_from_all(key),
            };

            match op_result {
                Ok(_) => result.successful_operations += 1,
                Err(e) => {
                    result.failed_operations.push(BatchOperationError { operation_index: op_index, error: e });
                }
            }
        }

        Ok(result)
    }

    /// Validate all operations in the batch
    pub fn validate(&self) -> IndexResult<()> {
        // Check for duplicate keys in insert operations
        let mut insert_keys = std::collections::HashSet::new();

        for operation in &self.operations {
            match operation {
                WriteOperation::Insert { key, .. } => {
                    if !insert_keys.insert(key.clone()) {
                        return Err(IndexError::InvalidOperation(format!("Duplicate insert key in batch: {:?}", key)));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Get estimated size of the batch in bytes
    pub fn estimated_size(&self) -> usize {
        self.operations
            .iter()
            .map(|op| match op {
                WriteOperation::Insert { key, value } => key.size() + value.to_bytes().len(),
                WriteOperation::Update { key, new_value, .. } => key.size() + new_value.to_bytes().len(),
                WriteOperation::Delete { key, .. } => key.size(),
            })
            .sum()
    }
}

impl<K, V> Default for WriteBatch<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Result of applying a write batch
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Number of operations that succeeded
    pub successful_operations: usize,
    /// List of failed operations with their errors
    pub failed_operations: Vec<BatchOperationError>,
}

impl BatchResult {
    /// Create a new batch result
    pub fn new() -> Self {
        Self {
            successful_operations: 0,
            failed_operations: Vec::new(),
        }
    }

    /// Check if all operations succeeded
    pub fn is_success(&self) -> bool {
        self.failed_operations.is_empty()
    }

    /// Get the total number of operations processed
    pub fn total_operations(&self) -> usize {
        self.successful_operations + self.failed_operations.len()
    }

    /// Get the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_operations() == 0 {
            100.0
        } else {
            (self.successful_operations as f64 / self.total_operations() as f64) * 100.0
        }
    }
}

impl Default for BatchResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Error information for a failed batch operation
#[derive(Debug, Clone)]
pub struct BatchOperationError {
    /// Index of the operation in the batch that failed
    pub operation_index: usize,
    /// The error that occurred
    pub error: IndexError,
}

/// Index manager that maintains multiple indices
pub struct IndexManager<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    /// B+ tree indices by name
    btree_indices: HashMap<String, Arc<Mutex<BPlusTree<K, V>>>>,
    /// Hash indices by name
    hash_indices: HashMap<String, Arc<Mutex<HashIndex<K, V>>>>,
    /// Composite indices by name
    composite_indices: HashMap<String, Arc<Mutex<CompositeIndex<V>>>>,
    /// Whether to verify operations
    verify_operations: bool,
    /// Persistence manager for disk operations
    persistence_manager: Option<Arc<Mutex<IndexPersistenceManager>>>,
}

impl<K, V> IndexManager<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    /// Create a new index manager
    pub fn new() -> Self {
        Self {
            btree_indices: HashMap::new(),
            hash_indices: HashMap::new(),
            composite_indices: HashMap::new(),
            verify_operations: false,
            persistence_manager: None,
        }
    }

    /// Create a new index manager with verification enabled
    pub fn with_verification() -> Self {
        Self {
            btree_indices: HashMap::new(),
            hash_indices: HashMap::new(),
            composite_indices: HashMap::new(),
            verify_operations: true,
            persistence_manager: None,
        }
    }

    /// Add a B+ tree index
    pub fn add_btree_index(&mut self, name: String, index: BPlusTree<K, V>) -> IndexResult<()> {
        if self.btree_indices.contains_key(&name) || self.hash_indices.contains_key(&name) || self.composite_indices.contains_key(&name) {
            return Err(IndexError::IndexExists(name));
        }

        self.btree_indices.insert(name, Arc::new(Mutex::new(index)));
        Ok(())
    }

    /// Add a hash index
    pub fn add_hash_index(&mut self, name: String, index: HashIndex<K, V>) -> IndexResult<()> {
        if self.btree_indices.contains_key(&name) || self.hash_indices.contains_key(&name) || self.composite_indices.contains_key(&name) {
            return Err(IndexError::IndexExists(name));
        }

        self.hash_indices.insert(name, Arc::new(Mutex::new(index)));
        Ok(())
    }

    /// Add a composite index
    pub fn add_composite_index(&mut self, name: String, index: CompositeIndex<V>) -> IndexResult<()> {
        if self.btree_indices.contains_key(&name) || self.hash_indices.contains_key(&name) || self.composite_indices.contains_key(&name) {
            return Err(IndexError::IndexExists(name));
        }

        self.composite_indices.insert(name, Arc::new(Mutex::new(index)));
        Ok(())
    }

    /// Remove an index by name
    pub fn remove_index(&mut self, name: &str) -> IndexResult<()> {
        if self.btree_indices.remove(name).is_some() || self.hash_indices.remove(name).is_some() || self.composite_indices.remove(name).is_some() {
            Ok(())
        } else {
            Err(IndexError::KeyNotFound(format!("Index: {}", name)))
        }
    }

    /// Get list of all index names
    pub fn index_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        names.extend(self.btree_indices.keys().cloned());
        names.extend(self.hash_indices.keys().cloned());
        names.extend(self.composite_indices.keys().cloned());
        names.sort();
        names
    }

    /// Get the type of an index by name
    pub fn index_type(&self, name: &str) -> Option<IndexType> {
        if self.btree_indices.contains_key(name) {
            Some(IndexType::BPlusTree)
        } else if self.hash_indices.contains_key(name) {
            Some(IndexType::Hash)
        } else if let Some(composite_index) = self.composite_indices.get(name) {
            if let Ok(index) = composite_index.lock() { Some(index.index_type()) } else { None }
        } else {
            None
        }
    }

    /// Insert a key-value pair into all indices
    pub fn insert_to_all(&mut self, key: K, value: V) -> IndexResult<()> {
        // Insert into B+ tree indices
        for (name, index_arc) in &self.btree_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock B+ tree index: {}", name)))?;

            index.insert(key.clone(), value.clone())?;
        }

        // Insert into hash indices
        for (name, index_arc) in &self.hash_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock hash index: {}", name)))?;

            index.insert(key.clone(), value.clone())?;
        }

        Ok(())
    }

    /// Update a key in all indices
    pub fn update_in_all(&mut self, key: K, value: V) -> IndexResult<()> {
        // Update in B+ tree indices
        for (name, index_arc) in &self.btree_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock B+ tree index: {}", name)))?;

            index.update(key.clone(), value.clone())?;
        }

        // Update in hash indices
        for (name, index_arc) in &self.hash_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock hash index: {}", name)))?;

            index.update(key.clone(), value.clone())?;
        }

        Ok(())
    }

    /// Delete a key from all indices
    pub fn delete_from_all(&mut self, key: &K) -> IndexResult<()> {
        // Delete from B+ tree indices
        for (name, index_arc) in &self.btree_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock B+ tree index: {}", name)))?;

            index.delete(key)?;
        }

        // Delete from hash indices
        for (name, index_arc) in &self.hash_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock hash index: {}", name)))?;

            index.delete(key)?;
        }

        Ok(())
    }

    /// Get a value from a specific index
    pub fn get_from_index(&self, index_name: &str, key: &K) -> IndexResult<Option<V>> {
        if let Some(index_arc) = self.btree_indices.get(index_name) {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock B+ tree index: {}", index_name)))?;

            return index.get(key);
        }

        if let Some(index_arc) = self.hash_indices.get(index_name) {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock hash index: {}", index_name)))?;

            return index.get(key);
        }

        // Note: Composite indices require CompositeKey, not K
        // This method is for simple key lookups only
        Err(IndexError::KeyNotFound(format!("Index: {}", index_name)))
    }

    /// Get a value from a composite index using CompositeKey
    pub fn get_from_composite_index(&self, index_name: &str, key: &CompositeKey) -> IndexResult<Option<V>> {
        if let Some(index_arc) = self.composite_indices.get(index_name) {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock composite index: {}", index_name)))?;

            return index.get(key);
        }

        Err(IndexError::KeyNotFound(format!("Composite index: {}", index_name)))
    }

    /// Clear all indices
    pub fn clear_all(&mut self) -> IndexResult<()> {
        // Clear B+ tree indices
        for (name, index_arc) in &self.btree_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock B+ tree index: {}", name)))?;

            index.clear();
        }

        // Clear hash indices
        for (name, index_arc) in &self.hash_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock hash index: {}", name)))?;

            index.clear();
        }

        // Clear composite indices
        for (name, index_arc) in &self.composite_indices {
            let mut index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock composite index: {}", name)))?;

            index.clear();
        }

        Ok(())
    }

    /// Get the total number of entries across all indices
    pub fn total_entries(&self) -> IndexResult<usize> {
        let mut total = 0;

        // Count B+ tree entries
        for (name, index_arc) in &self.btree_indices {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock B+ tree index: {}", name)))?;

            total += index.len();
        }

        // Count hash entries
        for (name, index_arc) in &self.hash_indices {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock hash index: {}", name)))?;

            total += index.len();
        }

        // Count composite entries
        for (name, index_arc) in &self.composite_indices {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock composite index: {}", name)))?;

            total += index.len();
        }

        Ok(total)
    }

    /// Get statistics for all indices
    pub fn all_stats(&self) -> IndexResult<HashMap<String, crate::indices::IndexStats>> {
        let mut stats = HashMap::new();

        // Get B+ tree stats
        for (name, index_arc) in &self.btree_indices {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock B+ tree index: {}", name)))?;

            stats.insert(name.clone(), index.stats());
        }

        // Get hash stats
        for (name, index_arc) in &self.hash_indices {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock hash index: {}", name)))?;

            stats.insert(name.clone(), index.stats());
        }

        // Get composite stats
        for (name, index_arc) in &self.composite_indices {
            let index = index_arc.lock().map_err(|_| IndexError::Corruption(format!("Failed to lock composite index: {}", name)))?;

            stats.insert(name.clone(), index.stats());
        }

        Ok(stats)
    }

    /// Set persistence manager for automatic disk operations
    pub fn set_persistence_manager(&mut self, manager: Arc<Mutex<IndexPersistenceManager>>) {
        self.persistence_manager = Some(manager);
    }

    /// Auto-save all indices to disk
    pub fn auto_save(&self) -> IndexResult<()> {
        if let Some(persistence_manager) = &self.persistence_manager {
            let mut manager = persistence_manager
                .lock()
                .map_err(|_| IndexError::Corruption("Failed to acquire persistence manager lock".to_string()))?;

            // Save B+ tree indices
            for (name, index) in &self.btree_indices {
                let index_guard = index.lock().map_err(|_| IndexError::Corruption("Failed to acquire lock".to_string()))?;
                manager.save_index(name, &*index_guard, false)?;
            }

            // Save hash indices
            for (name, index) in &self.hash_indices {
                let index_guard = index.lock().map_err(|_| IndexError::Corruption("Failed to acquire lock".to_string()))?;
                manager.save_index(name, &*index_guard, false)?;
            }

            // Save composite indices
            for (name, index) in &self.composite_indices {
                let index_guard = index.lock().map_err(|_| IndexError::Corruption("Failed to acquire lock".to_string()))?;
                manager.save_index(name, &*index_guard, false)?;
            }
        }

        Ok(())
    }

    /// Load all indices from disk
    pub fn load_indices(&mut self) -> IndexResult<()> {
        if let Some(persistence_manager) = &self.persistence_manager {
            let mut manager = persistence_manager
                .lock()
                .map_err(|_| IndexError::Corruption("Failed to acquire persistence manager lock".to_string()))?;

            // Load B+ tree indices
            for (name, index) in &self.btree_indices {
                let mut index_guard = index.lock().map_err(|_| IndexError::Corruption("Failed to acquire lock".to_string()))?;

                manager.load_index(&name, &mut *index_guard)?;
            }

            // Load hash indices
            for (name, index) in &self.hash_indices {
                let mut index_guard = index.lock().map_err(|_| IndexError::Corruption("Failed to acquire lock".to_string()))?;

                manager.load_index(&name, &mut *index_guard)?;
            }
        }

        Ok(())
    }

    /// Get persistence statistics
    pub fn persistence_stats(&self) -> IndexResult<HashMap<String, String>> {
        let mut stats = HashMap::new();

        if let Some(persistence_manager) = &self.persistence_manager {
            let manager = persistence_manager
                .lock()
                .map_err(|_| IndexError::Corruption("Failed to acquire persistence manager lock".to_string()))?;

            stats.insert("total_disk_usage".to_string(), manager.total_disk_usage().to_string());
            stats.insert("registered_indices".to_string(), manager.list_indices().len().to_string());

            // Verify integrity of all indices
            let integrity_results = manager.verify_all()?;
            let valid_indices = integrity_results.values().filter(|&&valid| valid).count();
            stats.insert("valid_indices".to_string(), valid_indices.to_string());
            stats.insert("total_indices".to_string(), integrity_results.len().to_string());
        } else {
            stats.insert("persistence_enabled".to_string(), "false".to_string());
        }

        Ok(stats)
    }
}

impl<K, V> Default for IndexManager<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_batch_creation() {
        let batch: WriteBatch<i32, String> = WriteBatch::new();
        assert_eq!(batch.len(), 0);
        assert!(batch.is_empty());
        assert!(!batch.is_full());
    }

    #[test]
    fn test_write_batch_operations() {
        let mut batch = WriteBatch::new();

        batch.insert(1, "one".to_string()).unwrap();
        batch.update(2, "old".to_string(), "new".to_string()).unwrap();
        batch.delete(3, "three".to_string()).unwrap();

        assert_eq!(batch.len(), 3);
        assert!(!batch.is_empty());

        let operations = batch.operations();
        assert_eq!(operations.len(), 3);

        match &operations[0] {
            WriteOperation::Insert { key, value } => {
                assert_eq!(*key, 1);
                assert_eq!(*value, "one".to_string());
            }
            _ => panic!("Expected Insert operation"),
        }
    }

    #[test]
    fn test_write_batch_full() {
        let mut batch = WriteBatch::with_config(2, true);

        batch.insert(1, "one".to_string()).unwrap();
        batch.insert(2, "two".to_string()).unwrap();

        assert!(batch.is_full());

        let result = batch.insert(3, "three".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_write_batch_validation() {
        let mut batch = WriteBatch::new();

        batch.insert(1, "one".to_string()).unwrap();
        batch.insert(1, "duplicate".to_string()).unwrap(); // Duplicate key

        let result = batch.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_index_manager_creation() {
        let manager: IndexManager<i32, String> = IndexManager::new();
        assert_eq!(manager.index_names().len(), 0);
    }

    #[test]
    fn test_index_manager_add_indices() {
        let mut manager: IndexManager<i32, String> = IndexManager::new();

        let btree = BPlusTree::new();
        let hash = HashIndex::new();

        manager.add_btree_index("btree1".to_string(), btree).unwrap();
        manager.add_hash_index("hash1".to_string(), hash).unwrap();

        let names = manager.index_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"btree1".to_string()));
        assert!(names.contains(&"hash1".to_string()));

        assert_eq!(manager.index_type("btree1"), Some(IndexType::BPlusTree));
        assert_eq!(manager.index_type("hash1"), Some(IndexType::Hash));
    }

    #[test]
    fn test_index_manager_duplicate_name() {
        let mut manager: IndexManager<i32, String> = IndexManager::new();

        let btree1 = BPlusTree::new();
        let btree2 = BPlusTree::new();

        manager.add_btree_index("index1".to_string(), btree1).unwrap();
        let result = manager.add_btree_index("index1".to_string(), btree2);

        assert!(result.is_err());
    }

    #[test]
    fn test_index_manager_operations() {
        let mut manager: IndexManager<i32, String> = IndexManager::new();

        let btree = BPlusTree::new();
        let hash = HashIndex::new();

        manager.add_btree_index("btree1".to_string(), btree).unwrap();
        manager.add_hash_index("hash1".to_string(), hash).unwrap();

        // Insert into all indices
        manager.insert_to_all(1, "one".to_string()).unwrap();

        // Get from specific indices
        let btree_result = manager.get_from_index("btree1", &1).unwrap();
        let hash_result = manager.get_from_index("hash1", &1).unwrap();

        assert_eq!(btree_result, Some("one".to_string()));
        assert_eq!(hash_result, Some("one".to_string()));

        // Update in all indices
        manager.update_in_all(1, "ONE".to_string()).unwrap();

        let btree_result = manager.get_from_index("btree1", &1).unwrap();
        assert_eq!(btree_result, Some("ONE".to_string()));

        // Delete from all indices
        manager.delete_from_all(&1).unwrap();

        let btree_result = manager.get_from_index("btree1", &1).unwrap();
        assert_eq!(btree_result, None);
    }

    #[test]
    fn test_batch_result() {
        let mut result = BatchResult::new();
        result.successful_operations = 8;
        result.failed_operations.push(BatchOperationError {
            operation_index: 5,
            error: IndexError::KeyNotFound("test".to_string()),
        });
        result.failed_operations.push(BatchOperationError {
            operation_index: 7,
            error: IndexError::InvalidKey("invalid".to_string()),
        });

        assert!(!result.is_success());
        assert_eq!(result.total_operations(), 10);
        assert_eq!(result.success_rate(), 80.0);
    }

    #[test]
    fn test_write_batch_apply_to_indices() {
        let mut manager: IndexManager<i32, String> = IndexManager::new();
        let btree = BPlusTree::new();
        manager.add_btree_index("test_btree".to_string(), btree).unwrap();

        let mut batch = WriteBatch::new();
        batch.insert(1, "one".to_string()).unwrap();
        batch.insert(2, "two".to_string()).unwrap();

        let result = batch.apply_to_indices(&mut manager).unwrap();

        assert!(result.is_success());
        assert_eq!(result.successful_operations, 2);
        assert_eq!(result.total_operations(), 2);

        // Verify data was inserted
        let value = manager.get_from_index("test_btree", &1).unwrap();
        assert_eq!(value, Some("one".to_string()));
    }

    #[test]
    fn test_write_batch_estimated_size() {
        let mut batch = WriteBatch::new();

        batch.insert(1, "one".to_string()).unwrap();
        batch.insert(2, "two".to_string()).unwrap();

        let size = batch.estimated_size();
        assert!(size > 0);

        // Size should include key and value sizes
        // i32 is 4 bytes, strings are their byte length
        let expected_size = 4 + 3 + 4 + 3; // i32 size (4 bytes) + string lengths
        assert_eq!(size, expected_size);
    }

    #[test]
    fn test_index_manager_add_composite_index() {
        use crate::indices::{CompositeIndexConfig, FieldSpec};

        let mut manager: IndexManager<i32, String> = IndexManager::new();

        // Create composite index configuration
        let config = CompositeIndexConfig::new(
            vec![FieldSpec::new("field1".to_string(), 0, true), FieldSpec::new("field2".to_string(), 1, true)],
            true, // Use B+ tree
        );

        let composite_index = CompositeIndex::new(config).unwrap();
        manager.add_composite_index("composite1".to_string(), composite_index).unwrap();

        let names = manager.index_names();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"composite1".to_string()));

        // Check index type
        if let Some(IndexType::Composite(fields)) = manager.index_type("composite1") {
            assert_eq!(fields.len(), 2);
            assert!(fields.contains(&"field1".to_string()));
            assert!(fields.contains(&"field2".to_string()));
        } else {
            panic!("Expected composite index type");
        }
    }

    #[test]
    fn test_index_manager_composite_index_operations() {
        use crate::indices::{CompositeIndexConfig, CompositeKey, FieldSpec};

        let mut manager: IndexManager<i32, String> = IndexManager::new();

        // Create and add composite index
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("field1".to_string(), 0, true)], true);

        let composite_index = CompositeIndex::new(config).unwrap();
        manager.add_composite_index("composite1".to_string(), composite_index).unwrap();

        // Test composite key operations
        let composite_key = CompositeKey::new(vec![b"test_value".to_vec()]);

        // Note: Regular insert_to_all doesn't work with composite indices
        // because they use CompositeKey, not K
        let result = manager.get_from_composite_index("composite1", &composite_key).unwrap();
        assert_eq!(result, None);

        // Test that composite index is included in stats
        let stats = manager.all_stats().unwrap();
        assert!(stats.contains_key("composite1"));
    }

    #[test]
    fn test_index_manager_mixed_indices() {
        use crate::indices::{CompositeIndexConfig, FieldSpec};

        let mut manager: IndexManager<i32, String> = IndexManager::new();

        // Add different types of indices
        let btree = BPlusTree::new();
        let hash = HashIndex::new();
        let config = CompositeIndexConfig::new(vec![FieldSpec::new("field1".to_string(), 0, false)], false);
        let composite = CompositeIndex::new(config).unwrap();

        manager.add_btree_index("btree1".to_string(), btree).unwrap();
        manager.add_hash_index("hash1".to_string(), hash).unwrap();
        manager.add_composite_index("composite1".to_string(), composite).unwrap();

        let names = manager.index_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"btree1".to_string()));
        assert!(names.contains(&"hash1".to_string()));
        assert!(names.contains(&"composite1".to_string()));

        // Test index types
        assert_eq!(manager.index_type("btree1"), Some(IndexType::BPlusTree));
        assert_eq!(manager.index_type("hash1"), Some(IndexType::Hash));

        if let Some(IndexType::Composite(fields)) = manager.index_type("composite1") {
            assert_eq!(fields.len(), 1);
        } else {
            panic!("Expected composite index type");
        }

        // Test removal
        manager.remove_index("btree1").unwrap();
        manager.remove_index("hash1").unwrap();
        manager.remove_index("composite1").unwrap();

        assert_eq!(manager.index_names().len(), 0);
    }
}
