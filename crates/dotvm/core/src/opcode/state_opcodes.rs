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

//! # State Opcodes
//!
//! This module defines opcodes for dot state access operations.
//! These opcodes interact with the dot storage layout and the underlying
//! Merkle Patricia Trie for efficient and secure state management.
//!
//! ## Key Opcodes
//!
//! - `SLOAD`: Load a value from dot storage
//! - `SSTORE`: Store a value to dot storage  
//! - `SSIZE`: Get the size of stored data
//! - `SEXISTS`: Check if a storage key exists
//! - `SKEYS`: Iterate over storage keys
//! - `SCLEAR`: Clear a storage slot
//! - `SMULTILOAD`: Load multiple values efficiently
//! - `SMULTISTORE`: Store multiple values efficiently

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// State-related opcodes for dot execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum StateOpcode {
    /// Load a value from dot storage
    /// Stack: [key] -> [value]
    SLOAD = 0x54,

    /// Store a value to dot storage
    /// Stack: [key, value] -> []
    SSTORE = 0x55,

    /// Get the size of stored data at key
    /// Stack: [key] -> [size]
    SSIZE = 0x56,

    /// Check if a storage key exists
    /// Stack: [key] -> [exists]
    SEXISTS = 0x57,

    /// Clear a storage slot (set to zero)
    /// Stack: [key] -> []
    SCLEAR = 0x58,

    /// Load multiple values from dot storage
    /// Stack: [key_count, key1, key2, ...] -> [value1, value2, ...]
    SMULTILOAD = 0x59,

    /// Store multiple values to dot storage
    /// Stack: [key_count, key1, value1, key2, value2, ...] -> []
    SMULTISTORE = 0x5A,

    /// Get storage keys iterator (for debugging/introspection)
    /// Stack: [start_key, max_count] -> [key_count, key1, key2, ...]
    SKEYS = 0x5B,

    // Advanced State Management Opcodes (ACID Operations)
    /// Read dot state with MVCC isolation
    /// Stack: [state_key] -> [value] or [null]
    StateRead = 0x5C,

    /// Write dot state with MVCC versioning
    /// Stack: [state_key, value] -> []
    StateWrite = 0x5D,

    /// Commit pending state changes atomically
    /// Stack: [] -> [state_root_hash]
    StateCommit = 0x5E,

    /// Rollback state changes to previous consistent state
    /// Stack: [] -> []
    StateRollback = 0x5F,

    /// Perform Merkle tree operations (proof generation/verification)
    /// Stack: [operation_type, key] -> [proof_data] or [verification_result]
    StateMerkle = 0x60,

    /// Create a point-in-time state snapshot
    /// Stack: [snapshot_id] -> []
    StateSnapshot = 0x61,

    /// Restore state from a snapshot
    /// Stack: [snapshot_id] -> []
    StateRestore = 0x62,
}

/// Configuration for state operation costs (deprecated)
#[derive(Debug, Clone, Default)]
pub struct StateOperationCosts {
    /// Base cost for operations
    pub base_cost: u64,
}

/// State operation context
#[derive(Debug, Clone)]
pub struct StateOperationContext {
    /// dot address performing the operation
    pub dot_address: [u8; 20],
    /// Operation costs configuration
    pub operation_costs: StateOperationCosts,
    /// Whether the operation is in a static context (read-only)
    pub is_static: bool,
    /// Transaction-level storage changes
    pub tx_storage_changes: BTreeMap<Vec<u8>, StorageChange>,
}

/// Storage change type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageChange {
    /// Value was set for the first time
    Set,
    /// Value was updated
    Update,
    /// Value was cleared
    Clear,
}

/// Result of a state operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateOperationResult {
    /// Operation output data
    pub output: Vec<u8>,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if operation failed
    pub error: Option<String>,
}

/// Errors that can occur during state operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateOpcodeError {
    /// Invalid opcode
    InvalidOpcode(u8),
    /// Stack underflow
    StackUnderflow,
    /// Stack overflow
    StackOverflow,
    /// Invalid storage key format
    InvalidStorageKey,
    /// Storage access denied (e.g., in static context)
    StorageAccessDenied,
    /// Storage write denied (e.g., in static context)
    StorageWriteDenied,
    /// Invalid data format
    InvalidDataFormat(String),
    /// Storage backend error
    StorageError(String),
    /// Operation limit exceeded
    OperationLimitExceeded,
}

impl fmt::Display for StateOpcodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateOpcodeError::InvalidOpcode(code) => write!(f, "Invalid opcode: 0x{code:02x}"),
            StateOpcodeError::StackUnderflow => write!(f, "Stack underflow"),
            StateOpcodeError::StackOverflow => write!(f, "Stack overflow"),
            StateOpcodeError::InvalidStorageKey => write!(f, "Invalid storage key format"),
            StateOpcodeError::StorageAccessDenied => write!(f, "Storage access denied"),
            StateOpcodeError::StorageWriteDenied => write!(f, "Storage write denied"),
            StateOpcodeError::InvalidDataFormat(msg) => write!(f, "Invalid data format: {msg}"),
            StateOpcodeError::StorageError(msg) => write!(f, "Storage error: {msg}"),
            StateOpcodeError::OperationLimitExceeded => write!(f, "Operation limit exceeded"),
        }
    }
}

impl std::error::Error for StateOpcodeError {}

impl StateOpcode {
    /// Convert u8 to StateOpcode
    pub fn from_u8(byte: u8) -> Result<Self, StateOpcodeError> {
        match byte {
            0x54 => Ok(StateOpcode::SLOAD),
            0x55 => Ok(StateOpcode::SSTORE),
            0x56 => Ok(StateOpcode::SSIZE),
            0x57 => Ok(StateOpcode::SEXISTS),
            0x58 => Ok(StateOpcode::SCLEAR),
            0x59 => Ok(StateOpcode::SMULTILOAD),
            0x5A => Ok(StateOpcode::SMULTISTORE),
            0x5B => Ok(StateOpcode::SKEYS),
            0x5C => Ok(StateOpcode::StateRead),
            0x5D => Ok(StateOpcode::StateWrite),
            0x5E => Ok(StateOpcode::StateCommit),
            0x5F => Ok(StateOpcode::StateRollback),
            0x60 => Ok(StateOpcode::StateMerkle),
            0x61 => Ok(StateOpcode::StateSnapshot),
            0x62 => Ok(StateOpcode::StateRestore),
            _ => Err(StateOpcodeError::InvalidOpcode(byte)),
        }
    }

    /// Convert StateOpcode to u8
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Get the name of the opcode
    pub fn name(self) -> &'static str {
        match self {
            StateOpcode::SLOAD => "SLOAD",
            StateOpcode::SSTORE => "SSTORE",
            StateOpcode::SSIZE => "SSIZE",
            StateOpcode::SEXISTS => "SEXISTS",
            StateOpcode::SCLEAR => "SCLEAR",
            StateOpcode::SMULTILOAD => "SMULTILOAD",
            StateOpcode::SMULTISTORE => "SMULTISTORE",
            StateOpcode::SKEYS => "SKEYS",
            StateOpcode::StateRead => "StateRead",
            StateOpcode::StateWrite => "StateWrite",
            StateOpcode::StateCommit => "StateCommit",
            StateOpcode::StateRollback => "StateRollback",
            StateOpcode::StateMerkle => "StateMerkle",
            StateOpcode::StateSnapshot => "StateSnapshot",
            StateOpcode::StateRestore => "StateRestore",
        }
    }

    /// Check if the opcode modifies state
    pub fn is_state_modifying(self) -> bool {
        matches!(
            self,
            StateOpcode::SSTORE | StateOpcode::SCLEAR | StateOpcode::SMULTISTORE | StateOpcode::StateWrite | StateOpcode::StateCommit | StateOpcode::StateRollback | StateOpcode::StateRestore
        )
    }

    /// Get the minimum stack size required for this opcode
    pub fn min_stack_size(self) -> usize {
        match self {
            StateOpcode::SLOAD | StateOpcode::SSIZE | StateOpcode::SEXISTS | StateOpcode::SCLEAR => 1,
            StateOpcode::SSTORE => 2,
            StateOpcode::SKEYS => 2,
            StateOpcode::SMULTILOAD | StateOpcode::SMULTISTORE => 1, // At least count
            StateOpcode::StateRead => 1,
            StateOpcode::StateWrite => 2,
            StateOpcode::StateCommit | StateOpcode::StateRollback => 0,
            StateOpcode::StateMerkle => 2,                               // operation_type + key
            StateOpcode::StateSnapshot | StateOpcode::StateRestore => 1, // snapshot_id
        }
    }

    /// Get the maximum stack size produced by this opcode
    pub fn max_stack_output(self) -> usize {
        match self {
            StateOpcode::SLOAD | StateOpcode::SSIZE | StateOpcode::SEXISTS => 1,
            StateOpcode::SSTORE | StateOpcode::SCLEAR => 0,
            StateOpcode::SMULTILOAD | StateOpcode::SMULTISTORE | StateOpcode::SKEYS => 255, // Dynamic
            StateOpcode::StateRead => 1,                                                    // [value] or [null]
            StateOpcode::StateWrite => 0,                                                   // []
            StateOpcode::StateCommit => 1,                                                  // [state_root_hash]
            StateOpcode::StateRollback => 0,                                                // []
            StateOpcode::StateMerkle => 1,                                                  // [proof_data] or [verification_result]
            StateOpcode::StateSnapshot | StateOpcode::StateRestore => 0,                    // []
        }
    }
}

impl StateOperationContext {
    /// Create a new state operation context
    pub fn new(dot_address: [u8; 20]) -> Self {
        Self {
            dot_address,
            operation_costs: StateOperationCosts::default(),
            is_static: false,
            tx_storage_changes: BTreeMap::new(),
        }
    }

    /// Create a static context (read-only)
    pub fn new_static(dot_address: [u8; 20]) -> Self {
        Self {
            dot_address,
            operation_costs: StateOperationCosts::default(),
            is_static: true,
            tx_storage_changes: BTreeMap::new(),
        }
    }

    /// Check if state modification is allowed
    pub fn can_modify_state(&self) -> bool {
        !self.is_static
    }
}

impl StateOperationResult {
    /// Create a successful result
    pub fn success(output: Vec<u8>) -> Self {
        Self { output, success: true, error: None }
    }

    /// Create an error result
    pub fn error(error: StateOpcodeError) -> Self {
        Self {
            output: Vec::new(),
            success: false,
            error: Some(error.to_string()),
        }
    }
}

/// Helper functions for state opcode execution
pub mod execution {
    use super::*;

    /// Validate stack for opcode execution
    pub fn validate_stack(opcode: StateOpcode, stack: &[Vec<u8>]) -> Result<(), StateOpcodeError> {
        if stack.len() < opcode.min_stack_size() {
            return Err(StateOpcodeError::StackUnderflow);
        }

        // Additional validations based on opcode
        match opcode {
            StateOpcode::SMULTILOAD | StateOpcode::SMULTISTORE => {
                if stack.is_empty() {
                    return Err(StateOpcodeError::StackUnderflow);
                }

                let count = u64::from_be_bytes(
                    stack[0]
                        .get(24..32)
                        .ok_or(StateOpcodeError::InvalidDataFormat("Invalid count".to_string()))?
                        .try_into()
                        .map_err(|_| StateOpcodeError::InvalidDataFormat("Invalid count format".to_string()))?,
                );

                let required_args = match opcode {
                    StateOpcode::SMULTILOAD => 1 + count as usize,        // count + keys
                    StateOpcode::SMULTISTORE => 1 + (count as usize * 2), // count + key-value pairs
                    _ => unreachable!(),
                };

                if stack.len() < required_args {
                    return Err(StateOpcodeError::StackUnderflow);
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Convert 32-byte value to storage key
    pub fn to_storage_key(value: &[u8]) -> Result<Vec<u8>, StateOpcodeError> {
        if value.len() != 32 {
            return Err(StateOpcodeError::InvalidStorageKey);
        }
        Ok(value.to_vec())
    }

    /// Convert value to 32-byte padded format
    pub fn to_padded_value(value: &[u8]) -> Vec<u8> {
        let mut padded = vec![0u8; 32];
        let start = if value.len() <= 32 { 32 - value.len() } else { 0 };

        let copy_len = value.len().min(32);
        padded[start..start + copy_len].copy_from_slice(&value[..copy_len]);
        padded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_opcode_from_u8() {
        assert_eq!(StateOpcode::from_u8(0x54).unwrap(), StateOpcode::SLOAD);
        assert_eq!(StateOpcode::from_u8(0x55).unwrap(), StateOpcode::SSTORE);
        assert_eq!(StateOpcode::from_u8(0x56).unwrap(), StateOpcode::SSIZE);
        assert_eq!(StateOpcode::from_u8(0x57).unwrap(), StateOpcode::SEXISTS);
        assert_eq!(StateOpcode::from_u8(0x58).unwrap(), StateOpcode::SCLEAR);
        assert_eq!(StateOpcode::from_u8(0x59).unwrap(), StateOpcode::SMULTILOAD);
        assert_eq!(StateOpcode::from_u8(0x5A).unwrap(), StateOpcode::SMULTISTORE);
        assert_eq!(StateOpcode::from_u8(0x5B).unwrap(), StateOpcode::SKEYS);
        assert_eq!(StateOpcode::from_u8(0x5C).unwrap(), StateOpcode::StateRead);
        assert_eq!(StateOpcode::from_u8(0x5D).unwrap(), StateOpcode::StateWrite);
        assert_eq!(StateOpcode::from_u8(0x5E).unwrap(), StateOpcode::StateCommit);
        assert_eq!(StateOpcode::from_u8(0x5F).unwrap(), StateOpcode::StateRollback);
        assert_eq!(StateOpcode::from_u8(0x60).unwrap(), StateOpcode::StateMerkle);
        assert_eq!(StateOpcode::from_u8(0x61).unwrap(), StateOpcode::StateSnapshot);
        assert_eq!(StateOpcode::from_u8(0x62).unwrap(), StateOpcode::StateRestore);

        assert!(StateOpcode::from_u8(0xFF).is_err());
    }

    #[test]
    fn test_state_opcode_to_u8() {
        assert_eq!(StateOpcode::SLOAD.to_u8(), 0x54);
        assert_eq!(StateOpcode::SSTORE.to_u8(), 0x55);
        assert_eq!(StateOpcode::SSIZE.to_u8(), 0x56);
        assert_eq!(StateOpcode::SEXISTS.to_u8(), 0x57);
        assert_eq!(StateOpcode::SCLEAR.to_u8(), 0x58);
        assert_eq!(StateOpcode::SMULTILOAD.to_u8(), 0x59);
        assert_eq!(StateOpcode::SMULTISTORE.to_u8(), 0x5A);
        assert_eq!(StateOpcode::SKEYS.to_u8(), 0x5B);
        assert_eq!(StateOpcode::StateRead.to_u8(), 0x5C);
        assert_eq!(StateOpcode::StateWrite.to_u8(), 0x5D);
        assert_eq!(StateOpcode::StateCommit.to_u8(), 0x5E);
        assert_eq!(StateOpcode::StateRollback.to_u8(), 0x5F);
        assert_eq!(StateOpcode::StateMerkle.to_u8(), 0x60);
        assert_eq!(StateOpcode::StateSnapshot.to_u8(), 0x61);
        assert_eq!(StateOpcode::StateRestore.to_u8(), 0x62);
    }

    #[test]
    fn test_opcode_names() {
        assert_eq!(StateOpcode::SLOAD.name(), "SLOAD");
        assert_eq!(StateOpcode::SSTORE.name(), "SSTORE");
        assert_eq!(StateOpcode::SSIZE.name(), "SSIZE");
        assert_eq!(StateOpcode::SEXISTS.name(), "SEXISTS");
        assert_eq!(StateOpcode::SCLEAR.name(), "SCLEAR");
        assert_eq!(StateOpcode::SMULTILOAD.name(), "SMULTILOAD");
        assert_eq!(StateOpcode::SMULTISTORE.name(), "SMULTISTORE");
        assert_eq!(StateOpcode::SKEYS.name(), "SKEYS");
        assert_eq!(StateOpcode::StateRead.name(), "StateRead");
        assert_eq!(StateOpcode::StateWrite.name(), "StateWrite");
        assert_eq!(StateOpcode::StateCommit.name(), "StateCommit");
        assert_eq!(StateOpcode::StateRollback.name(), "StateRollback");
        assert_eq!(StateOpcode::StateMerkle.name(), "StateMerkle");
        assert_eq!(StateOpcode::StateSnapshot.name(), "StateSnapshot");
        assert_eq!(StateOpcode::StateRestore.name(), "StateRestore");
    }

    #[test]
    fn test_state_modifying_opcodes() {
        assert!(!StateOpcode::SLOAD.is_state_modifying());
        assert!(StateOpcode::SSTORE.is_state_modifying());
        assert!(!StateOpcode::SSIZE.is_state_modifying());
        assert!(!StateOpcode::StateRead.is_state_modifying());
        assert!(StateOpcode::StateWrite.is_state_modifying());
        assert!(StateOpcode::StateCommit.is_state_modifying());
        assert!(StateOpcode::StateRollback.is_state_modifying());
        assert!(!StateOpcode::StateMerkle.is_state_modifying());
        assert!(!StateOpcode::StateSnapshot.is_state_modifying());
        assert!(StateOpcode::StateRestore.is_state_modifying());
        assert!(!StateOpcode::SEXISTS.is_state_modifying());
        assert!(StateOpcode::SCLEAR.is_state_modifying());
        assert!(!StateOpcode::SMULTILOAD.is_state_modifying());
        assert!(StateOpcode::SMULTISTORE.is_state_modifying());
        assert!(!StateOpcode::SKEYS.is_state_modifying());
    }

    #[test]
    fn test_min_stack_size() {
        assert_eq!(StateOpcode::SLOAD.min_stack_size(), 1);
        assert_eq!(StateOpcode::SSTORE.min_stack_size(), 2);
        assert_eq!(StateOpcode::SSIZE.min_stack_size(), 1);
        assert_eq!(StateOpcode::SEXISTS.min_stack_size(), 1);
        assert_eq!(StateOpcode::SCLEAR.min_stack_size(), 1);
        assert_eq!(StateOpcode::SMULTILOAD.min_stack_size(), 1);
        assert_eq!(StateOpcode::SMULTISTORE.min_stack_size(), 1);
        assert_eq!(StateOpcode::SKEYS.min_stack_size(), 2);
    }

    #[test]
    fn test_execution_helpers() {
        use execution::*;

        // Test stack validation
        let stack = vec![vec![0u8; 32]];
        assert!(validate_stack(StateOpcode::SLOAD, &stack).is_ok());
        assert!(validate_stack(StateOpcode::SSTORE, &stack).is_err()); // Needs 2 items

        // Test storage key conversion
        let key_bytes = vec![0u8; 32];
        assert!(to_storage_key(&key_bytes).is_ok());
        assert!(to_storage_key(&vec![0u8; 16]).is_err());

        // Test value padding
        let padded = to_padded_value(&[1, 2, 3]);
        assert_eq!(padded.len(), 32);
        assert_eq!(&padded[29..32], &[1, 2, 3]);
    }

    #[test]
    fn test_multi_operation_validation() {
        use execution::*;

        // Test SMULTILOAD with valid stack
        let mut stack = vec![vec![0u8; 32]]; // count = 0
        stack[0][31] = 2; // count = 2
        stack.push(vec![1u8; 32]); // key1
        stack.push(vec![2u8; 32]); // key2

        assert!(validate_stack(StateOpcode::SMULTILOAD, &stack).is_ok());

        // Test SMULTILOAD with insufficient stack
        let mut stack = vec![vec![0u8; 32]]; // count = 0, but count[31] = 2
        stack[0][31] = 2;
        assert!(validate_stack(StateOpcode::SMULTILOAD, &stack).is_err());
    }

    #[test]
    fn test_storage_change_types() {
        assert_eq!(StorageChange::Set, StorageChange::Set);
        assert_ne!(StorageChange::Set, StorageChange::Update);
        assert_ne!(StorageChange::Update, StorageChange::Clear);
    }

    #[test]
    fn test_error_display() {
        let error = StateOpcodeError::InvalidOpcode(0xFF);
        assert_eq!(error.to_string(), "Invalid opcode: 0xff");

        let error = StateOpcodeError::StorageError("Test error".to_string());
        assert_eq!(error.to_string(), "Storage error: Test error");
    }
}
