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

use super::lib::{Index, IndexError, IndexKey, IndexMaintenance, IndexResult, IndexStats, IndexType, IndexValue};
use super::persistence::IndexPersistence;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Default initial capacity for hash index
const DEFAULT_INITIAL_CAPACITY: usize = 16;

/// Default load factor threshold for resizing
const DEFAULT_LOAD_FACTOR: f64 = 0.75;

/// Minimum capacity for hash index
const MIN_CAPACITY: usize = 4;

/// Hash index entry
#[derive(Debug, Clone, PartialEq)]
pub struct HashIndexEntry<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// The key
    pub key: K,
    /// The value
    pub value: V,
    /// Hash of the key for quick comparison
    pub hash: u64,
    /// Next entry in case of collision (separate chaining)
    pub next: Option<Box<HashIndexEntry<K, V>>>,
}

impl<K, V> HashIndexEntry<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Create a new hash index entry
    pub fn new(key: K, value: V) -> Self {
        let hash = Self::calculate_hash(&key);
        Self { key, value, hash, next: None }
    }

    /// Calculate hash for a key
    pub fn calculate_hash(key: &K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Get the length of the collision chain starting from this entry
    pub fn chain_length(&self) -> usize {
        let mut count = 1;
        let mut current = &self.next;
        while let Some(entry) = current {
            count += 1;
            current = &entry.next;
        }
        count
    }
}

/// Hashing algorithm types supported
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HashAlgorithm {
    /// Default Rust hasher
    Default,
    /// Robin Hood hashing with linear probing
    RobinHood,
    /// Cuckoo hashing with two hash functions
    Cuckoo,
}

/// Advanced hash index statistics
#[derive(Debug, Clone, Default)]
pub struct AdvancedHashStats {
    /// Current load factor
    pub load_factor: f64,
    /// Number of collisions
    pub collisions: usize,
    /// Maximum chain length
    pub max_chain_length: usize,
    /// Average chain length
    pub average_chain_length: f64,
    /// Number of rehashes performed
    pub rehash_count: usize,
    /// Total probe distance (for open addressing)
    pub total_probe_distance: usize,
    /// Maximum probe distance
    pub max_probe_distance: usize,
    /// Memory utilization ratio
    pub memory_utilization: f64,
}

/// Hash index implementation with multiple collision resolution strategies
pub struct HashIndex<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Array of hash buckets (separate chaining)
    buckets: Vec<Option<Box<HashIndexEntry<K, V>>>>,
    /// Current number of entries
    size: usize,
    /// Current capacity (number of buckets)
    capacity: usize,
    /// Load factor threshold for resizing
    load_factor: f64,
    /// Total number of collisions
    collisions: usize,
    /// Maximum chain length observed
    max_chain_length: usize,
    /// Hashing algorithm in use
    algorithm: HashAlgorithm,
    /// Advanced statistics
    advanced_stats: AdvancedHashStats,
    /// Robin Hood specific data (displacement values)
    displacements: Vec<usize>,
    /// Cuckoo hashing: second hash table
    cuckoo_table2: Vec<Option<Box<HashIndexEntry<K, V>>>>,
    /// Cuckoo hashing: maximum number of relocations
    cuckoo_max_relocations: usize,
    /// Number of rehashes performed
    rehash_count: usize,
}

impl<K, V> HashIndex<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Create a new hash index with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_INITIAL_CAPACITY)
    }

    /// Create a new hash index with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(MIN_CAPACITY).next_power_of_two();
        Self {
            buckets: vec![None; capacity],
            size: 0,
            capacity,
            load_factor: DEFAULT_LOAD_FACTOR,
            collisions: 0,
            max_chain_length: 0,
            algorithm: HashAlgorithm::Default,
            advanced_stats: AdvancedHashStats::default(),
            displacements: vec![0; capacity],
            cuckoo_table2: vec![None; capacity],
            cuckoo_max_relocations: 8,
            rehash_count: 0,
        }
    }

    /// Create a new hash index with custom load factor
    pub fn with_load_factor(capacity: usize, load_factor: f64) -> Self {
        let mut index = Self::with_capacity(capacity);
        index.load_factor = load_factor.clamp(0.1, 1.0);
        index
    }

    /// Create a new hash index with Robin Hood hashing
    pub fn with_robin_hood(capacity: usize) -> Self {
        let mut index = Self::with_capacity(capacity);
        index.algorithm = HashAlgorithm::RobinHood;
        index.load_factor = 0.9; // Robin Hood can handle higher load factors
        index
    }

    /// Create a new hash index with Cuckoo hashing
    pub fn with_cuckoo(capacity: usize) -> Self {
        let mut index = Self::with_capacity(capacity);
        index.algorithm = HashAlgorithm::Cuckoo;
        index.load_factor = 0.5; // Cuckoo hashing works best with lower load factors
        index
    }

    /// Set the hashing algorithm
    pub fn set_algorithm(&mut self, algorithm: HashAlgorithm) -> IndexResult<()> {
        if self.algorithm != algorithm {
            // Need to rebuild with new algorithm
            let entries = self.entries();
            self.algorithm = algorithm;
            self.clear();

            // Adjust load factor based on algorithm
            match algorithm {
                HashAlgorithm::Default => self.load_factor = DEFAULT_LOAD_FACTOR,
                HashAlgorithm::RobinHood => self.load_factor = 0.9,
                HashAlgorithm::Cuckoo => self.load_factor = 0.5,
            }

            // Reinsert all entries
            for (key, value) in entries {
                self.insert(key, value)?;
            }
        }
        Ok(())
    }

    /// Get advanced statistics
    pub fn advanced_stats(&self) -> &AdvancedHashStats {
        &self.advanced_stats
    }

    /// Update advanced statistics
    fn update_advanced_stats(&mut self) {
        self.advanced_stats.load_factor = self.load_factor();
        self.advanced_stats.collisions = self.collisions;
        self.advanced_stats.max_chain_length = self.max_chain_length;
        self.advanced_stats.average_chain_length = self.average_chain_length();
        self.advanced_stats.rehash_count = self.rehash_count;

        // Calculate memory utilization
        let used_buckets = self.buckets.iter().filter(|b| b.is_some()).count() + self.cuckoo_table2.iter().filter(|b| b.is_some()).count();
        let total_buckets = self.capacity * 2; // Both tables
        self.advanced_stats.memory_utilization = used_buckets as f64 / total_buckets as f64;
    }

    /// Calculate bucket index for a hash value
    fn bucket_index(&self, hash: u64) -> usize {
        (hash as usize) & (self.capacity - 1) // Fast modulo for power of 2
    }

    /// Check if the index needs to be resized
    fn needs_resize(&self) -> bool {
        (self.size as f64 / self.capacity as f64) > self.load_factor
    }

    /// Resize the hash table
    fn resize(&mut self) -> IndexResult<()> {
        let old_buckets = std::mem::take(&mut self.buckets);
        let old_cuckoo_table2 = std::mem::take(&mut self.cuckoo_table2);
        let old_capacity = self.capacity;

        self.capacity *= 2;
        self.buckets = vec![None; self.capacity];
        self.cuckoo_table2 = vec![None; self.capacity];
        self.displacements = vec![0; self.capacity];
        self.size = 0;
        self.collisions = 0;
        self.max_chain_length = 0;
        self.rehash_count += 1;

        // Rehash all entries from both tables
        for bucket in old_buckets {
            let mut current = bucket;
            while let Some(mut entry) = current {
                let next = entry.next.take();
                self.insert_entry(*entry)?;
                current = next;
            }
        }

        // Rehash entries from cuckoo table 2
        for bucket in old_cuckoo_table2 {
            let mut current = bucket;
            while let Some(mut entry) = current {
                let next = entry.next.take();
                self.insert_entry(*entry)?;
                current = next;
            }
        }

        Ok(())
    }

    /// Insert an entry into the hash table
    fn insert_entry(&mut self, entry: HashIndexEntry<K, V>) -> IndexResult<()> {
        match self.algorithm {
            HashAlgorithm::Default => self.insert_entry_chaining(entry),
            HashAlgorithm::RobinHood => self.insert_entry_robin_hood(entry),
            HashAlgorithm::Cuckoo => self.insert_entry_cuckoo(entry),
        }
    }

    /// Insert using separate chaining (default algorithm)
    fn insert_entry_chaining(&mut self, mut entry: HashIndexEntry<K, V>) -> IndexResult<()> {
        let bucket_idx = self.bucket_index(entry.hash);
        entry.next = None; // Clear the next pointer

        match &mut self.buckets[bucket_idx] {
            None => {
                // Empty bucket, direct insertion
                self.buckets[bucket_idx] = Some(Box::new(entry));
                self.size += 1;
                self.max_chain_length = self.max_chain_length.max(1);
            }
            Some(head) => {
                // Collision, insert at the beginning of the chain
                let old_head = std::mem::replace(head, Box::new(entry));
                head.next = Some(old_head);
                self.collisions += 1;

                // Update chain length statistics
                let chain_len = head.chain_length();
                self.max_chain_length = self.max_chain_length.max(chain_len);
                self.size += 1;
            }
        }

        Ok(())
    }

    /// Insert using Robin Hood hashing with linear probing
    fn insert_entry_robin_hood(&mut self, entry: HashIndexEntry<K, V>) -> IndexResult<()> {
        let mut current_entry = Some(Box::new(entry));
        let mut probe_distance = 0;
        let mut bucket_idx = self.bucket_index(current_entry.as_ref().unwrap().hash);

        loop {
            match &mut self.buckets[bucket_idx] {
                None => {
                    // Empty slot found, insert here
                    self.buckets[bucket_idx] = current_entry;
                    self.displacements[bucket_idx] = probe_distance;
                    self.size += 1;
                    self.advanced_stats.max_probe_distance = self.advanced_stats.max_probe_distance.max(probe_distance);
                    self.advanced_stats.total_probe_distance += probe_distance;
                    break;
                }
                Some(existing) => {
                    let existing_distance = self.displacements[bucket_idx];

                    if probe_distance > existing_distance {
                        // Robin Hood: steal from the rich (swap entries)
                        let old_entry = std::mem::replace(existing, current_entry.unwrap());
                        current_entry = Some(old_entry);
                        self.displacements[bucket_idx] = probe_distance;
                        probe_distance = existing_distance;
                    }

                    // Continue probing
                    probe_distance += 1;
                    bucket_idx = (bucket_idx + 1) % self.capacity;

                    // Prevent infinite loops
                    if probe_distance > self.capacity {
                        return Err(IndexError::InvalidOperation("Robin Hood insertion failed".to_string()));
                    }
                }
            }
        }

        Ok(())
    }

    /// Insert using Cuckoo hashing
    fn insert_entry_cuckoo(&mut self, entry: HashIndexEntry<K, V>) -> IndexResult<()> {
        let mut current_entry = Some(Box::new(entry));
        let mut relocations = 0;
        let mut use_table1 = true;

        while relocations < self.cuckoo_max_relocations {
            let hash = current_entry.as_ref().unwrap().hash;
            let bucket_idx = if use_table1 { self.bucket_index(hash) } else { self.bucket_index_alt(hash) };

            let table = if use_table1 { &mut self.buckets } else { &mut self.cuckoo_table2 };

            match &mut table[bucket_idx] {
                None => {
                    // Empty slot found
                    table[bucket_idx] = current_entry;
                    self.size += 1;
                    return Ok(());
                }
                Some(existing) => {
                    // Kick out existing entry
                    let old_entry = std::mem::replace(existing, current_entry.unwrap());
                    current_entry = Some(old_entry);
                    use_table1 = !use_table1; // Switch tables
                    relocations += 1;
                }
            }
        }

        // Too many relocations, need to resize
        self.resize()?;
        self.insert_entry_cuckoo(*current_entry.unwrap())
    }

    /// Alternative hash function for cuckoo hashing
    fn bucket_index_alt(&self, hash: u64) -> usize {
        // Use a different hash function for the second table
        let alt_hash = hash.wrapping_mul(0x9e3779b97f4a7c15u64);
        (alt_hash as usize) & (self.capacity - 1)
    }

    /// Find an entry by key
    fn find_entry(&self, key: &K) -> Option<&HashIndexEntry<K, V>> {
        let hash = HashIndexEntry::<K, V>::calculate_hash(key);
        let bucket_idx = self.bucket_index(hash);

        let mut current = self.buckets[bucket_idx].as_ref()?;
        loop {
            if current.hash == hash && current.key == *key {
                return Some(current);
            }
            current = current.next.as_ref()?;
        }
    }

    /// Find a mutable entry by key
    fn find_entry_mut(&mut self, key: &K) -> Option<&mut HashIndexEntry<K, V>> {
        let hash = HashIndexEntry::<K, V>::calculate_hash(key);
        let bucket_idx = self.bucket_index(hash);

        let mut current = self.buckets[bucket_idx].as_mut()?;
        loop {
            if current.hash == hash && current.key == *key {
                return Some(current);
            }
            current = current.next.as_mut()?;
        }
    }

    /// Remove an entry by key
    fn remove_entry(&mut self, key: &K) -> Option<HashIndexEntry<K, V>> {
        let hash = HashIndexEntry::<K, V>::calculate_hash(key);
        let bucket_idx = self.bucket_index(hash);

        let bucket = &mut self.buckets[bucket_idx];
        if bucket.is_none() {
            return None;
        }

        // Check if the first entry matches
        if let Some(head) = bucket
            && head.hash == hash
            && head.key == *key
        {
            let removed = std::mem::take(bucket);
            if let Some(mut removed_entry) = removed {
                *bucket = removed_entry.next.take();
                self.size -= 1;
                return Some(*removed_entry);
            }
        }

        // Search in the chain
        let mut current = bucket.as_mut()?;
        while let Some(next_entry) = &mut current.next {
            if next_entry.hash == hash && next_entry.key == *key {
                let removed = current.next.take().unwrap();
                current.next = removed.next.clone();
                self.size -= 1;
                return Some(*removed);
            }
            current = current.next.as_mut()?;
        }

        None
    }

    /// Get current load factor
    pub fn load_factor(&self) -> f64 {
        if self.capacity == 0 { 0.0 } else { self.size as f64 / self.capacity as f64 }
    }

    /// Get number of collisions
    pub fn collision_count(&self) -> usize {
        self.collisions
    }

    /// Get maximum chain length
    pub fn max_chain_length(&self) -> usize {
        self.max_chain_length
    }

    /// Get current capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get average chain length
    pub fn average_chain_length(&self) -> f64 {
        if self.buckets.is_empty() {
            return 0.0;
        }

        let mut total_chain_length = 0;
        let mut non_empty_buckets = 0;

        for entry in self.buckets.iter().flatten() {
            total_chain_length += entry.chain_length();
            non_empty_buckets += 1;
        }

        if non_empty_buckets == 0 { 0.0 } else { total_chain_length as f64 / non_empty_buckets as f64 }
    }

    /// Get distribution of chain lengths
    pub fn chain_length_distribution(&self) -> HashMap<usize, usize> {
        let mut distribution = HashMap::new();

        for bucket in &self.buckets {
            let chain_length = if let Some(entry) = bucket { entry.chain_length() } else { 0 };
            *distribution.entry(chain_length).or_insert(0) += 1;
        }

        distribution
    }
}

impl<K, V> Default for HashIndex<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Index<K, V> for HashIndex<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn insert(&mut self, key: K, value: V) -> IndexResult<()> {
        // Check if key already exists
        if self.contains(&key) {
            return Err(IndexError::InvalidOperation("Key already exists".to_string()));
        }

        // Check if resize is needed
        if self.needs_resize() {
            self.resize()?;
        }

        let entry = HashIndexEntry::new(key, value);
        self.insert_entry(entry)
    }

    fn get(&self, key: &K) -> IndexResult<Option<V>> {
        Ok(self.find_entry(key).map(|entry| entry.value.clone()))
    }

    fn update(&mut self, key: K, value: V) -> IndexResult<()> {
        match self.find_entry_mut(&key) {
            Some(entry) => {
                entry.value = value;
                Ok(())
            }
            None => Err(IndexError::KeyNotFound(format!("{key:?}"))),
        }
    }

    fn delete(&mut self, key: &K) -> IndexResult<()> {
        match self.remove_entry(key) {
            Some(_) => Ok(()),
            None => Err(IndexError::KeyNotFound(format!("{key:?}"))),
        }
    }

    fn contains(&self, key: &K) -> bool {
        self.find_entry(key).is_some()
    }

    fn len(&self) -> usize {
        self.size
    }

    fn clear(&mut self) {
        self.buckets = vec![None; self.capacity];
        self.size = 0;
        self.collisions = 0;
        self.max_chain_length = 0;
    }

    fn index_type(&self) -> IndexType {
        IndexType::Hash
    }

    fn keys(&self) -> Vec<K> {
        let mut result = Vec::with_capacity(self.size);

        for bucket in &self.buckets {
            let mut current = bucket.as_ref();
            while let Some(entry) = current {
                result.push(entry.key.clone());
                current = entry.next.as_ref();
            }
        }

        result
    }

    fn values(&self) -> Vec<V> {
        let mut result = Vec::with_capacity(self.size);

        for bucket in &self.buckets {
            let mut current = bucket.as_ref();
            while let Some(entry) = current {
                result.push(entry.value.clone());
                current = entry.next.as_ref();
            }
        }

        result
    }

    fn entries(&self) -> Vec<(K, V)> {
        let mut result = Vec::with_capacity(self.size);

        for bucket in &self.buckets {
            let mut current = bucket.as_ref();
            while let Some(entry) = current {
                result.push((entry.key.clone(), entry.value.clone()));
                current = entry.next.as_ref();
            }
        }

        result
    }
}

impl<K, V> IndexMaintenance for HashIndex<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    fn compact(&mut self) -> IndexResult<()> {
        // For hash index, compaction means optimizing the load factor
        let optimal_capacity = (self.size as f64 / self.load_factor).ceil() as usize;
        let optimal_capacity = optimal_capacity.max(MIN_CAPACITY).next_power_of_two();

        if optimal_capacity != self.capacity {
            let old_capacity = self.capacity;
            self.capacity = optimal_capacity;
            let old_buckets = std::mem::take(&mut self.buckets);

            self.buckets = vec![None; self.capacity];
            self.size = 0;
            self.collisions = 0;
            self.max_chain_length = 0;

            // Rehash all entries
            for bucket in old_buckets {
                let mut current = bucket;
                while let Some(mut entry) = current {
                    let next = entry.next.take();
                    self.insert_entry(*entry)?;
                    current = next;
                }
            }
        }

        Ok(())
    }

    fn verify(&self) -> IndexResult<bool> {
        let mut actual_size = 0;
        let mut actual_collisions = 0;
        let mut actual_max_chain = 0;

        for bucket in &self.buckets {
            if let Some(entry) = bucket {
                let chain_length = entry.chain_length();
                actual_size += chain_length;
                if chain_length > 1 {
                    actual_collisions += chain_length - 1;
                }
                actual_max_chain = actual_max_chain.max(chain_length);

                // Verify hash consistency
                let mut current = entry.as_ref();
                loop {
                    let expected_hash = HashIndexEntry::<K, V>::calculate_hash(&current.key);
                    if current.hash != expected_hash {
                        return Ok(false);
                    }

                    if let Some(next) = &current.next {
                        current = next.as_ref();
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(actual_size == self.size && actual_collisions <= self.collisions && actual_max_chain == self.max_chain_length)
    }

    fn stats(&self) -> IndexStats {
        let mut stats = IndexStats::new(IndexType::Hash);
        stats.entry_count = self.size;
        stats.size_bytes = self.capacity * std::mem::size_of::<Option<Box<HashIndexEntry<K, V>>>>() + self.size * std::mem::size_of::<HashIndexEntry<K, V>>();

        if self.size > 0 {
            let keys = self.keys();
            let values = self.values();

            stats.avg_key_size = keys.iter().map(|k| k.size() as f64).sum::<f64>() / keys.len() as f64;

            stats.avg_value_size = values.iter().map(|v| v.to_bytes().len() as f64).sum::<f64>() / values.len() as f64;
        }

        stats.type_specific.insert("capacity".to_string(), self.capacity.to_string());
        stats.type_specific.insert("load_factor".to_string(), self.load_factor().to_string());
        stats.type_specific.insert("collisions".to_string(), self.collisions.to_string());
        stats.type_specific.insert("max_chain_length".to_string(), self.max_chain_length.to_string());
        stats.type_specific.insert("avg_chain_length".to_string(), self.average_chain_length().to_string());

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

impl<K, V> IndexPersistence<K, V> for HashIndex<K, V>
where
    K: IndexKey,
    V: IndexValue + 'static,
{
    fn serialize(&self) -> IndexResult<Vec<u8>> {
        let mut data = Vec::new();

        // Write header
        data.extend_from_slice(&self.capacity.to_le_bytes());
        data.extend_from_slice(&self.load_factor.to_le_bytes());
        data.extend_from_slice(&self.size.to_le_bytes());

        // Write all entries
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
        if data.len() < 32 {
            return Err(IndexError::SerializationError("Insufficient data for header".to_string()));
        }

        let mut offset = 0;

        // Read header
        let capacity = usize::from_le_bytes([
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

        let load_factor = f64::from_le_bytes([
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

        // Reconstruct the hash index
        *self = Self::with_load_factor(capacity, load_factor);

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

            self.insert_entry(HashIndexEntry::new(key, value))?;
        }

        Ok(())
    }

    fn save_to_disk<P: AsRef<std::path::Path>>(&self, path: P) -> IndexResult<()> {
        let data = self.serialize()?;
        std::fs::write(path, data).map_err(|e| IndexError::IoError(format!("Failed to write to disk: {e}")))
    }

    fn load_from_disk<P: AsRef<std::path::Path>>(&mut self, path: P) -> IndexResult<()> {
        let data = std::fs::read(path).map_err(|e| IndexError::IoError(format!("Failed to read from disk: {e}")))?;
        self.deserialize(&data)
    }

    fn format_version(&self) -> u32 {
        1
    }

    fn supports_incremental_save(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_index_creation() {
        let index: HashIndex<i32, String> = HashIndex::new();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        assert_eq!(index.index_type(), IndexType::Hash);
        assert!(index.capacity() >= DEFAULT_INITIAL_CAPACITY);
    }

    #[test]
    fn test_hash_index_insert_and_get() {
        let mut index = HashIndex::new();

        index.insert(1, "one".to_string()).unwrap();
        index.insert(2, "two".to_string()).unwrap();
        index.insert(3, "three".to_string()).unwrap();

        assert_eq!(index.len(), 3);
        assert_eq!(index.get(&1).unwrap(), Some("one".to_string()));
        assert_eq!(index.get(&2).unwrap(), Some("two".to_string()));
        assert_eq!(index.get(&3).unwrap(), Some("three".to_string()));
        assert_eq!(index.get(&4).unwrap(), None);
    }

    #[test]
    fn test_hash_index_duplicate_insert() {
        let mut index = HashIndex::new();

        index.insert(1, "one".to_string()).unwrap();
        let result = index.insert(1, "another_one".to_string());

        assert!(result.is_err());
        assert_eq!(index.get(&1).unwrap(), Some("one".to_string()));
    }

    #[test]
    fn test_hash_index_update() {
        let mut index = HashIndex::new();

        index.insert(1, "one".to_string()).unwrap();
        assert_eq!(index.get(&1).unwrap(), Some("one".to_string()));

        index.update(1, "ONE".to_string()).unwrap();
        assert_eq!(index.get(&1).unwrap(), Some("ONE".to_string()));

        assert!(index.update(999, "does_not_exist".to_string()).is_err());
    }

    #[test]
    fn test_hash_index_delete() {
        let mut index = HashIndex::new();

        index.insert(1, "one".to_string()).unwrap();
        index.insert(2, "two".to_string()).unwrap();

        assert!(index.contains(&1));
        index.delete(&1).unwrap();
        assert!(!index.contains(&1));
        assert_eq!(index.len(), 1);

        assert!(index.delete(&999).is_err());
    }

    #[test]
    fn test_hash_index_collision_handling() {
        let mut index = HashIndex::with_capacity(4); // Small capacity to force collisions

        // Insert enough items to cause collisions
        for i in 0..20 {
            index.insert(i, format!("value_{}", i)).unwrap();
        }

        assert_eq!(index.len(), 20);

        // Verify all items can be retrieved
        for i in 0..20 {
            assert_eq!(index.get(&i).unwrap(), Some(format!("value_{}", i)));
        }

        assert!(index.collision_count() > 0);
        assert!(index.max_chain_length() > 1);
    }

    #[test]
    fn test_hash_index_resize() {
        let mut index = HashIndex::with_capacity(4);
        let initial_capacity = index.capacity();

        // Insert enough items to trigger resize
        for i in 0..20 {
            index.insert(i, format!("value_{}", i)).unwrap();
        }

        assert!(index.capacity() > initial_capacity);
        assert_eq!(index.len(), 20);

        // Verify all items are still accessible after resize
        for i in 0..20 {
            assert_eq!(index.get(&i).unwrap(), Some(format!("value_{}", i)));
        }
    }

    #[test]
    fn test_hash_index_clear() {
        let mut index = HashIndex::new();

        index.insert(1, "one".to_string()).unwrap();
        index.insert(2, "two".to_string()).unwrap();

        assert_eq!(index.len(), 2);
        index.clear();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        assert_eq!(index.collision_count(), 0);
    }

    #[test]
    fn test_hash_index_load_factor() {
        let index: HashIndex<i32, String> = HashIndex::with_load_factor(16, 0.5);
        assert_eq!(index.load_factor, 0.5);

        let index: HashIndex<i32, String> = HashIndex::with_load_factor(16, 2.0); // Should be clamped
        assert_eq!(index.load_factor, 1.0);

        let index: HashIndex<i32, String> = HashIndex::with_load_factor(16, -1.0); // Should be clamped
        assert_eq!(index.load_factor, 0.1);
    }

    #[test]
    fn test_hash_index_keys_and_values() {
        let mut index = HashIndex::new();

        index.insert(1, "one".to_string()).unwrap();
        index.insert(2, "two".to_string()).unwrap();
        index.insert(3, "three".to_string()).unwrap();

        let keys = index.keys();
        let values = index.values();

        assert_eq!(keys.len(), 3);
        assert_eq!(values.len(), 3);

        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
        assert!(keys.contains(&3));

        assert!(values.contains(&"one".to_string()));
        assert!(values.contains(&"two".to_string()));
        assert!(values.contains(&"three".to_string()));
    }

    #[test]
    fn test_hash_index_entry_chain_length() {
        let entry1 = HashIndexEntry::new(1, "one".to_string());
        assert_eq!(entry1.chain_length(), 1);

        let mut entry2 = HashIndexEntry::new(2, "two".to_string());
        entry2.next = Some(Box::new(entry1));
        assert_eq!(entry2.chain_length(), 2);
    }

    #[test]
    fn test_hash_index_statistics() {
        let mut index = HashIndex::new();

        for i in 0..100 {
            index.insert(i, format!("value_{}", i)).unwrap();
        }

        let stats = index.stats();
        assert_eq!(stats.entry_count, 100);
        assert_eq!(stats.index_type, IndexType::Hash);
        assert!(stats.type_specific.contains_key("capacity"));
        assert!(stats.type_specific.contains_key("load_factor"));
        assert!(stats.type_specific.contains_key("collisions"));
    }

    #[test]
    fn test_hash_index_verify() {
        let mut index = HashIndex::new();

        for i in 0..50 {
            index.insert(i, format!("value_{}", i)).unwrap();
        }

        assert!(index.verify().unwrap());
    }

    #[test]
    fn test_hash_index_compact() {
        let mut index = HashIndex::with_capacity(1024); // Large initial capacity

        // Insert only a few items
        for i in 0..10 {
            index.insert(i, format!("value_{}", i)).unwrap();
        }

        let original_capacity = index.capacity();
        index.compact().unwrap();

        // Capacity should be reduced
        assert!(index.capacity() <= original_capacity);
        assert_eq!(index.len(), 10);

        // All items should still be accessible
        for i in 0..10 {
            assert_eq!(index.get(&i).unwrap(), Some(format!("value_{}", i)));
        }
    }

    #[test]
    fn test_hash_index_rebuild() {
        let mut index = HashIndex::new();

        for i in 0..20 {
            index.insert(i, format!("value_{}", i)).unwrap();
        }

        index.rebuild().unwrap();
        assert_eq!(index.len(), 20);

        // All items should still be accessible
        for i in 0..20 {
            assert_eq!(index.get(&i).unwrap(), Some(format!("value_{}", i)));
        }
    }

    #[test]
    fn test_hash_index_chain_length_distribution() {
        let mut index = HashIndex::with_capacity(4); // Small capacity to force collisions

        for i in 0..16 {
            index.insert(i, format!("value_{}", i)).unwrap();
        }

        let distribution = index.chain_length_distribution();

        // Should have various chain lengths
        assert!(distribution.len() > 1);

        // Sum of all chain lengths should equal the capacity
        let total_buckets: usize = distribution.values().sum();
        assert_eq!(total_buckets, index.capacity());
    }
}
