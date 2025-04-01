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

//! Common types and utilities for state management

use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// General error type for state management operations
#[derive(Debug)]
pub enum Error {
    /// Error related to version conflicts in MVCC
    VersionConflict,
    /// Error when an item is not found
    NotFound,
    /// Error during Merkle tree operations
    MerkleError(String),
    /// Error during snapshot operations
    SnapshotError(String),
    /// Error during validation
    ValidationError(String),
    /// General error with a message
    Other(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::VersionConflict => write!(f, "Version conflict detected"),
            Error::NotFound => write!(f, "Item not found"),
            Error::MerkleError(msg) => write!(f, "Merkle tree error: {}", msg),
            Error::SnapshotError(msg) => write!(f, "Snapshot error: {}", msg),
            Error::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type using our custom Error
pub type Result<T> = std::result::Result<T, Error>;

/// Generic key type for state entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StateKey(pub Vec<u8>);

/// Core types for state management system.
/// Defines cryptographic primitives and thread-safe structures.
impl StateKey {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    /// Creates key from string with:
    /// - **Byte Conversion**: UTF-8 encoding
    /// - **Ordinal Sorting**: Enables BTreeMap ordering
    /// - **Hash Compatibility**: Works with Merkle hashing
    pub fn from_string(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Generic value type for state entries
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateValue(pub Vec<u8>);

impl StateValue {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn from_string(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Returns the current timestamp in milliseconds
pub fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Thread-safe generic map type used across the state management system
pub type SharedMap<K, V> = Arc<RwLock<HashMap<K, V>>>;

/// Create a new thread-safe map
pub fn new_shared_map<K, V>() -> SharedMap<K, V>
where
    K: Eq + Hash,
{
    Arc::new(RwLock::new(HashMap::new()))
}

/// Simple hash function that can be used for testing
#[cfg(test)]
pub fn test_hash(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_key() {
        let key1 = StateKey::from_string("test_key");
        let key2 = StateKey::new(b"test_key".to_vec());

        assert_eq!(key1, key2);
        assert_eq!(key1.as_bytes(), b"test_key");
    }

    #[test]
    fn test_state_value() {
        let value1 = StateValue::from_string("test_value");
        let value2 = StateValue::new(b"test_value".to_vec());

        assert_eq!(value1, value2);
        assert_eq!(value1.as_bytes(), b"test_value");
    }

    #[test]
    fn test_shared_map() {
        let map = new_shared_map::<String, i32>();

        // Write to the map
        {
            let mut write_guard = map.write().unwrap();
            write_guard.insert("key1".to_string(), 42);
            write_guard.insert("key2".to_string(), 100);
        }

        // Read from the map
        {
            let read_guard = map.read().unwrap();
            assert_eq!(*read_guard.get("key1").unwrap(), 42);
            assert_eq!(*read_guard.get("key2").unwrap(), 100);
        }
    }

    #[test]
    fn test_error_display() {
        let err1 = Error::NotFound;
        let err2 = Error::MerkleError("invalid node".to_string());

        assert_eq!(err1.to_string(), "Item not found");
        assert_eq!(err2.to_string(), "Merkle tree error: invalid node");
    }
}
