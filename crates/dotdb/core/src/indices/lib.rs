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

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

/// Result type for index operations
pub type IndexResult<T> = Result<T, IndexError>;

/// Index key trait that all index keys must implement
pub trait IndexKey: Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Hash + Send + Sync {
    /// Serialize the key to bytes
    fn to_bytes(&self) -> Vec<u8>;

    /// Deserialize the key from bytes
    fn from_bytes(bytes: &[u8]) -> IndexResult<Self>
    where
        Self: Sized;

    /// Get the size in bytes
    fn size(&self) -> usize {
        self.to_bytes().len()
    }
}

/// Index value that can be stored in indices
pub trait IndexValue: Clone + Debug + Send + Sync {
    /// Serialize the value to bytes
    fn to_bytes(&self) -> Vec<u8>;

    /// Deserialize the value from bytes
    fn from_bytes(bytes: &[u8]) -> IndexResult<Self>
    where
        Self: Sized;
}

/// Types of indices available
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexType {
    /// B+ tree index for range queries and sorted access
    BPlusTree,
    /// Hash index for fast equality lookups
    Hash,
    /// Composite index over multiple fields
    Composite(Vec<String>),
}

/// Errors that can occur during index operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexError {
    /// Key not found in index
    KeyNotFound(String),
    /// Index is full (for fixed-size indices)
    IndexFull,
    /// Serialization/deserialization error
    SerializationError(String),
    /// Invalid operation for this index type
    InvalidOperation(String),
    /// IO error during index operations
    IoError(String),
    /// Index corruption detected
    Corruption(String),
    /// Invalid key format
    InvalidKey(String),
    /// Index already exists
    IndexExists(String),
    /// Memory allocation error
    OutOfMemory,
}

impl Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::KeyNotFound(key) => write!(f, "Key not found: {key}"),
            IndexError::IndexFull => write!(f, "Index is full"),
            IndexError::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            IndexError::InvalidOperation(msg) => write!(f, "Invalid operation: {msg}"),
            IndexError::IoError(msg) => write!(f, "IO error: {msg}"),
            IndexError::Corruption(msg) => write!(f, "Index corruption: {msg}"),
            IndexError::InvalidKey(msg) => write!(f, "Invalid key: {msg}"),
            IndexError::IndexExists(name) => write!(f, "Index already exists: {name}"),
            IndexError::OutOfMemory => write!(f, "Out of memory"),
        }
    }
}

impl std::error::Error for IndexError {}

/// Operations that can be performed on indices
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexOperation<K, V> {
    /// Insert a key-value pair
    Insert(K, V),
    /// Update an existing key with new value
    Update(K, V),
    /// Delete a key
    Delete(K),
    /// Get value by key
    Get(K),
}

/// Common trait for all index implementations
pub trait Index<K, V>: Send + Sync
where
    K: IndexKey,
    V: IndexValue,
{
    /// Insert a key-value pair into the index
    fn insert(&mut self, key: K, value: V) -> IndexResult<()>;

    /// Get a value by key from the index
    fn get(&self, key: &K) -> IndexResult<Option<V>>;

    /// Update an existing key with a new value
    fn update(&mut self, key: K, value: V) -> IndexResult<()>;

    /// Delete a key from the index
    fn delete(&mut self, key: &K) -> IndexResult<()>;

    /// Check if a key exists in the index
    fn contains(&self, key: &K) -> bool;

    /// Get the number of entries in the index
    fn len(&self) -> usize;

    /// Check if the index is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries from the index
    fn clear(&mut self);

    /// Get the type of this index
    fn index_type(&self) -> IndexType;

    /// Get all keys in the index (order depends on index type)
    fn keys(&self) -> Vec<K>;

    /// Get all values in the index
    fn values(&self) -> Vec<V>;

    /// Get all key-value pairs
    fn entries(&self) -> Vec<(K, V)>;
}

/// Iterator trait for index scanning
pub trait IndexIterator<K, V>: Iterator<Item = (K, V)>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Seek to a specific key
    fn seek(&mut self, key: &K);

    /// Seek to the first key greater than or equal to the given key
    fn seek_to_first(&mut self);

    /// Seek to the last key
    fn seek_to_last(&mut self);

    /// Check if the iterator is valid
    fn valid(&self) -> bool;
}

/// Range query support for ordered indices
pub trait RangeQuery<K, V>
where
    K: IndexKey,
    V: IndexValue,
{
    /// Get all key-value pairs in the given range [start, end]
    fn range(&self, start: &K, end: &K) -> IndexResult<Vec<(K, V)>>;

    /// Get all key-value pairs with keys greater than the given key
    fn range_from(&self, start: &K) -> IndexResult<Vec<(K, V)>>;

    /// Get all key-value pairs with keys less than the given key
    fn range_to(&self, end: &K) -> IndexResult<Vec<(K, V)>>;

    /// Create an iterator for range queries
    fn range_iter(&self, start: &K, end: &K) -> Box<dyn IndexIterator<K, V>>;
}

/// Composite key for multi-field indices
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompositeKey {
    fields: Vec<Vec<u8>>,
}

impl CompositeKey {
    /// Create a new composite key from field values
    pub fn new(fields: Vec<Vec<u8>>) -> Self {
        Self { fields }
    }

    /// Add a field to the composite key
    pub fn add_field(&mut self, field: Vec<u8>) {
        self.fields.push(field);
    }

    /// Get the number of fields in the key
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Get a specific field by index
    pub fn get_field(&self, index: usize) -> Option<&Vec<u8>> {
        if index < self.fields.len() { Some(&self.fields[index]) } else { None }
    }

    /// Get all fields
    pub fn fields(&self) -> &[Vec<u8>] {
        &self.fields
    }
}

impl IndexKey for CompositeKey {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Write number of fields
        bytes.extend_from_slice(&(self.fields.len() as u32).to_le_bytes());

        // Write each field with its length
        for field in &self.fields {
            bytes.extend_from_slice(&(field.len() as u32).to_le_bytes());
            bytes.extend_from_slice(field);
        }

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> IndexResult<Self> {
        if bytes.len() < 4 {
            return Err(IndexError::SerializationError("Invalid composite key format".to_string()));
        }

        let mut offset = 0;
        let field_count = u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]) as usize;
        offset += 4;

        let mut fields = Vec::with_capacity(field_count);

        for _ in 0..field_count {
            if offset + 4 > bytes.len() {
                return Err(IndexError::SerializationError("Invalid composite key format".to_string()));
            }

            let field_len = u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]) as usize;
            offset += 4;

            if offset + field_len > bytes.len() {
                return Err(IndexError::SerializationError("Invalid composite key format".to_string()));
            }

            let field = bytes[offset..offset + field_len].to_vec();
            fields.push(field);
            offset += field_len;
        }

        Ok(CompositeKey { fields })
    }
}

/// Helper function to create a composite key from typed values
pub fn create_composite_key<T: IndexKey>(values: &[T]) -> IndexResult<CompositeKey> {
    let fields: Result<Vec<_>, _> = values.iter().map(|v| Ok(v.to_bytes())).collect();
    Ok(CompositeKey::new(fields?))
}

/// Index maintenance operations
pub trait IndexMaintenance {
    /// Compact the index to optimize storage
    fn compact(&mut self) -> IndexResult<()>;

    /// Verify index integrity
    fn verify(&self) -> IndexResult<bool>;

    /// Get index statistics
    fn stats(&self) -> IndexStats;

    /// Rebuild the index from scratch
    fn rebuild(&mut self) -> IndexResult<()>;
}

/// Statistics about an index
#[derive(Debug, Clone, PartialEq)]
pub struct IndexStats {
    /// Number of entries in the index
    pub entry_count: usize,
    /// Total size in bytes
    pub size_bytes: usize,
    /// Average key size
    pub avg_key_size: f64,
    /// Average value size
    pub avg_value_size: f64,
    /// Index type
    pub index_type: IndexType,
    /// Additional type-specific statistics
    pub type_specific: HashMap<String, String>,
}

impl IndexStats {
    pub fn new(index_type: IndexType) -> Self {
        Self {
            entry_count: 0,
            size_bytes: 0,
            avg_key_size: 0.0,
            avg_value_size: 0.0,
            index_type,
            type_specific: HashMap::new(),
        }
    }
}

// Implementations for common types
impl IndexKey for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> IndexResult<Self> {
        String::from_utf8(bytes.to_vec()).map_err(|e| IndexError::SerializationError(format!("UTF-8 error: {e}")))
    }
}

impl IndexKey for u64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> IndexResult<Self> {
        if bytes.len() != 8 {
            return Err(IndexError::SerializationError("Invalid u64 format".to_string()));
        }

        let mut array = [0u8; 8];
        array.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(array))
    }
}

impl IndexKey for i32 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> IndexResult<Self> {
        if bytes.len() != 4 {
            return Err(IndexError::SerializationError("Invalid i32 format".to_string()));
        }

        let mut array = [0u8; 4];
        array.copy_from_slice(bytes);
        Ok(i32::from_le_bytes(array))
    }
}

impl IndexKey for i64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> IndexResult<Self> {
        if bytes.len() != 8 {
            return Err(IndexError::SerializationError("Invalid i64 format".to_string()));
        }

        let mut array = [0u8; 8];
        array.copy_from_slice(bytes);
        Ok(i64::from_le_bytes(array))
    }
}

impl IndexValue for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> IndexResult<Self> {
        String::from_utf8(bytes.to_vec()).map_err(|e| IndexError::SerializationError(format!("UTF-8 error: {e}")))
    }
}

impl IndexValue for Vec<u8> {
    fn to_bytes(&self) -> Vec<u8> {
        self.clone()
    }

    fn from_bytes(bytes: &[u8]) -> IndexResult<Self> {
        Ok(bytes.to_vec())
    }
}
