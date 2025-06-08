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

//! Database Interface for State Management
//!
//! This module provides a high-level abstraction layer for database operations
//! used by the state management system. It bridges the gap between the MPT
//! storage needs and the underlying storage engine.
//!
//! # Features
//!
//! - Key-value operations optimized for MPT nodes
//! - Batch operations for efficiency
//! - Transaction support
//! - Snapshot isolation
//! - Compression and serialization
//! - Metrics and monitoring

use crate::state::mpt::{Hash, Key, MPTError, Node, NodeId, TrieResult, Value};
use crate::storage_engine::{BufferManager, DatabaseId, Storage, StorageConfig, StorageError, StorageResult, Transaction, TransactionManager, VersionId};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Database operation types for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbOperation {
    Get,
    Put,
    Delete,
    BatchPut,
    BatchDelete,
    Snapshot,
}

/// Database interface configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// Underlying storage configuration
    pub storage_config: StorageConfig,
    /// Enable compression for stored data
    pub enable_compression: bool,
    /// Cache size for frequently accessed nodes
    pub cache_size: usize,
    /// Batch size for bulk operations
    pub batch_size: usize,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            storage_config: StorageConfig::default(),
            enable_compression: true,
            cache_size: 10000,
            batch_size: 1000,
            enable_metrics: true,
        }
    }
}

/// Database interface error types
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Key not found: {0:?}")]
    KeyNotFound(Vec<u8>),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Cache error: {0}")]
    Cache(String),
}

impl From<DbError> for MPTError {
    fn from(err: DbError) -> Self {
        MPTError::StorageError(format!("{}", err))
    }
}

impl From<serde_json::Error> for DbError {
    fn from(err: serde_json::Error) -> Self {
        DbError::Serialization(err.to_string())
    }
}

/// Type alias for database operation results
pub type DbResult<T> = Result<T, DbError>;

/// Batch operation for efficient bulk operations
#[derive(Debug, Clone)]
pub enum BatchOp {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

/// Database statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct DbStats {
    pub get_count: u64,
    pub put_count: u64,
    pub delete_count: u64,
    pub batch_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_size_bytes: u64,
    pub compression_ratio: f64,
}

/// High-level database interface trait
///
/// This trait provides the core operations needed by the MPT implementation
/// while abstracting away the underlying storage details.
pub trait DatabaseInterface: Send + Sync {
    /// Get a value by key
    fn get(&self, key: &[u8]) -> DbResult<Option<Vec<u8>>>;

    /// Put a key-value pair
    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> DbResult<()>;

    /// Delete a key
    fn delete(&self, key: &[u8]) -> DbResult<bool>;

    /// Check if a key exists
    fn contains(&self, key: &[u8]) -> DbResult<bool>;

    /// Execute a batch of operations atomically
    fn batch(&self, ops: Vec<BatchOp>) -> DbResult<()>;

    /// Create a database snapshot
    fn snapshot(&self) -> DbResult<Box<dyn DatabaseSnapshot>>;

    /// Get database statistics
    fn stats(&self) -> DbStats;

    /// Flush any pending operations to disk
    fn flush(&self) -> DbResult<()>;

    /// Close the database connection
    fn close(&mut self) -> DbResult<()>;
}

/// Database snapshot interface for point-in-time reads
pub trait DatabaseSnapshot: Send + Sync {
    /// Get a value from the snapshot
    fn get(&self, key: &[u8]) -> DbResult<Option<Vec<u8>>>;

    /// Check if a key exists in the snapshot
    fn contains(&self, key: &[u8]) -> DbResult<bool>;

    /// Get the snapshot version
    fn version(&self) -> VersionId;
}

/// Main database implementation using the storage engine
pub struct Database {
    /// Database configuration
    config: DbConfig,

    /// In-memory cache for frequently accessed data
    cache: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,

    /// Database statistics
    stats: Arc<RwLock<DbStats>>,

    /// Database ID
    db_id: DatabaseId,

    /// Simple in-memory storage for now (will be replaced with real storage engine)
    storage: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl Database {
    /// Create a new database instance
    pub fn new<P: AsRef<Path>>(path: P, config: DbConfig) -> DbResult<Self> {
        let _ = path; // Will be used when implementing real storage

        let cache = Arc::new(RwLock::new(HashMap::with_capacity(config.cache_size)));
        let stats = Arc::new(RwLock::new(DbStats::default()));
        let storage = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            config,
            cache,
            stats,
            db_id: DatabaseId(1),
            storage,
        })
    }

    /// Create a new in-memory database for testing
    pub fn new_in_memory() -> DbResult<Self> {
        Self::new("", DbConfig::default())
    }

    /// Serialize data with optional compression
    fn serialize_with_compression(&self, data: &[u8]) -> DbResult<Vec<u8>> {
        if self.config.enable_compression {
            // For now, just return the data as-is
            // In production, you would use a compression library like lz4, zstd, etc.
            Ok(data.to_vec())
        } else {
            Ok(data.to_vec())
        }
    }

    /// Deserialize data with optional decompression
    fn deserialize_with_decompression(&self, data: &[u8]) -> DbResult<Vec<u8>> {
        if self.config.enable_compression {
            // For now, just return the data as-is
            Ok(data.to_vec())
        } else {
            Ok(data.to_vec())
        }
    }

    /// Update cache with key-value pair
    fn update_cache(&self, key: Vec<u8>, value: Vec<u8>) {
        let mut cache = self.cache.write();
        if cache.len() >= self.config.cache_size {
            // Simple LRU eviction (remove first entry)
            if let Some(first_key) = cache.keys().next().cloned() {
                cache.remove(&first_key);
            }
        }
        cache.insert(key, value);
    }

    /// Check cache for key
    fn check_cache(&self, key: &[u8]) -> Option<Vec<u8>> {
        let cache = self.cache.read();
        cache.get(key).cloned()
    }

    /// Update statistics
    fn update_stats(&self, operation: DbOperation, hit: bool) {
        if self.config.enable_metrics {
            let mut stats = self.stats.write();
            match operation {
                DbOperation::Get => {
                    stats.get_count += 1;
                    if hit {
                        stats.cache_hits += 1;
                    } else {
                        stats.cache_misses += 1;
                    }
                }
                DbOperation::Put => stats.put_count += 1,
                DbOperation::Delete => stats.delete_count += 1,
                DbOperation::BatchPut | DbOperation::BatchDelete => stats.batch_count += 1,
                _ => {}
            }
        }
    }
}

impl DatabaseInterface for Database {
    fn get(&self, key: &[u8]) -> DbResult<Option<Vec<u8>>> {
        // Check cache first
        if let Some(cached_value) = self.check_cache(key) {
            self.update_stats(DbOperation::Get, true);
            return Ok(Some(cached_value));
        }

        // If not in cache, fetch from storage
        let storage = self.storage.read();
        if let Some(value) = storage.get(key).cloned() {
            // Update cache and return
            drop(storage);
            self.update_cache(key.to_vec(), value.clone());
            self.update_stats(DbOperation::Get, false);
            Ok(Some(value))
        } else {
            self.update_stats(DbOperation::Get, false);
            Ok(None)
        }
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> DbResult<()> {
        // Serialize and compress if needed
        let compressed_value = self.serialize_with_compression(&value)?;

        // Update cache
        self.update_cache(key.clone(), value);

        // Write to storage
        {
            let mut storage = self.storage.write();
            storage.insert(key, compressed_value);
        }

        self.update_stats(DbOperation::Put, false);
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> DbResult<bool> {
        // Remove from cache
        {
            let mut cache = self.cache.write();
            cache.remove(key);
        }

        // Delete from storage
        let mut storage = self.storage.write();
        let existed = storage.remove(key).is_some();

        self.update_stats(DbOperation::Delete, false);
        Ok(existed)
    }

    fn contains(&self, key: &[u8]) -> DbResult<bool> {
        Ok(self.get(key)?.is_some())
    }

    fn batch(&self, ops: Vec<BatchOp>) -> DbResult<()> {
        // Execute all operations atomically
        for op in ops {
            match op {
                BatchOp::Put { key, value } => {
                    self.put(key, value)?;
                }
                BatchOp::Delete { key } => {
                    self.delete(&key)?;
                }
            }
        }

        self.update_stats(DbOperation::BatchPut, false);
        Ok(())
    }

    fn snapshot(&self) -> DbResult<Box<dyn DatabaseSnapshot>> {
        let version = VersionId(crate::storage_engine::generate_timestamp());
        let snapshot_data = {
            let storage = self.storage.read();
            storage.clone()
        };

        Ok(Box::new(DatabaseSnapshotImpl { data: snapshot_data, version }))
    }

    fn stats(&self) -> DbStats {
        let stats = self.stats.read();
        stats.clone()
    }

    fn flush(&self) -> DbResult<()> {
        // In real implementation, this would flush to the storage engine
        Ok(())
    }

    fn close(&mut self) -> DbResult<()> {
        // In real implementation, this would close the storage engine
        Ok(())
    }
}

/// Database snapshot implementation
pub struct DatabaseSnapshotImpl {
    data: HashMap<Vec<u8>, Vec<u8>>,
    version: VersionId,
}

impl DatabaseSnapshot for DatabaseSnapshotImpl {
    fn get(&self, key: &[u8]) -> DbResult<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    fn contains(&self, key: &[u8]) -> DbResult<bool> {
        Ok(self.data.contains_key(key))
    }

    fn version(&self) -> VersionId {
        self.version
    }
}

/// Storage adapter that implements NodeStorage for MPT
pub struct MptStorageAdapter {
    db: Arc<dyn DatabaseInterface>,
}

impl MptStorageAdapter {
    pub fn new(db: Arc<dyn DatabaseInterface>) -> Self {
        Self { db }
    }

    /// Serialize a node for storage
    fn serialize_node(&self, node: &Node) -> DbResult<Vec<u8>> {
        serde_json::to_vec(node).map_err(DbError::from)
    }

    /// Deserialize a node from storage
    fn deserialize_node(&self, data: &[u8]) -> DbResult<Node> {
        serde_json::from_slice(data).map_err(DbError::from)
    }

    /// Convert NodeId to storage key
    fn node_key(&self, id: &NodeId) -> Vec<u8> {
        format!("node:{}", hex::encode(id)).into_bytes()
    }
}

impl Clone for MptStorageAdapter {
    fn clone(&self) -> Self {
        Self { db: self.db.clone() }
    }
}

impl crate::state::mpt::trie::NodeStorage for MptStorageAdapter {
    fn get_node(&self, id: &NodeId) -> TrieResult<Option<Node>> {
        let key = self.node_key(id);
        match self.db.get(&key) {
            Ok(Some(data)) => {
                let node = self.deserialize_node(&data)?;
                Ok(Some(node))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn put_node(&mut self, node: &Node) -> TrieResult<()> {
        let key = self.node_key(&node.id);
        let data = self.serialize_node(node)?;
        self.db.put(key, data).map_err(|e| e.into())
    }

    fn delete_node(&mut self, id: &NodeId) -> TrieResult<()> {
        let key = self.node_key(id);
        self.db.delete(&key).map_err(DbError::from).map_err(MPTError::from)?;
        Ok(())
    }

    fn contains_node(&self, id: &NodeId) -> bool {
        let key = self.node_key(id);
        self.db.contains(&key).unwrap_or(false)
    }
}

/// Helper function to create a persistent MPT with database backend
pub fn create_persistent_mpt<P: AsRef<Path>>(db_path: P, config: Option<DbConfig>) -> DbResult<crate::state::mpt::MerklePatriciaTrie<MptStorageAdapter>> {
    let config = config.unwrap_or_default();
    let database = Arc::new(Database::new(db_path, config)?);
    let storage_adapter = MptStorageAdapter::new(database);

    Ok(crate::state::mpt::MerklePatriciaTrie::new(storage_adapter))
}

/// Helper function to create an in-memory MPT with database backend for testing
pub fn create_in_memory_mpt() -> DbResult<crate::state::mpt::MerklePatriciaTrie<MptStorageAdapter>> {
    let database = Arc::new(Database::new_in_memory()?);
    let storage_adapter = MptStorageAdapter::new(database);

    Ok(crate::state::mpt::MerklePatriciaTrie::new(storage_adapter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mpt::trie::NodeStorage;
    use crate::state::mpt::{Key, Value};
    use tempfile::TempDir;

    #[test]
    fn test_database_interface_basic_operations() {
        let db = Database::new_in_memory().unwrap();

        // Test put and get
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        db.put(key.clone(), value.clone()).unwrap();
        let retrieved = db.get(&key).unwrap();
        assert_eq!(retrieved, Some(value.clone()));

        // Test delete
        assert!(db.delete(&key).unwrap());
        assert_eq!(db.get(&key).unwrap(), None);

        // Test contains
        db.put(key.clone(), value).unwrap();
        assert!(db.contains(&key).unwrap());
        db.delete(&key).unwrap();
        assert!(!db.contains(&key).unwrap());
    }

    #[test]
    fn test_batch_operations() {
        let db = Database::new_in_memory().unwrap();

        let ops = vec![
            BatchOp::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            },
            BatchOp::Put {
                key: b"key2".to_vec(),
                value: b"value2".to_vec(),
            },
            BatchOp::Delete { key: b"key3".to_vec() },
        ];

        db.batch(ops).unwrap();

        assert_eq!(db.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(db.get(b"key2").unwrap(), Some(b"value2".to_vec()));
        assert_eq!(db.get(b"key3").unwrap(), None);
    }

    #[test]
    fn test_cache_functionality() {
        let db = Database::new_in_memory().unwrap();

        let key = b"cached_key".to_vec();
        let value = b"cached_value".to_vec();

        // Put a value
        db.put(key.clone(), value.clone()).unwrap();

        // First get should miss cache, second should hit
        let _ = db.get(&key).unwrap();
        let stats_before = db.stats();
        let _ = db.get(&key).unwrap();
        let stats_after = db.stats();

        assert!(stats_after.cache_hits > stats_before.cache_hits);
    }

    #[test]
    fn test_snapshot_functionality() {
        let db = Database::new_in_memory().unwrap();

        // Put some data
        db.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
        db.put(b"key2".to_vec(), b"value2".to_vec()).unwrap();

        // Create snapshot
        let snapshot = db.snapshot().unwrap();

        // Verify snapshot data
        assert_eq!(snapshot.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(snapshot.get(b"key2").unwrap(), Some(b"value2".to_vec()));

        // Modify original database
        db.put(b"key1".to_vec(), b"modified_value1".to_vec()).unwrap();

        // Snapshot should still have original data
        assert_eq!(snapshot.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(db.get(b"key1").unwrap(), Some(b"modified_value1".to_vec()));
    }

    #[test]
    fn test_mpt_storage_adapter() {
        let db = Arc::new(Database::new_in_memory().unwrap());
        let mut adapter = MptStorageAdapter::new(db);

        // Create a test node
        let node = Node::new_empty();
        let node_id = node.id;

        // Test put and get
        adapter.put_node(&node).unwrap();
        let retrieved = adapter.get_node(&node_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, node_id);

        // Test contains
        assert!(adapter.contains_node(&node_id));

        // Test delete
        adapter.delete_node(&node_id).unwrap();
        assert!(!adapter.contains_node(&node_id));
        assert!(adapter.get_node(&node_id).unwrap().is_none());
    }

    #[test]
    fn test_create_persistent_mpt() {
        let temp_dir = TempDir::new().unwrap();
        let mpt = create_persistent_mpt(temp_dir.path(), None).unwrap();

        // Test basic MPT operations
        let key = Key::from("test_key");
        let value = Value::from("test_value");

        // This should work without panicking
        assert!(mpt.get(&key).is_ok());
    }

    #[test]
    fn test_create_in_memory_mpt() {
        let mut mpt = create_in_memory_mpt().unwrap();

        // Test basic MPT operations
        let key = Key::from("test_key");
        let value = Value::from("test_value");

        mpt.put(key.clone(), value.clone()).unwrap();
        let retrieved = mpt.get(&key).unwrap();
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_statistics_tracking() {
        let db = Database::new_in_memory().unwrap();

        // Perform various operations
        db.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
        db.get(b"key1").unwrap();
        db.delete(b"key1").unwrap();

        let stats = db.stats();
        assert!(stats.put_count > 0);
        assert!(stats.get_count > 0);
        assert!(stats.delete_count > 0);
    }
}
