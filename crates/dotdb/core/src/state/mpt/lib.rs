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

//! Merkle Patricia Trie (MPT) core library
//!
//! This module provides the core functionality for implementing a Merkle Patricia Trie,
//! which is a cryptographically authenticated data structure used to store key-value pairs.
//! The implementation includes efficient path compression, secure hashing, and proof generation.
//!
//! # Key Features
//!
//! - Binary Merkle Patricia Trie implementation
//! - Keccak-256 hashing for cryptographic security
//! - Path compression for efficient storage
//! - Compact encoding for node paths
//! - Type-safe error handling
//!
//! # Performance Considerations
//!
//! - Uses pre-allocated vectors for nibble conversion
//! - Implements efficient path compression
//! - Optimized common prefix calculation
//! - Zero-copy operations where possible

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use thiserror::Error;

/// 32-byte hash used throughout the MPT
///
/// This type represents a cryptographic hash (Keccak-256) used to identify nodes
/// and ensure data integrity in the trie.
pub type Hash = [u8; 32];

/// Key type for the trie
///
/// Keys are stored as byte vectors, allowing for flexible key types.
/// The actual key format is determined by the application using the trie.
pub type Key = Vec<u8>;

/// Value type for the trie
///
/// Values are stored as byte vectors, allowing for arbitrary data storage.
/// The actual value format is determined by the application using the trie.
pub type Value = Vec<u8>;

/// Node identifier (hash of the node)
///
/// Each node in the trie is identified by its Keccak-256 hash.
/// This provides a unique identifier and cryptographic verification.
pub type NodeId = Hash;

/// Result type for trie operations
///
/// A type alias for Result that uses MPTError as the error type.
/// This provides consistent error handling across all trie operations.
pub type TrieResult<T> = Result<T, MPTError>;

/// Errors that can occur in MPT operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MPTError {
    /// Node with the specified ID was not found in storage
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),

    /// Key was not found in the trie
    #[error("Key not found: {0:?}")]
    KeyNotFound(Key),

    /// Invalid or corrupted proof
    #[error("Invalid proof")]
    InvalidProof,

    /// Error during serialization/deserialization
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Error during storage operations
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Invalid node type encountered
    #[error("Invalid node type")]
    InvalidNodeType,

    /// Error during path traversal
    #[error("Path traversal error")]
    PathTraversalError,
}

/// Calculate Keccak-256 hash of the input data
///
/// # Arguments
///
/// * `data` - The input data to hash
///
/// # Returns
///
/// A 32-byte hash of the input data
///
/// # Performance
///
/// This function uses the SHA-3 Keccak-256 algorithm, which is optimized for
/// cryptographic security and performance.
pub fn keccak256(data: &[u8]) -> Hash {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Convert a key to nibbles (4-bit values)
///
/// # Arguments
///
/// * `key` - The input key to convert
///
/// # Returns
///
/// A vector of nibbles (4-bit values) representing the key
///
/// # Performance
///
/// This function pre-allocates the output vector to avoid reallocations
/// and processes the input in a single pass.
pub fn key_to_nibbles(key: &[u8]) -> Vec<u8> {
    let mut nibbles = Vec::with_capacity(key.len() * 2);
    for byte in key {
        nibbles.push(byte >> 4);
        nibbles.push(byte & 0x0f);
    }
    nibbles
}

/// Convert nibbles back to bytes
///
/// # Arguments
///
/// * `nibbles` - The input nibbles to convert
///
/// # Returns
///
/// A vector of bytes representing the original key
///
/// # Performance
///
/// This function pre-allocates the output vector and processes the input
/// in chunks of 2 nibbles for efficiency.
pub fn nibbles_to_key(nibbles: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(nibbles.len().div_ceil(2));
    for pair in nibbles.chunks(2) {
        if pair.len() == 2 {
            key.push((pair[0] << 4) | pair[1]);
        } else {
            key.push(pair[0] << 4);
        }
    }
    key
}

/// Find common prefix between two nibble slices
///
/// # Arguments
///
/// * `a` - First nibble slice
/// * `b` - Second nibble slice
///
/// # Returns
///
/// The length of the common prefix
///
/// # Performance
///
/// This function uses a single pass comparison and avoids unnecessary
/// allocations or copies.
pub fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    let mut i = 0;
    while i < a.len().min(b.len()) && a[i] == b[i] {
        i += 1;
    }
    i
}

/// Compact encoding for path compression in the trie
///
/// This structure implements the compact encoding scheme used in Ethereum's
/// Merkle Patricia Trie for efficient path storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactPath {
    /// The nibbles representing the path
    pub nibbles: Vec<u8>,
    /// Whether this path leads to a leaf node
    pub is_leaf: bool,
}

impl CompactPath {
    /// Create a new compact path
    ///
    /// # Arguments
    ///
    /// * `nibbles` - The nibbles representing the path
    /// * `is_leaf` - Whether this path leads to a leaf node
    pub fn new(nibbles: Vec<u8>, is_leaf: bool) -> Self {
        Self { nibbles, is_leaf }
    }

    /// Encode the path into a compact byte representation
    ///
    /// # Returns
    ///
    /// A vector of bytes representing the compact encoding of the path
    ///
    /// # Performance
    ///
    /// This function pre-allocates the output vector and processes the nibbles
    /// in chunks for efficiency.
    pub fn encode(&self) -> Vec<u8> {
        if self.nibbles.is_empty() {
            return vec![if self.is_leaf { 0x20 } else { 0x00 }];
        }

        let mut encoded = Vec::with_capacity(self.nibbles.len() / 2 + 1);
        let odd_len = self.nibbles.len() % 2 == 1;
        let mut flags = if self.is_leaf { 0x20 } else { 0x00 };

        if odd_len {
            flags |= 0x10;
            flags |= self.nibbles[0];
            encoded.push(flags);

            for pair in self.nibbles[1..].chunks(2) {
                if pair.len() == 2 {
                    encoded.push((pair[0] << 4) | pair[1]);
                }
            }
        } else {
            encoded.push(flags);
            for pair in self.nibbles.chunks(2) {
                if pair.len() == 2 {
                    encoded.push((pair[0] << 4) | pair[1]);
                }
            }
        }

        encoded
    }

    /// Decode a compact byte representation into a path
    ///
    /// # Arguments
    ///
    /// * `data` - The compact byte representation to decode
    ///
    /// # Returns
    ///
    /// A Result containing either the decoded path or an error
    ///
    /// # Performance
    ///
    /// This function pre-allocates the output vector and processes the input
    /// in a single pass.
    pub fn decode(data: &[u8]) -> TrieResult<Self> {
        if data.is_empty() {
            return Ok(CompactPath::new(vec![], false));
        }

        let flags = data[0];
        let is_leaf = (flags & 0x20) != 0;
        let odd_len = (flags & 0x10) != 0;

        let mut nibbles = Vec::with_capacity(data.len() * 2);

        if odd_len {
            nibbles.push(flags & 0x0f);
            for &byte in &data[1..] {
                nibbles.push(byte >> 4);
                nibbles.push(byte & 0x0f);
            }
        } else {
            for &byte in &data[1..] {
                nibbles.push(byte >> 4);
                nibbles.push(byte & 0x0f);
            }
        }

        Ok(CompactPath::new(nibbles, is_leaf))
    }
}
