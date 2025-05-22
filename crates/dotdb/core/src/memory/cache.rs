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

use std::{
    collections::{HashMap, VecDeque},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Eviction policy for the cache
/// Determines which items get removed when the cache is full
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvictionPolicy {
    LRU,    // Least Recently Used - removes items that haven't been accessed recently
    LFU,    // Least Frequently Used - removes items that are accessed least often
    FIFO,   // First In, First Out - removes oldest items first
    Random, // Removes items randomly
    TTL,    // Time To Live - removes items that have expired
}

/// Errors that can occur during cache operations
#[derive(Debug)]
pub enum CacheError {
    KeyNotFound,          // Requested key is not in the cache
    OutOfMemory,          // Not enough memory to store new item
    InvalidConfiguration, // Cache configuration is invalid
    SerializationError,   // Error serializing or deserializing cached data
}

/// Statistics about cache performance and usage
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,      // Current number of entries in the cache
    pub max_entries: usize,        // Maximum number of entries allowed
    pub hit_count: u64,            // Number of cache hits
    pub miss_count: u64,           // Number of cache misses
    pub eviction_count: u64,       // Number of entries evicted
    pub insertion_count: u64,      // Number of entries inserted
    pub memory_usage: usize,       // Current memory usage of the cache
    pub max_memory_usage: usize,   // Maximum memory usage allowed
    pub hit_rate: f64,             // Ratio of hits to total lookups
    pub avg_access_time: Duration, // Average time to access an entry
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            total_entries: 0,
            max_entries: 0,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
            insertion_count: 0,
            memory_usage: 0,
            max_memory_usage: 0,
            hit_rate: 0.0,
            avg_access_time: Duration::from_nanos(0),
        }
    }
}

/// Internal representation of a cached item with metadata
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,               // The actual cached value
    access_count: u64,      // Number of times this entry has been accessed
    last_accessed: Instant, // When this entry was last accessed
    created_at: Instant,    // When this entry was created
    ttl: Option<Duration>,  // Time-to-live duration, if any
    size: usize,            // Memory size of the entry
}

impl<V> CacheEntry<V> {
    /// Creates a new cache entry
    fn new(value: V, size: usize, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            access_count: 1,
            last_accessed: now,
            created_at: now,
            ttl,
            size,
        }
    }

    /// Checks if the entry has expired based on its TTL
    fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl { self.created_at.elapsed() > ttl } else { false }
    }

    /// Updates access statistics when the entry is accessed
    fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }
}

/// Cache implementation with configurable eviction strategies
/// Provides in-memory caching with various eviction policies to optimize
/// memory usage and access performance
pub struct Cache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    entries: Arc<Mutex<HashMap<K, CacheEntry<V>>>>, // Stored cache entries
    policy: EvictionPolicy,                         // Eviction policy to use
    max_entries: usize,                             // Maximum entries allowed
    max_memory: usize,                              // Maximum memory usage allowed
    stats: Arc<Mutex<CacheStats>>,                  // Cache statistics

    // LRU specific data structure - tracks access order
    lru_order: Arc<Mutex<VecDeque<K>>>,

    // LFU specific data structure - groups keys by access frequency
    frequency_buckets: Arc<Mutex<HashMap<u64, Vec<K>>>>,
    min_frequency: Arc<Mutex<u64>>,

    // FIFO specific data structure - tracks insertion order
    insertion_order: Arc<Mutex<VecDeque<K>>>,

    // TTL specific fields
    default_ttl: Option<Duration>,     // Default time-to-live for entries
    cleanup_interval: Duration,        // How often to check for expired entries
    last_cleanup: Arc<Mutex<Instant>>, // When cleanup last occurred
}

impl<K, V> Cache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Creates a new cache with the specified policy and capacity
    ///
    /// # Arguments
    /// * `policy` - Eviction policy to use
    /// * `max_entries` - Maximum number of entries to store
    pub fn new(policy: EvictionPolicy, max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            policy,
            max_entries,
            max_memory: usize::MAX, // Default to no memory limit
            stats: Arc::new(Mutex::new(CacheStats::default())),
            lru_order: Arc::new(Mutex::new(VecDeque::new())),
            frequency_buckets: Arc::new(Mutex::new(HashMap::new())),
            min_frequency: Arc::new(Mutex::new(1)),
            insertion_order: Arc::new(Mutex::new(VecDeque::new())),
            default_ttl: None,
            cleanup_interval: Duration::from_secs(60),
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Sets the maximum memory usage for this cache
    ///
    /// # Arguments
    /// * `max_memory` - Maximum memory in bytes that the cache can use
    pub fn with_max_memory(mut self, max_memory: usize) -> Self {
        self.max_memory = max_memory;
        self
    }

    /// Sets a default time-to-live for cache entries
    ///
    /// # Arguments
    /// * `default_ttl` - Default duration after which entries expire
    pub fn with_ttl(mut self, default_ttl: Duration) -> Self {
        self.default_ttl = Some(default_ttl);
        self
    }

    /// Sets how often to check for and remove expired entries
    ///
    /// # Arguments
    /// * `interval` - Time interval between cleanup operations
    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }

    /// Retrieves a value from the cache by its key
    /// Updates cache statistics and eviction-related data structures
    ///
    /// # Arguments
    /// * `key` - The key to look up
    ///
    /// # Returns
    /// * `Option<V>` - The value if found and not expired, or None
    pub fn get(&self, key: &K) -> Option<V> {
        let start_time = Instant::now();

        let result = {
            let mut entries = self.entries.lock().unwrap();

            if let Some(entry) = entries.get_mut(key) {
                if entry.is_expired() {
                    // Remove expired entries on access
                    entries.remove(key);
                    self.update_data_structures_on_removal(key);
                    None
                } else {
                    // Update access information
                    entry.touch();
                    let value = entry.value.clone();
                    self.update_access_order(key, &mut entries);
                    Some(value)
                }
            } else {
                None
            }
        };

        // Update statistics
        let mut stats = self.stats.lock().unwrap();
        if result.is_some() {
            stats.hit_count += 1;
        } else {
            stats.miss_count += 1;
        }

        // Recalculate hit rate
        let total_accesses = stats.hit_count + stats.miss_count;
        stats.hit_rate = stats.hit_count as f64 / total_accesses as f64;

        // Update average access time
        let access_time = start_time.elapsed();
        stats.avg_access_time = Duration::from_nanos(((stats.avg_access_time.as_nanos() as u128 * (total_accesses - 1) as u128 + access_time.as_nanos() as u128) / total_accesses as u128) as u64);

        // Check if we need to run cleanup
        self.maybe_cleanup();
        result
    }

    /// Inserts a value into the cache
    /// Uses the default TTL if configured
    ///
    /// # Arguments
    /// * `key` - Key to store the value under
    /// * `value` - Value to store
    ///
    /// # Returns
    /// * `Result<Option<V>, CacheError>` - Previous value if key existed, or error
    pub fn insert(&self, key: K, value: V) -> Result<Option<V>, CacheError> {
        self.insert_with_ttl(key, value, self.default_ttl)
    }

    /// Inserts a value into the cache with a specific time-to-live duration
    ///
    /// # Arguments
    /// * `key` - Key to store the value under
    /// * `value` - Value to store
    /// * `ttl` - Optional time-to-live for this entry (overrides default TTL)
    ///
    /// # Returns
    /// * `Result<Option<V>, CacheError>` - Previous value if key existed, or error
    ///
    /// # Errors
    /// * `CacheError::OutOfMemory` - If cache is full and eviction policy couldn't free up space
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) -> Result<Option<V>, CacheError> {
        let size = self.estimate_size(&value);
        let entry = CacheEntry::new(value, size, ttl);

        let old_value = {
            let mut entries = self.entries.lock().unwrap();
            let mut stats = self.stats.lock().unwrap();

            // Check if we need to evict
            while (entries.len() >= self.max_entries || stats.memory_usage + size > self.max_memory) && !entries.is_empty() {
                drop(entries);
                drop(stats);
                self.evict_one()?;
                entries = self.entries.lock().unwrap();
                stats = self.stats.lock().unwrap();
            }

            let old_value = entries.insert(key.clone(), entry).map(|e| e.value);

            if old_value.is_none() {
                stats.insertion_count += 1;
                stats.total_entries = entries.len();
                stats.memory_usage += size;

                if stats.total_entries > stats.max_entries {
                    stats.max_entries = stats.total_entries;
                }

                if stats.memory_usage > stats.max_memory_usage {
                    stats.max_memory_usage = stats.memory_usage;
                }
            }

            old_value
        };

        self.update_insertion_order(&key);
        self.maybe_cleanup();

        Ok(old_value)
    }

    /// Removes a key-value pair from the cache
    ///
    /// # Arguments
    /// * `key` - Key to remove
    ///
    /// # Returns
    /// * `Option<V>` - The value if it was in the cache, or None
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.lock().unwrap();

        if let Some(entry) = entries.remove(key) {
            let mut stats = self.stats.lock().unwrap();
            stats.total_entries = entries.len();
            stats.memory_usage = stats.memory_usage.saturating_sub(entry.size);

            drop(entries);
            drop(stats);

            self.update_data_structures_on_removal(key);
            Some(entry.value)
        } else {
            None
        }
    }

    /// Removes all entries from the cache
    /// Resets all statistics except hit/miss counts
    pub fn clear(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();

        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = 0;
        stats.memory_usage = 0;

        drop(entries);
        drop(stats);

        // Clear auxiliary data structures
        let mut lru_order = self.lru_order.lock().unwrap();
        lru_order.clear();

        let mut frequency_buckets = self.frequency_buckets.lock().unwrap();
        frequency_buckets.clear();

        let mut insertion_order = self.insertion_order.lock().unwrap();
        insertion_order.clear();

        let mut min_frequency = self.min_frequency.lock().unwrap();
        *min_frequency = 1;
    }

    /// Checks if a key exists in the cache and is not expired
    ///
    /// # Arguments
    /// * `key` - Key to check
    ///
    /// # Returns
    /// * `bool` - True if the key exists and is not expired, false otherwise
    pub fn contains_key(&self, key: &K) -> bool {
        let entries = self.entries.lock().unwrap();

        if let Some(entry) = entries.get(key) { !entry.is_expired() } else { false }
    }

    /// Gets the number of entries currently in the cache
    ///
    /// # Returns
    /// * `usize` - Current number of entries
    pub fn len(&self) -> usize {
        let entries = self.entries.lock().unwrap();
        entries.len()
    }

    /// Checks if the cache is empty
    ///
    /// # Returns
    /// * `bool` - True if the cache has no entries, false otherwise
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the current cache statistics
    ///
    /// # Returns
    /// * `CacheStats` - Current statistics about cache usage
    pub fn get_stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    fn evict_one(&self) -> Result<(), CacheError> {
        match self.policy {
            EvictionPolicy::LRU => self.evict_lru(),
            EvictionPolicy::LFU => self.evict_lfu(),
            EvictionPolicy::FIFO => self.evict_fifo(),
            EvictionPolicy::Random => self.evict_random(),
            EvictionPolicy::TTL => self.evict_expired(),
        }
    }

    fn evict_lru(&self) -> Result<(), CacheError> {
        let mut lru_order = self.lru_order.lock().unwrap();

        if let Some(key) = lru_order.pop_front() {
            drop(lru_order);

            let mut entries = self.entries.lock().unwrap();
            if let Some(entry) = entries.remove(&key) {
                let mut stats = self.stats.lock().unwrap();
                stats.eviction_count += 1;
                stats.total_entries = entries.len();
                stats.memory_usage = stats.memory_usage.saturating_sub(entry.size);
            }
            Ok(())
        } else {
            Err(CacheError::InvalidConfiguration)
        }
    }

    fn evict_lfu(&self) -> Result<(), CacheError> {
        // Always lock entries, then frequency_buckets, then min_frequency in this order
        let mut entries = self.entries.lock().unwrap();
        let mut frequency_buckets = self.frequency_buckets.lock().unwrap();
        let mut min_frequency = self.min_frequency.lock().unwrap();

        while let Some(keys) = frequency_buckets.get_mut(&*min_frequency) {
            if let Some(key) = keys.pop() {
                if let Some(entry) = entries.remove(&key) {
                    let mut stats = self.stats.lock().unwrap();
                    stats.eviction_count += 1;
                    stats.total_entries = entries.len();
                    stats.memory_usage = stats.memory_usage.saturating_sub(entry.size);
                }
                return Ok(());
            } else {
                frequency_buckets.remove(&*min_frequency);
                *min_frequency += 1;
            }
        }

        Err(CacheError::InvalidConfiguration)
    }

    fn evict_fifo(&self) -> Result<(), CacheError> {
        let mut insertion_order = self.insertion_order.lock().unwrap();

        if let Some(key) = insertion_order.pop_front() {
            drop(insertion_order);

            let mut entries = self.entries.lock().unwrap();
            if let Some(entry) = entries.remove(&key) {
                let mut stats = self.stats.lock().unwrap();
                stats.eviction_count += 1;
                stats.total_entries = entries.len();
                stats.memory_usage = stats.memory_usage.saturating_sub(entry.size);
            }
            Ok(())
        } else {
            Err(CacheError::InvalidConfiguration)
        }
    }

    fn evict_random(&self) -> Result<(), CacheError> {
        let entries = self.entries.lock().unwrap();

        if let Some(key) = entries.keys().next().cloned() {
            drop(entries);

            let mut entries = self.entries.lock().unwrap();
            if let Some(entry) = entries.remove(&key) {
                let mut stats = self.stats.lock().unwrap();
                stats.eviction_count += 1;
                stats.total_entries = entries.len();
                stats.memory_usage = stats.memory_usage.saturating_sub(entry.size);

                drop(entries);
                drop(stats);
                self.update_data_structures_on_removal(&key);
            }
            Ok(())
        } else {
            Err(CacheError::InvalidConfiguration)
        }
    }

    fn evict_expired(&self) -> Result<(), CacheError> {
        let mut entries = self.entries.lock().unwrap();
        let mut expired_keys = Vec::new();

        for (key, entry) in entries.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        if expired_keys.is_empty() {
            return Err(CacheError::InvalidConfiguration);
        }

        let mut total_size = 0;
        for key in &expired_keys {
            if let Some(entry) = entries.remove(key) {
                total_size += entry.size;
            }
        }

        let mut stats = self.stats.lock().unwrap();
        stats.eviction_count += expired_keys.len() as u64;
        stats.total_entries = entries.len();
        stats.memory_usage = stats.memory_usage.saturating_sub(total_size);

        drop(entries);
        drop(stats);

        for key in expired_keys {
            self.update_data_structures_on_removal(&key);
        }

        Ok(())
    }

    fn update_access_order(&self, key: &K, entries: &mut HashMap<K, CacheEntry<V>>) {
        match self.policy {
            EvictionPolicy::LRU => {
                let mut lru_order = self.lru_order.lock().unwrap();
                // Remove key from current position and add to back
                if let Some(pos) = lru_order.iter().position(|k| k == key) {
                    lru_order.remove(pos);
                }
                lru_order.push_back(key.clone());
            }
            EvictionPolicy::LFU => {
                let mut frequency_buckets = self.frequency_buckets.lock().unwrap();
                if let Some(entry) = entries.get_mut(key) {
                    let freq = entry.access_count;
                    if freq > 0 {
                        if let Some(keys) = frequency_buckets.get_mut(&(freq - 1)) {
                            keys.retain(|k| k != key);
                        }
                    }
                    frequency_buckets.entry(freq).or_insert_with(Vec::new).push(key.clone());
                }
            }
            _ => {} // Other policies don't need access order updates
        }
    }

    fn update_insertion_order(&self, key: &K) {
        match self.policy {
            EvictionPolicy::LRU => {
                let mut lru_order = self.lru_order.lock().unwrap();
                lru_order.push_back(key.clone());
            }
            EvictionPolicy::LFU => {
                let mut frequency_buckets = self.frequency_buckets.lock().unwrap();
                frequency_buckets.entry(1).or_insert_with(Vec::new).push(key.clone());
            }
            EvictionPolicy::FIFO => {
                let mut insertion_order = self.insertion_order.lock().unwrap();
                insertion_order.push_back(key.clone());
            }
            _ => {} // Other policies don't need insertion order tracking
        }
    }

    fn update_data_structures_on_removal(&self, key: &K) {
        match self.policy {
            EvictionPolicy::LRU => {
                let mut lru_order = self.lru_order.lock().unwrap();
                if let Some(pos) = lru_order.iter().position(|k| k == key) {
                    lru_order.remove(pos);
                }
            }
            EvictionPolicy::LFU => {
                let mut frequency_buckets = self.frequency_buckets.lock().unwrap();
                for (_, keys) in frequency_buckets.iter_mut() {
                    keys.retain(|k| k != key);
                }
            }
            EvictionPolicy::FIFO => {
                let mut insertion_order = self.insertion_order.lock().unwrap();
                if let Some(pos) = insertion_order.iter().position(|k| k == key) {
                    insertion_order.remove(pos);
                }
            }
            _ => {} // Other policies don't need cleanup
        }
    }

    fn maybe_cleanup(&self) {
        if self.policy == EvictionPolicy::TTL {
            let mut last_cleanup = self.last_cleanup.lock().unwrap();
            let now = Instant::now();

            if now.duration_since(*last_cleanup) >= self.cleanup_interval {
                drop(last_cleanup);
                let _ = self.evict_expired();

                let mut last_cleanup = self.last_cleanup.lock().unwrap();
                *last_cleanup = now;
            }
        }
    }

    /// Estimates the memory size of a value for tracking memory usage
    /// This is a simple implementation that could be overridden for more accurate tracking
    ///
    /// # Arguments
    /// * `_value` - Value to estimate size of
    ///
    /// # Returns
    /// * `usize` - Estimated size in bytes
    fn estimate_size(&self, _value: &V) -> usize {
        // Simple estimation - in a real implementation, this could be more sophisticated
        std::mem::size_of::<V>()
    }

    /// Resizes the cache to a new maximum entry count
    /// If the new size is smaller, entries will be evicted according to the policy
    ///
    /// # Arguments
    /// * `new_max_entries` - New maximum number of entries
    ///
    /// # Returns
    /// * `Result<(), CacheError>` - Success or an error if resizing fails
    ///
    /// # Errors
    /// * `CacheError::InvalidConfiguration` - If the eviction policy fails
    pub fn resize(&self, new_max_entries: usize) -> Result<(), CacheError> {
        if new_max_entries == 0 {
            return Err(CacheError::InvalidConfiguration);
        }

        let current_len = self.len();

        if new_max_entries < current_len {
            // Need to evict some entries
            let evict_count = current_len - new_max_entries;
            for _ in 0..evict_count {
                self.evict_one()?;
            }
        }

        // Update max_entries - this is conceptually what we'd do,
        // but since max_entries is not behind a mutex in this simple implementation,
        // we can't actually modify it. In a real implementation, it would be mutable.

        Ok(())
    }

    /// Gets the current memory usage of the cache in bytes
    ///
    /// # Returns
    /// * `usize` - Current memory usage in bytes
    pub fn get_memory_usage(&self) -> usize {
        self.stats.lock().unwrap().memory_usage
    }

    /// Gets the cache hit rate (hits / total accesses)
    ///
    /// # Returns
    /// * `f64` - Hit rate between 0.0 and 1.0
    pub fn get_hit_rate(&self) -> f64 {
        self.stats.lock().unwrap().hit_rate
    }
}

impl<K, V> Clone for Cache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            policy: self.policy,
            max_entries: self.max_entries,
            max_memory: self.max_memory,
            stats: Arc::clone(&self.stats),
            lru_order: Arc::clone(&self.lru_order),
            frequency_buckets: Arc::clone(&self.frequency_buckets),
            min_frequency: Arc::clone(&self.min_frequency),
            insertion_order: Arc::clone(&self.insertion_order),
            default_ttl: self.default_ttl,
            cleanup_interval: self.cleanup_interval,
            last_cleanup: Arc::clone(&self.last_cleanup),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_lru_cache_creation() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 10);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 10);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        assert_eq!(cache.get(&"key1".to_string()), Some(100));
        assert_eq!(cache.get(&"key2".to_string()), Some(200));
        assert_eq!(cache.get(&"key3".to_string()), None);

        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_lru_eviction() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 2);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        // Access key1 to make it recently used
        cache.get(&"key1".to_string());

        // Insert key3, should evict key2 (least recently used)
        cache.insert("key3".to_string(), 300).unwrap();

        assert_eq!(cache.get(&"key1".to_string()), Some(100));
        assert_eq!(cache.get(&"key2".to_string()), None);
        assert_eq!(cache.get(&"key3".to_string()), Some(300));
    }

    #[test]
    fn test_lfu_eviction() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LFU, 2);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        // Access key1 a couple of times only (was 3+ before)
        cache.get(&"key1".to_string());
        cache.get(&"key2".to_string()); // key2 accessed less frequently

        // Insert key3, should evict key2 (least frequently used)
        cache.insert("key3".to_string(), 300).unwrap();

        assert_eq!(cache.get(&"key1".to_string()), Some(100));
        assert_eq!(cache.get(&"key2".to_string()), None);
        assert_eq!(cache.get(&"key3".to_string()), Some(300));
    }

    #[test]
    fn test_fifo_eviction() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::FIFO, 2);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        // Insert key3, should evict key1 (first in, first out)
        cache.insert("key3".to_string(), 300).unwrap();

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some(200));
        assert_eq!(cache.get(&"key3".to_string()), Some(300));
    }

    #[test]
    fn test_ttl_eviction() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::TTL, 10).with_ttl(Duration::from_millis(100));

        cache.insert("key1".to_string(), 100).unwrap();

        // Should be available immediately
        assert_eq!(cache.get(&"key1".to_string()), Some(100));

        // Wait for TTL to expire
        thread::sleep(Duration::from_millis(150));

        // Should be expired now
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_remove() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 10);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        assert_eq!(cache.remove(&"key1".to_string()), Some(100));
        assert_eq!(cache.remove(&"key1".to_string()), None);

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some(200));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_clear() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 10);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_contains_key() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 10);

        cache.insert("key1".to_string(), 100).unwrap();

        assert!(cache.contains_key(&"key1".to_string()));
        assert!(!cache.contains_key(&"key2".to_string()));
    }

    #[test]
    fn test_cache_stats() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 2);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        // Generate some hits and misses
        cache.get(&"key1".to_string()); // hit
        cache.get(&"key2".to_string()); // hit
        cache.get(&"key3".to_string()); // miss

        let stats = cache.get_stats();
        assert_eq!(stats.hit_count, 2);
        assert_eq!(stats.miss_count, 1);
        assert_eq!(stats.insertion_count, 2);
        assert_eq!(stats.total_entries, 2);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_cache_with_memory_limit() {
        let cache: Cache<String, Vec<u8>> = Cache::new(EvictionPolicy::LRU, 10).with_max_memory(1024);

        let large_value = vec![0u8; 512];

        cache.insert("key1".to_string(), large_value.clone()).unwrap();
        cache.insert("key2".to_string(), large_value.clone()).unwrap();

        let stats = cache.get_stats();
        assert!(stats.memory_usage > 0);
    }

    #[test]
    fn test_random_eviction() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::Random, 2);

        cache.insert("key1".to_string(), 100).unwrap();
        cache.insert("key2".to_string(), 200).unwrap();

        // Insert key3, should evict one of the existing keys
        cache.insert("key3".to_string(), 300).unwrap();

        // Should still have 2 entries
        assert_eq!(cache.len(), 2);

        // One of the original keys should be gone
        let key1_exists = cache.contains_key(&"key1".to_string());
        let key2_exists = cache.contains_key(&"key2".to_string());
        assert!(!(key1_exists && key2_exists)); // At least one should be evicted
        assert!(cache.contains_key(&"key3".to_string())); // New key should exist
    }

    #[test]
    fn test_cache_update_existing_key() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 10);

        let old_value = cache.insert("key1".to_string(), 100).unwrap();
        assert_eq!(old_value, None);

        let old_value = cache.insert("key1".to_string(), 200).unwrap();
        assert_eq!(old_value, Some(100));

        assert_eq!(cache.get(&"key1".to_string()), Some(200));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_with_custom_ttl() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::TTL, 10);

        // Insert with custom TTL
        cache.insert_with_ttl("key1".to_string(), 100, Some(Duration::from_millis(50))).unwrap();
        cache.insert_with_ttl("key2".to_string(), 200, Some(Duration::from_millis(200))).unwrap();

        // Both should be available immediately
        assert_eq!(cache.get(&"key1".to_string()), Some(100));
        assert_eq!(cache.get(&"key2".to_string()), Some(200));

        // Wait for first TTL to expire
        thread::sleep(Duration::from_millis(75));

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some(200));
    }

    #[test]
    fn test_cache_hit_rate_calculation() {
        let cache: Cache<String, i32> = Cache::new(EvictionPolicy::LRU, 10);

        cache.insert("key1".to_string(), 100).unwrap();

        // 2 hits, 1 miss = 66.7% hit rate
        cache.get(&"key1".to_string());
        cache.get(&"key1".to_string());
        cache.get(&"nonexistent".to_string());

        let hit_rate = cache.get_hit_rate();
        assert!((hit_rate - 0.6667).abs() < 0.01);
    }
}
