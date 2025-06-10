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
//! This module defines opcodes for smart contract state access operations.
//! These opcodes interact with the contract storage layout and the underlying
//! Merkle Patricia Trie for efficient and secure state management.
//!
//! ## Key Opcodes
//!
//! - `SLOAD`: Load a value from contract storage
//! - `SSTORE`: Store a value to contract storage  
//! - `SSIZE`: Get the size of stored data
//! - `SEXISTS`: Check if a storage key exists
//! - `SKEYS`: Iterate over storage keys (with gas limits)
//! - `SCLEAR`: Clear a storage slot
//! - `SMULTILOAD`: Load multiple values efficiently
//! - `SMULTISTORE`: Store multiple values efficiently

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// State-related opcodes for smart contract execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum StateOpcode {
    /// Load a value from contract storage
    /// Stack: [key] -> [value]
    /// Gas: BASE_GAS + STORAGE_ACCESS_GAS
    SLOAD = 0x54,

    /// Store a value to contract storage
    /// Stack: [key, value] -> []
    /// Gas: BASE_GAS + STORAGE_WRITE_GAS + (conditional STORAGE_SET_GAS)
    SSTORE = 0x55,

    /// Get the size of stored data at key
    /// Stack: [key] -> [size]
    /// Gas: BASE_GAS + STORAGE_ACCESS_GAS
    SSIZE = 0x56,

    /// Check if a storage key exists
    /// Stack: [key] -> [exists]
    /// Gas: BASE_GAS + STORAGE_ACCESS_GAS
    SEXISTS = 0x57,

    /// Clear a storage slot (set to zero)
    /// Stack: [key] -> []
    /// Gas: BASE_GAS + STORAGE_CLEAR_GAS
    SCLEAR = 0x58,

    /// Load multiple values from contract storage
    /// Stack: [key_count, key1, key2, ...] -> [value1, value2, ...]
    /// Gas: BASE_GAS + (key_count * STORAGE_ACCESS_GAS)
    SMULTILOAD = 0x59,

    /// Store multiple values to contract storage
    /// Stack: [key_count, key1, value1, key2, value2, ...] -> []
    /// Gas: BASE_GAS + (key_count * STORAGE_WRITE_GAS)
    SMULTISTORE = 0x5A,

    /// Get storage keys iterator (for debugging/introspection)
    /// Stack: [start_key, max_count] -> [key_count, key1, key2, ...]
    /// Gas: BASE_GAS + (result_count * STORAGE_ACCESS_GAS)
    SKEYS = 0x5B,
}

/// Gas costs for state operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateGasCosts {
    /// Base gas cost for any state operation
    pub base_gas: u64,
    /// Gas cost for reading from storage
    pub storage_access_gas: u64,
    /// Gas cost for writing to storage (existing key)
    pub storage_write_gas: u64,
    /// Gas cost for setting new storage (new key)
    pub storage_set_gas: u64,
    /// Gas cost for clearing storage (refund available)
    pub storage_clear_gas: u64,
    /// Gas refund when clearing storage
    pub storage_clear_refund: u64,
    /// Maximum gas for iteration operations
    pub max_iteration_gas: u64,
}

impl Default for StateGasCosts {
    fn default() -> Self {
        Self {
            base_gas: 3,
            storage_access_gas: 200,
            storage_write_gas: 5000,
            storage_set_gas: 20000,
            storage_clear_gas: 5000,
            storage_clear_refund: 15000,
            max_iteration_gas: 100000,
        }
    }
}

/// State operation context
#[derive(Debug, Clone)]
pub struct StateOperationContext {
    /// Contract address performing the operation
    pub contract_address: [u8; 20],
    /// Current gas limit
    pub gas_limit: u64,
    /// Gas costs configuration
    pub gas_costs: StateGasCosts,
    /// Whether the operation is in a static context (read-only)
    pub is_static: bool,
    /// Transaction-level storage changes for gas calculation
    pub tx_storage_changes: BTreeMap<Vec<u8>, StorageChange>,
}

/// Storage change type for gas calculation
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
    /// Gas consumed by the operation
    pub gas_used: u64,
    /// Gas refund (if any)
    pub gas_refund: u64,
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
    /// Insufficient gas for operation
    InsufficientGas { required: u64, available: u64 },
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
            StateOpcodeError::InsufficientGas { required, available } => {
                write!(f, "Insufficient gas: required {}, available {}", required, available)
            }
            StateOpcodeError::InvalidOpcode(code) => write!(f, "Invalid opcode: 0x{:02x}", code),
            StateOpcodeError::StackUnderflow => write!(f, "Stack underflow"),
            StateOpcodeError::StackOverflow => write!(f, "Stack overflow"),
            StateOpcodeError::InvalidStorageKey => write!(f, "Invalid storage key format"),
            StateOpcodeError::StorageAccessDenied => write!(f, "Storage access denied"),
            StateOpcodeError::StorageWriteDenied => write!(f, "Storage write denied"),
            StateOpcodeError::InvalidDataFormat(msg) => write!(f, "Invalid data format: {}", msg),
            StateOpcodeError::StorageError(msg) => write!(f, "Storage error: {}", msg),
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
        }
    }

    /// Check if the opcode modifies state
    pub fn is_state_modifying(self) -> bool {
        matches!(self, StateOpcode::SSTORE | StateOpcode::SCLEAR | StateOpcode::SMULTISTORE)
    }

    /// Get the minimum stack size required for this opcode
    pub fn min_stack_size(self) -> usize {
        match self {
            StateOpcode::SLOAD | StateOpcode::SSIZE | StateOpcode::SEXISTS | StateOpcode::SCLEAR => 1,
            StateOpcode::SSTORE => 2,
            StateOpcode::SKEYS => 2,
            StateOpcode::SMULTILOAD | StateOpcode::SMULTISTORE => 1, // At least count
        }
    }

    /// Get the maximum stack size produced by this opcode
    pub fn max_stack_output(self) -> usize {
        match self {
            StateOpcode::SLOAD | StateOpcode::SSIZE | StateOpcode::SEXISTS => 1,
            StateOpcode::SSTORE | StateOpcode::SCLEAR => 0,
            StateOpcode::SMULTILOAD | StateOpcode::SMULTISTORE | StateOpcode::SKEYS => 255, // Dynamic
        }
    }

    /// Calculate the gas cost for this opcode
    pub fn calculate_gas_cost(self, context: &StateOperationContext, stack_args: &[Vec<u8>]) -> Result<u64, StateOpcodeError> {
        let base_cost = context.gas_costs.base_gas;

        match self {
            StateOpcode::SLOAD | StateOpcode::SSIZE | StateOpcode::SEXISTS => Ok(base_cost + context.gas_costs.storage_access_gas),
            StateOpcode::SSTORE => {
                let key = &stack_args[0];
                let value = &stack_args[1];

                let mut cost = base_cost;

                // Check if this is a new storage slot or modification
                if let Some(change) = context.tx_storage_changes.get(key) {
                    cost += match change {
                        StorageChange::Set => context.gas_costs.storage_set_gas,
                        StorageChange::Update => context.gas_costs.storage_write_gas,
                        StorageChange::Clear => context.gas_costs.storage_clear_gas,
                    };
                } else {
                    // First time writing to this slot in transaction
                    if value.iter().all(|&b| b == 0) {
                        cost += context.gas_costs.storage_clear_gas;
                    } else {
                        cost += context.gas_costs.storage_set_gas;
                    }
                }

                Ok(cost)
            }
            StateOpcode::SCLEAR => Ok(base_cost + context.gas_costs.storage_clear_gas),
            StateOpcode::SMULTILOAD => {
                if stack_args.is_empty() {
                    return Err(StateOpcodeError::StackUnderflow);
                }

                let count = u64::from_be_bytes(
                    stack_args[0]
                        .get(24..32)
                        .ok_or(StateOpcodeError::InvalidDataFormat("Invalid count".to_string()))?
                        .try_into()
                        .map_err(|_| StateOpcodeError::InvalidDataFormat("Invalid count format".to_string()))?,
                );

                Ok(base_cost + count * context.gas_costs.storage_access_gas)
            }
            StateOpcode::SMULTISTORE => {
                if stack_args.is_empty() {
                    return Err(StateOpcodeError::StackUnderflow);
                }

                let count = u64::from_be_bytes(
                    stack_args[0]
                        .get(24..32)
                        .ok_or(StateOpcodeError::InvalidDataFormat("Invalid count".to_string()))?
                        .try_into()
                        .map_err(|_| StateOpcodeError::InvalidDataFormat("Invalid count format".to_string()))?,
                );

                Ok(base_cost + count * context.gas_costs.storage_write_gas)
            }
            StateOpcode::SKEYS => {
                if stack_args.len() < 2 {
                    return Err(StateOpcodeError::StackUnderflow);
                }

                let max_count = u64::from_be_bytes(
                    stack_args[1]
                        .get(24..32)
                        .ok_or(StateOpcodeError::InvalidDataFormat("Invalid max_count".to_string()))?
                        .try_into()
                        .map_err(|_| StateOpcodeError::InvalidDataFormat("Invalid max_count format".to_string()))?,
                );

                let max_gas = (max_count * context.gas_costs.storage_access_gas).min(context.gas_costs.max_iteration_gas);
                Ok(base_cost + max_gas)
            }
        }
    }
}

impl StateOperationContext {
    /// Create a new state operation context
    pub fn new(contract_address: [u8; 20], gas_limit: u64) -> Self {
        Self {
            contract_address,
            gas_limit,
            gas_costs: StateGasCosts::default(),
            is_static: false,
            tx_storage_changes: BTreeMap::new(),
        }
    }

    /// Create a static context (read-only)
    pub fn new_static(contract_address: [u8; 20], gas_limit: u64) -> Self {
        Self {
            contract_address,
            gas_limit,
            gas_costs: StateGasCosts::default(),
            is_static: true,
            tx_storage_changes: BTreeMap::new(),
        }
    }

    /// Check if gas is available for an operation
    pub fn check_gas(&self, required: u64) -> Result<(), StateOpcodeError> {
        if self.gas_limit < required {
            Err(StateOpcodeError::InsufficientGas { required, available: self.gas_limit })
        } else {
            Ok(())
        }
    }

    /// Record a storage change for gas calculation
    pub fn record_storage_change(&mut self, key: Vec<u8>, change: StorageChange) {
        self.tx_storage_changes.insert(key, change);
    }

    /// Check if state modification is allowed
    pub fn can_modify_state(&self) -> bool {
        !self.is_static
    }
}

impl StateOperationResult {
    /// Create a successful result
    pub fn success(gas_used: u64, output: Vec<u8>) -> Self {
        Self {
            gas_used,
            gas_refund: 0,
            output,
            success: true,
            error: None,
        }
    }

    /// Create a successful result with gas refund
    pub fn success_with_refund(gas_used: u64, gas_refund: u64, output: Vec<u8>) -> Self {
        Self {
            gas_used,
            gas_refund,
            output,
            success: true,
            error: None,
        }
    }

    /// Create an error result
    pub fn error(gas_used: u64, error: StateOpcodeError) -> Self {
        Self {
            gas_used,
            gas_refund: 0,
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
    }

    #[test]
    fn test_state_modifying_opcodes() {
        assert!(!StateOpcode::SLOAD.is_state_modifying());
        assert!(StateOpcode::SSTORE.is_state_modifying());
        assert!(!StateOpcode::SSIZE.is_state_modifying());
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
    fn test_gas_cost_calculation() {
        let context = StateOperationContext::new([1u8; 20], 1000000);

        // SLOAD should cost base + access
        let args = vec![vec![0u8; 32]];
        let cost = StateOpcode::SLOAD.calculate_gas_cost(&context, &args).unwrap();
        assert_eq!(cost, context.gas_costs.base_gas + context.gas_costs.storage_access_gas);

        // SSTORE with new value should cost base + set
        let args = vec![vec![0u8; 32], vec![1u8; 32]];
        let cost = StateOpcode::SSTORE.calculate_gas_cost(&context, &args).unwrap();
        assert_eq!(cost, context.gas_costs.base_gas + context.gas_costs.storage_set_gas);
    }

    #[test]
    fn test_state_operation_context() {
        let mut context = StateOperationContext::new([1u8; 20], 100000);
        assert!(!context.is_static);
        assert!(context.can_modify_state());

        // Test gas checking
        assert!(context.check_gas(50000).is_ok());
        assert!(context.check_gas(150000).is_err());

        // Test storage change recording
        context.record_storage_change(vec![1, 2, 3], StorageChange::Set);
        assert!(context.tx_storage_changes.contains_key(&vec![1, 2, 3]));
    }

    #[test]
    fn test_static_context() {
        let context = StateOperationContext::new_static([1u8; 20], 100000);
        assert!(context.is_static);
        assert!(!context.can_modify_state());
    }

    #[test]
    fn test_state_operation_result() {
        let success_result = StateOperationResult::success(1000, vec![1, 2, 3]);
        assert!(success_result.success);
        assert_eq!(success_result.gas_used, 1000);
        assert_eq!(success_result.output, vec![1, 2, 3]);
        assert!(success_result.error.is_none());

        let refund_result = StateOperationResult::success_with_refund(1000, 500, vec![]);
        assert!(refund_result.success);
        assert_eq!(refund_result.gas_refund, 500);

        let error_result = StateOperationResult::error(500, StateOpcodeError::StackUnderflow);
        assert!(!error_result.success);
        assert_eq!(error_result.gas_used, 500);
        assert!(error_result.error.is_some());
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
    fn test_gas_costs_configuration() {
        let gas_costs = StateGasCosts::default();
        assert_eq!(gas_costs.base_gas, 3);
        assert_eq!(gas_costs.storage_access_gas, 200);
        assert_eq!(gas_costs.storage_write_gas, 5000);
        assert_eq!(gas_costs.storage_set_gas, 20000);
        assert_eq!(gas_costs.storage_clear_gas, 5000);
        assert_eq!(gas_costs.storage_clear_refund, 15000);
        assert_eq!(gas_costs.max_iteration_gas, 100000);
    }

    #[test]
    fn test_storage_change_types() {
        assert_eq!(StorageChange::Set, StorageChange::Set);
        assert_ne!(StorageChange::Set, StorageChange::Update);
        assert_ne!(StorageChange::Update, StorageChange::Clear);
    }

    #[test]
    fn test_error_display() {
        let error = StateOpcodeError::InsufficientGas { required: 1000, available: 500 };
        assert_eq!(error.to_string(), "Insufficient gas: required 1000, available 500");

        let error = StateOpcodeError::InvalidOpcode(0xFF);
        assert_eq!(error.to_string(), "Invalid opcode: 0xff");

        let error = StateOpcodeError::StorageError("Test error".to_string());
        assert_eq!(error.to_string(), "Storage error: Test error");
    }
}
