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

//! # State Access Module
//!
//! This module implements the logic for accessing contract state during execution.
//! It bridges the gap between the contract storage layout and the underlying
//! Merkle Patricia Trie, providing efficient and secure state access patterns.
//!
//! ## Key Features
//!
//! - Contract-specific state isolation
//! - Efficient batch operations
//! - Gas accounting integration
//! - Cache layer for frequently accessed data
//! - Transaction-level state change tracking

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Arc, RwLock};

use dotdb_core::state::mpt::trie::NodeStorage;
use dotdb_core::state::mpt::{Key, Value};
use dotdb_core::state::{ContractAddress, ContractStorageLayout, MerklePatriciaTrie};
use serde::{Deserialize, Serialize};

/// Result type for state access operations
pub type StateAccessResult<T> = Result<T, StateAccessError>;

/// Interface for state access operations
pub trait StateAccessInterface {
    /// Load a value from contract storage
    fn load_storage(&self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<Option<Vec<u8>>>;

    /// Store a value to contract storage
    fn store_storage(&mut self, contract: ContractAddress, key: &[u8], value: &[u8]) -> StateAccessResult<()>;

    /// Check if a storage key exists
    fn storage_exists(&self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<bool>;

    /// Get the size of stored data
    fn storage_size(&self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<usize>;

    /// Clear a storage slot
    fn clear_storage(&mut self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<()>;

    /// Load multiple values efficiently
    fn multi_load(&self, contract: ContractAddress, keys: &[Vec<u8>]) -> StateAccessResult<Vec<Option<Vec<u8>>>>;

    /// Store multiple values efficiently
    fn multi_store(&mut self, contract: ContractAddress, entries: &[(Vec<u8>, Vec<u8>)]) -> StateAccessResult<()>;

    /// Get storage keys with pagination
    fn get_storage_keys(&self, contract: ContractAddress, start_key: Option<&[u8]>, limit: usize) -> StateAccessResult<Vec<Vec<u8>>>;
}

/// State access manager that implements the storage interface
pub struct StateAccessManager<S: NodeStorage> {
    /// Underlying Merkle Patricia Trie
    trie: Arc<RwLock<MerklePatriciaTrie<S>>>,
    /// Storage layouts for contracts
    layouts: Arc<RwLock<HashMap<ContractAddress, ContractStorageLayout>>>,
    /// Cache for frequently accessed storage
    cache: Arc<RwLock<StateCache>>,
    /// Configuration options
    config: StateAccessConfig,
}

impl<S: NodeStorage> fmt::Debug for StateAccessManager<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateAccessManager")
            .field("layouts", &self.layouts)
            .field("cache", &self.cache)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

/// Configuration for state access manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateAccessConfig {
    /// Maximum cache size in entries
    pub max_cache_size: usize,
    /// Enable cache for storage reads
    pub enable_cache: bool,
    /// Enable batch operations optimization
    pub enable_batch_optimization: bool,
    /// Maximum batch size for operations
    pub max_batch_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for StateAccessConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 10000,
            enable_cache: true,
            enable_batch_optimization: true,
            max_batch_size: 100,
            cache_ttl_seconds: 300, // 5 minutes
        }
    }
}

/// Cache for storage access
#[derive(Debug)]
struct StateCache {
    /// Cached storage entries
    entries: BTreeMap<CacheKey, CacheEntry>,
    /// Cache size tracking
    size: usize,
    /// Maximum cache size
    max_size: usize,
}

/// Cache key combining contract address and storage key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CacheKey {
    contract: ContractAddress,
    storage_key: Vec<u8>,
}

/// Cache entry with value and metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    value: Option<Vec<u8>>,
    timestamp: u64,
    access_count: u64,
}

/// Errors that can occur during state access
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateAccessError {
    /// Storage backend error
    StorageError(String),
    /// Invalid storage key format
    InvalidKey(String),
    /// Invalid storage value format
    InvalidValue(String),
    /// Contract not found
    ContractNotFound(ContractAddress),
    /// Layout not found for contract
    LayoutNotFound(ContractAddress),
    /// Cache operation failed
    CacheError(String),
    /// Batch operation too large
    BatchTooLarge { size: usize, max: usize },
    /// Key encoding error
    KeyEncodingError(String),
    /// Value encoding error
    ValueEncodingError(String),
    /// Access denied
    AccessDenied,
    /// Concurrency error (lock acquisition failed)
    ConcurrencyError,
    /// Operation not supported
    OperationNotSupported(String),
}

impl fmt::Display for StateAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateAccessError::StorageError(msg) => write!(f, "Storage error: {msg}"),
            StateAccessError::InvalidKey(msg) => write!(f, "Invalid key: {msg}"),
            StateAccessError::InvalidValue(msg) => write!(f, "Invalid value: {msg}"),
            StateAccessError::ContractNotFound(addr) => write!(f, "Contract not found: {addr:?}"),
            StateAccessError::LayoutNotFound(addr) => write!(f, "Layout not found for contract: {addr:?}"),
            StateAccessError::CacheError(msg) => write!(f, "Cache error: {msg}"),
            StateAccessError::BatchTooLarge { size, max } => {
                write!(f, "Batch too large: {size} entries, max {max}")
            }
            StateAccessError::KeyEncodingError(msg) => write!(f, "Key encoding error: {msg}"),
            StateAccessError::ValueEncodingError(msg) => write!(f, "Value encoding error: {msg}"),
            StateAccessError::AccessDenied => write!(f, "Access denied"),
            StateAccessError::ConcurrencyError => write!(f, "Concurrency error: failed to acquire lock"),
            StateAccessError::OperationNotSupported(op) => write!(f, "Operation not supported: {op}"),
        }
    }
}

impl std::error::Error for StateAccessError {}

impl<S: NodeStorage> StateAccessManager<S> {
    /// Create a new state access manager
    pub fn new(trie: MerklePatriciaTrie<S>) -> Self {
        Self::with_config(trie, StateAccessConfig::default())
    }

    /// Create a new state access manager with custom configuration
    pub fn with_config(trie: MerklePatriciaTrie<S>, config: StateAccessConfig) -> Self {
        let cache = StateCache::new(config.max_cache_size);

        Self {
            trie: Arc::new(RwLock::new(trie)),
            layouts: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(cache)),
            config,
        }
    }

    /// Register a storage layout for a contract
    pub fn register_layout(&self, contract: ContractAddress, layout: ContractStorageLayout) -> StateAccessResult<()> {
        let mut layouts = self.layouts.write().map_err(|e| StateAccessError::StorageError(format!("Lock error: {e}")))?;

        layouts.insert(contract, layout);
        Ok(())
    }

    /// Get the storage layout for a contract
    pub fn get_layout(&self, contract: ContractAddress) -> StateAccessResult<ContractStorageLayout> {
        let layouts = self.layouts.read().map_err(|e| StateAccessError::StorageError(format!("Lock error: {e}")))?;

        layouts.get(&contract).cloned().ok_or(StateAccessError::LayoutNotFound(contract))
    }

    /// Generate MPT key from contract address and storage key
    fn generate_mpt_key(&self, contract: ContractAddress, storage_key: &[u8]) -> StateAccessResult<Key> {
        let layout = self.get_layout(contract)?;

        // For now, use simple slot-based key generation
        // In practice, this would use the storage layout to determine the appropriate key generation method
        if storage_key.len() != 32 {
            return Err(StateAccessError::InvalidKey("Storage key must be 32 bytes".to_string()));
        }

        // Convert storage key to slot number (simplified)
        let slot = u32::from_be_bytes([storage_key[28], storage_key[29], storage_key[30], storage_key[31]]);

        layout.generate_storage_key(slot).map_err(|e| StateAccessError::KeyEncodingError(e.to_string()))
    }

    /// Update cache with a value
    fn update_cache(&self, contract: ContractAddress, storage_key: &[u8], value: Option<Vec<u8>>) {
        if !self.config.enable_cache {
            return;
        }

        if let Ok(mut cache) = self.cache.write() {
            let cache_key = CacheKey {
                contract,
                storage_key: storage_key.to_vec(),
            };

            let entry = CacheEntry {
                value,
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                access_count: 1,
            };

            cache.insert(cache_key, entry);
        }
    }

    /// Get value from cache
    fn get_from_cache(&self, contract: ContractAddress, storage_key: &[u8]) -> Option<Option<Vec<u8>>> {
        if !self.config.enable_cache {
            return None;
        }

        if let Ok(mut cache) = self.cache.write() {
            let cache_key = CacheKey {
                contract,
                storage_key: storage_key.to_vec(),
            };

            if let Some(entry) = cache.entries.get_mut(&cache_key) {
                // Check TTL
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

                if now - entry.timestamp < self.config.cache_ttl_seconds {
                    entry.access_count += 1;
                    return Some(entry.value.clone());
                } else {
                    // Entry expired, remove it
                    cache.entries.remove(&cache_key);
                    cache.size -= 1;
                }
            }
        }

        None
    }

    /// Invalidate cache entry
    fn invalidate_cache(&self, contract: ContractAddress, storage_key: &[u8]) {
        if !self.config.enable_cache {
            return;
        }

        if let Ok(mut cache) = self.cache.write() {
            let cache_key = CacheKey {
                contract,
                storage_key: storage_key.to_vec(),
            };

            if cache.entries.remove(&cache_key).is_some() {
                cache.size -= 1;
            }
        }
    }

    /// Generate dot prefix for MPT key filtering
    fn generate_dot_prefix(&self, dot: ContractAddress) -> StateAccessResult<Vec<u8>> {
        // Create a prefix from dot address for efficient key filtering
        let mut prefix = Vec::new();
        prefix.extend_from_slice(&dot); // ContractAddress is [u8; 20]
        prefix.push(0xFF); // Separator to avoid key collisions
        Ok(prefix)
    }

    /// Extract storage key from MPT key by removing dot prefix
    fn extract_storage_key_from_mpt(&self, mpt_key: &Key, dot: ContractAddress) -> StateAccessResult<Option<Vec<u8>>> {
        let dot_prefix = self.generate_dot_prefix(dot)?;
        let key_bytes: &[u8] = mpt_key.as_ref(); // Key implements AsRef<[u8]>

        // Check if key starts with dot prefix
        if key_bytes.len() <= dot_prefix.len() || !key_bytes.starts_with(&dot_prefix) {
            return Ok(None);
        }

        // Extract storage key part (after dot prefix)
        let storage_key = key_bytes[dot_prefix.len()..].to_vec();
        Ok(Some(storage_key))
    }
}

impl<S: NodeStorage> StateAccessInterface for StateAccessManager<S> {
    fn load_storage(&self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<Option<Vec<u8>>> {
        // Check cache first
        if let Some(cached_value) = self.get_from_cache(contract, key) {
            return Ok(cached_value);
        }

        // Generate MPT key
        let mpt_key = self.generate_mpt_key(contract, key)?;

        // Read from trie
        let trie = self.trie.read().map_err(|e| StateAccessError::StorageError(format!("Lock error: {e}")))?;

        let result = trie.get(&mpt_key).map_err(|e| StateAccessError::StorageError(e.to_string()))?.map(|value| value);

        // Update cache
        self.update_cache(contract, key, result.clone());

        Ok(result)
    }

    fn store_storage(&mut self, contract: ContractAddress, key: &[u8], value: &[u8]) -> StateAccessResult<()> {
        // Generate MPT key
        let mpt_key = self.generate_mpt_key(contract, key)?;

        // Store in trie
        let mut trie = self.trie.write().map_err(|e| StateAccessError::StorageError(format!("Lock error: {e}")))?;

        let mpt_value = Value::from(value.to_vec());
        trie.put(mpt_key, mpt_value).map_err(|e| StateAccessError::StorageError(e.to_string()))?;

        // Invalidate cache
        self.invalidate_cache(contract, key);

        Ok(())
    }

    fn storage_exists(&self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<bool> {
        // Check cache first
        if let Some(cached_value) = self.get_from_cache(contract, key) {
            return Ok(cached_value.is_some());
        }

        // Generate MPT key
        let mpt_key = self.generate_mpt_key(contract, key)?;

        // Check existence in trie
        let trie = self.trie.read().map_err(|e| StateAccessError::StorageError(format!("Lock error: {e}")))?;

        let exists = trie.get(&mpt_key).map_err(|e| StateAccessError::StorageError(e.to_string()))?.is_some();

        Ok(exists)
    }

    fn storage_size(&self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<usize> {
        match self.load_storage(contract, key)? {
            Some(value) => Ok(value.len()),
            None => Ok(0),
        }
    }

    fn clear_storage(&mut self, contract: ContractAddress, key: &[u8]) -> StateAccessResult<()> {
        // Generate MPT key
        let mpt_key = self.generate_mpt_key(contract, key)?;

        // Remove from trie
        let mut trie = self.trie.write().map_err(|e| StateAccessError::StorageError(format!("Lock error: {e}")))?;

        trie.delete(&mpt_key).map_err(|e| StateAccessError::StorageError(e.to_string()))?;

        // Invalidate cache
        self.invalidate_cache(contract, key);

        Ok(())
    }

    fn multi_load(&self, contract: ContractAddress, keys: &[Vec<u8>]) -> StateAccessResult<Vec<Option<Vec<u8>>>> {
        if keys.len() > self.config.max_batch_size {
            return Err(StateAccessError::BatchTooLarge {
                size: keys.len(),
                max: self.config.max_batch_size,
            });
        }

        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            let result = self.load_storage(contract, key)?;
            results.push(result);
        }

        Ok(results)
    }

    fn multi_store(&mut self, contract: ContractAddress, entries: &[(Vec<u8>, Vec<u8>)]) -> StateAccessResult<()> {
        if entries.len() > self.config.max_batch_size {
            return Err(StateAccessError::BatchTooLarge {
                size: entries.len(),
                max: self.config.max_batch_size,
            });
        }

        for (key, value) in entries {
            self.store_storage(contract, key, value)?;
        }

        Ok(())
    }

    fn get_storage_keys(&self, dot: ContractAddress, start_key: Option<&[u8]>, limit: usize) -> StateAccessResult<Vec<Vec<u8>>> {
        // Generate dot prefix for filtering keys
        let dot_prefix = self.generate_dot_prefix(dot)?;

        // Get trie read lock
        let trie = self.trie.read().map_err(|_| StateAccessError::ConcurrencyError)?;

        // Get all keys from trie and filter by dot prefix
        let all_keys = trie.get_all_keys().map_err(|e| StateAccessError::StorageError(e.to_string()))?;

        let mut filtered_keys = Vec::new();
        let mut count = 0;

        // Filter keys by dot prefix and apply pagination
        for key in all_keys {
            let key_bytes: &[u8] = key.as_ref();

            // Check if key starts with dot prefix
            if !key_bytes.starts_with(&dot_prefix) {
                continue;
            }

            // Extract storage key from MPT key (remove dot prefix)
            if let Some(storage_key) = self.extract_storage_key_from_mpt(&key, dot)? {
                // Apply start_key constraint
                if let Some(start) = start_key {
                    if storage_key.as_slice() < start {
                        continue;
                    }
                }

                filtered_keys.push(storage_key);
                count += 1;

                // Apply limit
                if count >= limit {
                    break;
                }
            }
        }

        // Sort keys for consistent ordering
        filtered_keys.sort();

        Ok(filtered_keys)
    }
}

impl StateCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            size: 0,
            max_size,
        }
    }

    fn insert(&mut self, key: CacheKey, entry: CacheEntry) {
        // Remove oldest entry if cache is full
        if self.size >= self.max_size && !self.entries.contains_key(&key) {
            self.evict_oldest();
        }

        let is_new = self.entries.insert(key, entry).is_none();
        if is_new {
            self.size += 1;
        }
    }

    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self.entries.iter().min_by_key(|(_, entry)| entry.timestamp).map(|(k, v)| (k.clone(), v.clone())) {
            self.entries.remove(&oldest_key);
            self.size -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotdb_core::state::{ContractStorageLayout, StorageVariableType, create_in_memory_mpt};

    fn create_test_manager() -> StateAccessManager<dotdb_core::state::db_interface::MptStorageAdapter> {
        let trie = create_in_memory_mpt().unwrap();
        StateAccessManager::new(trie)
    }

    fn create_test_layout() -> ContractStorageLayout {
        let contract_addr = [1u8; 20];
        let mut layout = ContractStorageLayout::new(contract_addr);
        layout.add_variable("balance".to_string(), StorageVariableType::Simple).unwrap();
        layout
    }

    #[test]
    fn test_state_access_manager_creation() {
        let manager = create_test_manager();
        assert!(manager.trie.read().is_ok());
        assert!(manager.layouts.read().is_ok());
        assert!(manager.cache.read().is_ok());
    }

    #[test]
    fn test_layout_registration() {
        let manager = create_test_manager();
        let contract_addr = [1u8; 20];
        let layout = create_test_layout();

        assert!(manager.register_layout(contract_addr, layout).is_ok());
        assert!(manager.get_layout(contract_addr).is_ok());
    }

    #[test]
    fn test_layout_not_found() {
        let manager = create_test_manager();
        let contract_addr = [1u8; 20];

        assert!(matches!(manager.get_layout(contract_addr), Err(StateAccessError::LayoutNotFound(_))));
    }

    #[test]
    fn test_storage_operations() {
        let mut manager = create_test_manager();
        let contract_addr = [1u8; 20];
        let layout = create_test_layout();

        // Register layout
        manager.register_layout(contract_addr, layout).unwrap();

        // Test key and value
        let key = vec![0u8; 32];
        let value = b"test_value";

        // Initially should not exist
        assert!(!manager.storage_exists(contract_addr, &key).unwrap());
        assert_eq!(manager.storage_size(contract_addr, &key).unwrap(), 0);
        assert!(manager.load_storage(contract_addr, &key).unwrap().is_none());

        // Store value
        manager.store_storage(contract_addr, &key, value).unwrap();

        // Should now exist
        assert!(manager.storage_exists(contract_addr, &key).unwrap());
        assert_eq!(manager.storage_size(contract_addr, &key).unwrap(), value.len());
        assert_eq!(manager.load_storage(contract_addr, &key).unwrap().unwrap(), value);

        // Clear value
        manager.clear_storage(contract_addr, &key).unwrap();

        // Should be gone
        assert!(!manager.storage_exists(contract_addr, &key).unwrap());
        assert!(manager.load_storage(contract_addr, &key).unwrap().is_none());
    }

    #[test]
    fn test_multi_operations() {
        let mut manager = create_test_manager();
        let contract_addr = [1u8; 20];
        let layout = create_test_layout();

        manager.register_layout(contract_addr, layout).unwrap();

        // Prepare test data
        let entries = vec![(vec![0u8; 32], b"value1".to_vec()), (vec![1u8; 32], b"value2".to_vec()), (vec![2u8; 32], b"value3".to_vec())];

        let keys: Vec<Vec<u8>> = entries.iter().map(|(k, _)| k.clone()).collect();

        // Multi store
        manager.multi_store(contract_addr, &entries).unwrap();

        // Multi load
        let results = manager.multi_load(contract_addr, &keys).unwrap();

        assert_eq!(results.len(), 3);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.as_ref().unwrap(), &entries[i].1);
        }
    }

    #[test]
    fn test_batch_size_limits() {
        let mut manager = create_test_manager();
        let contract_addr = [1u8; 20];
        let layout = create_test_layout();

        manager.register_layout(contract_addr, layout).unwrap();

        // Create oversized batch
        let large_batch: Vec<(Vec<u8>, Vec<u8>)> = (0..200).map(|i| (vec![i as u8; 32], vec![i as u8])).collect();

        let large_keys: Vec<Vec<u8>> = (0..200).map(|i| vec![i as u8; 32]).collect();

        // Should fail due to batch size
        assert!(matches!(manager.multi_store(contract_addr, &large_batch), Err(StateAccessError::BatchTooLarge { .. })));

        assert!(matches!(manager.multi_load(contract_addr, &large_keys), Err(StateAccessError::BatchTooLarge { .. })));
    }

    #[test]
    fn test_invalid_key_format() {
        let manager = create_test_manager();
        let contract_addr = [1u8; 20];
        let layout = create_test_layout();

        manager.register_layout(contract_addr, layout).unwrap();

        // Invalid key length
        let invalid_key = vec![0u8; 16]; // Should be 32 bytes

        assert!(matches!(manager.load_storage(contract_addr, &invalid_key), Err(StateAccessError::InvalidKey(_))));
    }

    #[test]
    fn test_cache_functionality() {
        let mut manager = create_test_manager();
        let contract_addr = [1u8; 20];
        let layout = create_test_layout();

        manager.register_layout(contract_addr, layout).unwrap();

        let key = vec![0u8; 32];
        let value = b"cached_value";

        // Store value
        manager.store_storage(contract_addr, &key, value).unwrap();

        // First load should populate cache
        let result1 = manager.load_storage(contract_addr, &key).unwrap();
        assert_eq!(result1.unwrap(), value);

        // Second load should use cache (we can't directly test this without cache statistics)
        let result2 = manager.load_storage(contract_addr, &key).unwrap();
        assert_eq!(result2.unwrap(), value);
    }

    #[test]
    fn test_config_creation() {
        let config = StateAccessConfig::default();
        assert_eq!(config.max_cache_size, 10000);
        assert!(config.enable_cache);
        assert!(config.enable_batch_optimization);
        assert_eq!(config.max_batch_size, 100);
        assert_eq!(config.cache_ttl_seconds, 300);
    }

    #[test]
    fn test_cache_key_ordering() {
        let key1 = CacheKey {
            contract: [1u8; 20],
            storage_key: vec![0u8; 32],
        };

        let key2 = CacheKey {
            contract: [1u8; 20],
            storage_key: vec![1u8; 32],
        };

        let key3 = CacheKey {
            contract: [2u8; 20],
            storage_key: vec![0u8; 32],
        };

        assert!(key1 < key2);
        assert!(key1 < key3);
        assert!(key2 > key1);
    }

    #[test]
    fn test_error_display() {
        let error = StateAccessError::ContractNotFound([1u8; 20]);
        assert!(error.to_string().contains("Contract not found"));

        let error = StateAccessError::BatchTooLarge { size: 200, max: 100 };
        assert!(error.to_string().contains("Batch too large"));

        let error = StateAccessError::StorageError("Test error".to_string());
        assert!(error.to_string().contains("Storage error: Test error"));
    }
}
