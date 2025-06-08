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

//! Merkle Patricia Trie (MPT) Module
//!
//! This module provides a complete implementation of a Merkle Patricia Trie,
//! a cryptographically authenticated data structure used for efficient storage
//! and verification of key-value pairs.
//!
//! # Module Structure
//!
//! - `lib`: Core types, utilities, and error definitions
//! - `node`: Trie node implementations and node type definitions
//! - `proof`: Merkle proof generation and verification
//! - `trie`: Main trie implementation and operations
//!
//! # Usage
//!
//! ```rust
//! use dotdb_core::state::mpt::{MerklePatriciaTrie, Key, Value};
//!
//! // Create a new trie
//! let mut trie = MerklePatriciaTrie::new_in_memory();
//!
//! // Insert key-value pairs
//! trie.put(Key::from("key1"), Value::from("value1")).unwrap();
//!
//! // Retrieve values
//! let value = trie.get(&Key::from("key1")).unwrap();
//! assert_eq!(value, Some(Value::from("value1")));
//!
//! // Generate proofs
//! let proof = trie.get_proof(&Key::from("key1")).unwrap();
//! assert!(proof.verify().is_ok());
//! ```
//!
//! # Performance Considerations
//!
//! - The trie implementation uses efficient path compression
//! - Node storage is optimized for minimal memory usage
//! - Proof generation is optimized for verification speed
//! - All operations are designed to minimize allocations

/// Core types and utilities for the MPT implementation
pub mod lib;

/// Trie node implementations and node type definitions
pub mod node;

/// Merkle proof generation and verification
pub mod proof;

/// Main trie implementation and operations
pub mod trie;

// Re-export commonly used types for convenience
pub use lib::{Hash, Key, NodeId, TrieResult, Value};
pub use node::{Node, NodeType};
pub use proof::StateProof;
pub use trie::MerklePatriciaTrie;

/// Main error type for MPT operations
///
/// This is a re-export of the error type defined in the lib module
/// for convenience and consistency.
pub type MPTError = lib::MPTError;
