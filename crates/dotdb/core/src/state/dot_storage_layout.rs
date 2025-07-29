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

//! # Contract Storage Layout
//!
//! This module defines a deterministic scheme for mapping a contract's internal storage
//! variables to keys within the global state tree (Merkle Patricia Trie).
//!
//! ## Key Design Principles
//!
//! 1. **Deterministic**: The same storage variable always maps to the same key
//! 2. **Collision-free**: Different storage variables map to different keys
//! 3. **Efficient**: Common access patterns are optimized
//! 4. **Extensible**: Can handle various data types and structures
//!
//! ## Storage Layout Structure
//!
//! Each contract has a dedicated subtree within the MPT, using the contract address
//! as a prefix. The layout supports:
//!
//! - Simple storage slots (256-bit aligned)
//! - Dynamic arrays with automatic slot allocation
//! - Mappings with keccak256-based key derivation
//! - Structs with packed storage optimization
//!
//! ## Key Encoding Format
//!
//! ```text
//! Contract Storage Key = dot_address || storage_slot_key
//!
//! Where:
//! - dot_address: 20 bytes (160 bits)
//! - storage_slot_key: 32 bytes (256 bits)
//! ```

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

use super::mpt::{Key, Value};

/// Contract address type (20 bytes)
pub type DotAddress = [u8; 20];

/// Storage slot identifier (32 bytes)
pub type StorageSlot = [u8; 32];

/// Storage value type that can hold various data types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageValue {
    /// Raw bytes for any data type
    Bytes(Vec<u8>),
    /// 256-bit unsigned integer
    U256([u8; 32]),
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
    /// Array of storage values
    Array(Vec<StorageValue>),
    /// Mapping structure
    Mapping(HashMap<String, StorageValue>),
}

/// Storage variable types supported by the contract storage layout
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageVariableType {
    /// Simple value stored in a single slot
    Simple,
    /// Dynamic array with length stored separately
    DynamicArray,
    /// Mapping with keccak256-based key derivation
    Mapping { key_type: String, value_type: String },
    /// Struct with multiple fields
    Struct { fields: Vec<StructField> },
}

/// Structure field definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub field_type: String,
    pub offset: u32,
    pub size: u32,
}

/// Contract storage layout definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotStorageLayout {
    /// Contract address this layout belongs to
    pub dot_address: DotAddress,
    /// Storage variables and their slot assignments
    pub variables: HashMap<String, StorageVariable>,
    /// Next available slot for new variables
    pub next_slot: u32,
    /// Storage optimization enabled
    pub packed_storage: bool,
}

/// Storage variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageVariable {
    /// Variable name
    pub name: String,
    /// Base storage slot
    pub base_slot: u32,
    /// Variable type information
    pub var_type: StorageVariableType,
    /// Size in slots
    pub slot_count: u32,
}

/// Errors that can occur during storage layout operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageLayoutError {
    /// Invalid contract address
    InvalidAddress,
    /// Storage slot collision
    SlotCollision,
    /// Invalid variable type
    InvalidVariableType,
    /// Encoding error
    EncodingError(String),
    /// Key generation error
    KeyGenerationError(String),
    /// Value serialization error
    SerializationError(String),
}

impl fmt::Display for StorageLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageLayoutError::InvalidAddress => write!(f, "Invalid contract address"),
            StorageLayoutError::SlotCollision => write!(f, "Storage slot collision detected"),
            StorageLayoutError::InvalidVariableType => write!(f, "Invalid variable type"),
            StorageLayoutError::EncodingError(msg) => write!(f, "Encoding error: {msg}"),
            StorageLayoutError::KeyGenerationError(msg) => write!(f, "Key generation error: {msg}"),
            StorageLayoutError::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
        }
    }
}

impl std::error::Error for StorageLayoutError {}

impl DotStorageLayout {
    /// Create a new storage layout for a contract
    pub fn new(dot_address: DotAddress) -> Self {
        Self {
            dot_address,
            variables: HashMap::new(),
            next_slot: 0,
            packed_storage: true,
        }
    }

    /// Add a storage variable to the layout
    pub fn add_variable(&mut self, name: String, var_type: StorageVariableType) -> Result<u32, StorageLayoutError> {
        let slot_count = self.calculate_slot_count(&var_type);
        let base_slot = self.next_slot;

        let variable = StorageVariable {
            name: name.clone(),
            base_slot,
            var_type,
            slot_count,
        };

        self.variables.insert(name, variable);
        self.next_slot += slot_count;

        Ok(base_slot)
    }

    /// Generate MPT key for a storage slot
    pub fn generate_storage_key(&self, slot: u32) -> Result<Key, StorageLayoutError> {
        let mut key_bytes = Vec::with_capacity(52); // 20 + 32 bytes

        // Add contract address prefix
        key_bytes.extend_from_slice(&self.dot_address);

        // Add slot as 32-byte big-endian
        let slot_bytes = slot.to_be_bytes();
        key_bytes.extend_from_slice(&[0u8; 28]); // Pad to 32 bytes
        key_bytes.extend_from_slice(&slot_bytes);

        Ok(Key::from(key_bytes))
    }

    /// Generate MPT key for a mapping entry
    pub fn generate_mapping_key(&self, base_slot: u32, mapping_key: &[u8]) -> Result<Key, StorageLayoutError> {
        let mut hasher = Keccak256::new();

        // Hash: keccak256(mapping_key || base_slot)
        hasher.update(mapping_key);
        hasher.update(base_slot.to_be_bytes());
        let hash = hasher.finalize();

        let mut key_bytes = Vec::with_capacity(52);
        key_bytes.extend_from_slice(&self.dot_address);
        key_bytes.extend_from_slice(&hash);

        Ok(Key::from(key_bytes))
    }

    /// Generate MPT key for an array element
    pub fn generate_array_key(&self, base_slot: u32, index: u64) -> Result<Key, StorageLayoutError> {
        let mut hasher = Keccak256::new();

        // Hash: keccak256(base_slot) + index
        hasher.update(base_slot.to_be_bytes());
        let base_hash = hasher.finalize();

        // Convert hash to u256 and add index
        let mut slot_number = [0u8; 32];
        slot_number.copy_from_slice(&base_hash);

        // Simple addition for slot calculation (in practice, this should handle overflow)
        let slot_as_u64 = u64::from_be_bytes([
            slot_number[24],
            slot_number[25],
            slot_number[26],
            slot_number[27],
            slot_number[28],
            slot_number[29],
            slot_number[30],
            slot_number[31],
        ]);

        let final_slot = slot_as_u64.wrapping_add(index);
        slot_number[24..32].copy_from_slice(&final_slot.to_be_bytes());

        let mut key_bytes = Vec::with_capacity(52);
        key_bytes.extend_from_slice(&self.dot_address);
        key_bytes.extend_from_slice(&slot_number);

        Ok(Key::from(key_bytes))
    }

    /// Calculate the number of slots needed for a variable type
    fn calculate_slot_count(&self, var_type: &StorageVariableType) -> u32 {
        match var_type {
            StorageVariableType::Simple => 1,
            StorageVariableType::DynamicArray => 1,   // Only stores length, elements are stored separately
            StorageVariableType::Mapping { .. } => 1, // Only marker slot
            StorageVariableType::Struct { fields } => {
                if self.packed_storage {
                    // Calculate packed size
                    let total_size: u32 = fields.iter().map(|f| f.size).sum();
                    total_size.div_ceil(32) // Round up to nearest slot
                } else {
                    fields.len() as u32 // One slot per field
                }
            }
        }
    }

    /// Get storage variable information by name
    pub fn get_variable(&self, name: &str) -> Option<&StorageVariable> {
        self.variables.get(name)
    }

    /// Get all variables in the layout
    pub fn get_variables(&self) -> &HashMap<String, StorageVariable> {
        &self.variables
    }
}

impl StorageValue {
    /// Encode storage value to bytes for MPT storage
    pub fn encode(&self) -> Result<Value, StorageLayoutError> {
        let bytes = match self {
            StorageValue::Bytes(data) => data.clone(),
            StorageValue::U256(data) => data.to_vec(),
            StorageValue::Bool(value) => {
                let mut bytes = vec![0u8; 32];
                if *value {
                    bytes[31] = 1;
                }
                bytes
            }
            StorageValue::String(s) => {
                let mut bytes = s.as_bytes().to_vec();
                // Pad to 32 bytes if needed
                if bytes.len() < 32 {
                    bytes.resize(32, 0);
                }
                bytes
            }
            StorageValue::Array(arr) => serde_json::to_vec(arr).map_err(|e| StorageLayoutError::SerializationError(e.to_string()))?,
            StorageValue::Mapping(map) => serde_json::to_vec(map).map_err(|e| StorageLayoutError::SerializationError(e.to_string()))?,
        };

        Ok(Value::from(bytes))
    }

    /// Decode storage value from bytes
    pub fn decode(bytes: &[u8], expected_type: &str) -> Result<Self, StorageLayoutError> {
        match expected_type {
            "bytes" => Ok(StorageValue::Bytes(bytes.to_vec())),
            "uint256" => {
                if bytes.len() != 32 {
                    return Err(StorageLayoutError::EncodingError("U256 must be exactly 32 bytes".to_string()));
                }
                let mut array = [0u8; 32];
                array.copy_from_slice(bytes);
                Ok(StorageValue::U256(array))
            }
            "bool" => {
                let value = bytes.last().copied().unwrap_or(0) != 0;
                Ok(StorageValue::Bool(value))
            }
            "string" => {
                // Find the last non-zero byte
                let end = bytes.iter().rposition(|&b| b != 0).map(|i| i + 1).unwrap_or(0);
                let trimmed = &bytes[..end];
                let s = String::from_utf8_lossy(trimmed).to_string();
                Ok(StorageValue::String(s))
            }
            "array" => {
                let arr: Vec<StorageValue> = serde_json::from_slice(bytes).map_err(|e| StorageLayoutError::SerializationError(e.to_string()))?;
                Ok(StorageValue::Array(arr))
            }
            "mapping" => {
                let map: HashMap<String, StorageValue> = serde_json::from_slice(bytes).map_err(|e| StorageLayoutError::SerializationError(e.to_string()))?;
                Ok(StorageValue::Mapping(map))
            }
            _ => Err(StorageLayoutError::InvalidVariableType),
        }
    }

    /// Create a U256 value from a number
    pub fn from_u256(value: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[24..32].copy_from_slice(&value.to_be_bytes());
        StorageValue::U256(bytes)
    }

    /// Convert to U256 if possible
    pub fn as_u256(&self) -> Option<[u8; 32]> {
        match self {
            StorageValue::U256(data) => Some(*data),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_storage_layout_creation() {
        let address = [1u8; 20];
        let layout = DotStorageLayout::new(address);

        assert_eq!(layout.dot_address, address);
        assert_eq!(layout.next_slot, 0);
        assert!(layout.variables.is_empty());
        assert!(layout.packed_storage);
    }

    #[test]
    fn test_add_simple_variable() {
        let mut layout = DotStorageLayout::new([1u8; 20]);

        let slot = layout.add_variable("balance".to_string(), StorageVariableType::Simple).unwrap();

        assert_eq!(slot, 0);
        assert_eq!(layout.next_slot, 1);
        assert!(layout.variables.contains_key("balance"));
    }

    #[test]
    fn test_add_dynamic_array_variable() {
        let mut layout = DotStorageLayout::new([1u8; 20]);

        let slot = layout.add_variable("items".to_string(), StorageVariableType::DynamicArray).unwrap();

        assert_eq!(slot, 0);
        assert_eq!(layout.next_slot, 1);

        let variable = layout.get_variable("items").unwrap();
        assert_eq!(variable.slot_count, 1);
        assert_eq!(variable.base_slot, 0);
    }

    #[test]
    fn test_add_mapping_variable() {
        let mut layout = DotStorageLayout::new([1u8; 20]);

        let slot = layout
            .add_variable(
                "balances".to_string(),
                StorageVariableType::Mapping {
                    key_type: "address".to_string(),
                    value_type: "uint256".to_string(),
                },
            )
            .unwrap();

        assert_eq!(slot, 0);
        assert_eq!(layout.next_slot, 1);

        let variable = layout.get_variable("balances").unwrap();
        assert_eq!(variable.slot_count, 1);
    }

    #[test]
    fn test_add_struct_variable() {
        let mut layout = DotStorageLayout::new([1u8; 20]);

        let fields = vec![
            StructField {
                name: "x".to_string(),
                field_type: "uint256".to_string(),
                offset: 0,
                size: 32,
            },
            StructField {
                name: "y".to_string(),
                field_type: "uint256".to_string(),
                offset: 32,
                size: 32,
            },
        ];

        let slot = layout.add_variable("point".to_string(), StorageVariableType::Struct { fields }).unwrap();

        assert_eq!(slot, 0);
        assert_eq!(layout.next_slot, 2); // Two slots for the struct

        let variable = layout.get_variable("point").unwrap();
        assert_eq!(variable.slot_count, 2);
    }

    #[test]
    fn test_generate_storage_key() {
        let layout = DotStorageLayout::new([1u8; 20]);
        let key = layout.generate_storage_key(5).unwrap();

        // Key should be 52 bytes: 20 (address) + 32 (slot)
        assert_eq!(key.len(), 52);

        // First 20 bytes should be the contract address
        assert_eq!(&key[0..20], &[1u8; 20]);

        // Last 4 bytes should contain the slot number (5)
        assert_eq!(&key[48..52], &5u32.to_be_bytes());
    }

    #[test]
    fn test_generate_mapping_key() {
        let layout = DotStorageLayout::new([1u8; 20]);
        let mapping_key = b"test_key";
        let key = layout.generate_mapping_key(0, mapping_key).unwrap();

        // Key should be 52 bytes: 20 (address) + 32 (hash)
        assert_eq!(key.len(), 52);

        // First 20 bytes should be the contract address
        assert_eq!(&key[0..20], &[1u8; 20]);

        // Different mapping keys should generate different storage keys
        let key2 = layout.generate_mapping_key(0, b"different_key").unwrap();
        assert_ne!(key, key2);
    }

    #[test]
    fn test_generate_array_key() {
        let layout = DotStorageLayout::new([1u8; 20]);
        let key1 = layout.generate_array_key(0, 0).unwrap();
        let key2 = layout.generate_array_key(0, 1).unwrap();

        // Keys should be different for different indices
        assert_ne!(key1, key2);

        // Both should be 52 bytes
        assert_eq!(key1.len(), 52);
        assert_eq!(key2.len(), 52);

        // First 20 bytes should be the contract address
        assert_eq!(&key1[0..20], &[1u8; 20]);
        assert_eq!(&key2[0..20], &[1u8; 20]);
    }

    #[test]
    fn test_storage_value_encoding() {
        // Test U256 encoding
        let value = StorageValue::from_u256(42);
        let encoded = value.encode().unwrap();
        assert_eq!(encoded.len(), 32);

        // Test boolean encoding
        let bool_value = StorageValue::Bool(true);
        let encoded_bool = bool_value.encode().unwrap();
        assert_eq!(encoded_bool.len(), 32);
        assert_eq!(encoded_bool[31], 1);

        // Test string encoding
        let string_value = StorageValue::String("hello".to_string());
        let encoded_string = string_value.encode().unwrap();
        assert_eq!(encoded_string.len(), 32);
    }

    #[test]
    fn test_storage_value_decoding() {
        // Test U256 decoding
        let mut bytes = [0u8; 32];
        bytes[31] = 42;
        let decoded = StorageValue::decode(&bytes, "uint256").unwrap();
        assert_eq!(decoded.as_u256().unwrap()[31], 42);

        // Test boolean decoding
        let mut bool_bytes = [0u8; 32];
        bool_bytes[31] = 1;
        let decoded_bool = StorageValue::decode(&bool_bytes, "bool").unwrap();
        assert_eq!(decoded_bool, StorageValue::Bool(true));

        // Test string decoding
        let mut string_bytes = [0u8; 32];
        string_bytes[0..5].copy_from_slice(b"hello");
        let decoded_string = StorageValue::decode(&string_bytes, "string").unwrap();
        assert_eq!(decoded_string, StorageValue::String("hello".to_string()));
    }

    #[test]
    fn test_slot_count_calculation() {
        let layout = DotStorageLayout::new([1u8; 20]);

        // Simple type should use 1 slot
        assert_eq!(layout.calculate_slot_count(&StorageVariableType::Simple), 1);

        // Dynamic array should use 1 slot (for length)
        assert_eq!(layout.calculate_slot_count(&StorageVariableType::DynamicArray), 1);

        // Mapping should use 1 slot (marker)
        assert_eq!(
            layout.calculate_slot_count(&StorageVariableType::Mapping {
                key_type: "address".to_string(),
                value_type: "uint256".to_string(),
            }),
            1
        );

        // Struct with packed storage
        let fields = vec![
            StructField {
                name: "a".to_string(),
                field_type: "uint128".to_string(),
                offset: 0,
                size: 16,
            },
            StructField {
                name: "b".to_string(),
                field_type: "uint128".to_string(),
                offset: 16,
                size: 16,
            },
        ];
        assert_eq!(layout.calculate_slot_count(&StorageVariableType::Struct { fields }), 1);
    }

    #[test]
    fn test_multiple_variables() {
        let mut layout = DotStorageLayout::new([1u8; 20]);

        // Add multiple variables
        let slot1 = layout.add_variable("var1".to_string(), StorageVariableType::Simple).unwrap();
        let slot2 = layout.add_variable("var2".to_string(), StorageVariableType::Simple).unwrap();
        let slot3 = layout.add_variable("var3".to_string(), StorageVariableType::DynamicArray).unwrap();

        assert_eq!(slot1, 0);
        assert_eq!(slot2, 1);
        assert_eq!(slot3, 2);
        assert_eq!(layout.next_slot, 3);

        // Verify all variables are stored
        assert!(layout.get_variable("var1").is_some());
        assert!(layout.get_variable("var2").is_some());
        assert!(layout.get_variable("var3").is_some());
        assert!(layout.get_variable("nonexistent").is_none());
    }

    #[test]
    fn test_key_uniqueness() {
        let layout = DotStorageLayout::new([1u8; 20]);

        // Generate multiple keys and ensure they're all different
        let key1 = layout.generate_storage_key(0).unwrap();
        let key2 = layout.generate_storage_key(1).unwrap();
        let key3 = layout.generate_mapping_key(0, b"test").unwrap();
        let key4 = layout.generate_array_key(0, 0).unwrap();

        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key1, key4);
        assert_ne!(key2, key3);
        assert_ne!(key2, key4);
        assert_ne!(key3, key4);
    }
}
