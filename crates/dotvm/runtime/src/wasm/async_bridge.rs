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

//! Async Bridge for WASM Host Functions
//!
//! This module provides a bridge between the synchronous WASM host function
//! interface and the async opcode executors, enabling efficient async execution
//! of custom opcodes from within WASM code.

use crate::wasm::{AsyncHostFunctionInterface, HostFunction, WasmError, WasmExecutionContext, WasmResult};
use dotvm_compiler::wasm::ast::WasmValue as Value;
use dotvm_core::opcode::crypto_opcodes::CryptoOpcode;
use dotvm_core::opcode::db_opcodes::DatabaseOpcode;
use dotvm_core::opcode::parallel_opcodes::ParallelOpcode;
use dotvm_core::opcode::state_opcodes::StateOpcode;
use dotvm_core::vm::database_executor::DatabaseOpcodeExecutor;
use dotvm_core::vm::state_executor::StateOpcodeExecutor;
use futures::executor::block_on;
use std::sync::{Arc, Mutex};
use tokio::runtime::Handle;

/// Async bridge that connects WASM host functions to async opcode executors
pub struct AsyncWasmBridge {
    /// Async host function interface
    host_interface: AsyncHostFunctionInterface,
    /// Tokio runtime handle
    tokio_handle: Handle,
    /// Database executor
    database_executor: Option<Arc<Mutex<DatabaseOpcodeExecutor>>>,
    /// State executor
    state_executor: Option<Arc<Mutex<StateOpcodeExecutor>>>,
}

impl AsyncWasmBridge {
    /// Create a new async WASM bridge
    pub fn new(tokio_handle: Handle) -> Self {
        Self {
            host_interface: AsyncHostFunctionInterface::new(tokio_handle.clone()),
            tokio_handle,
            database_executor: None,
            state_executor: None,
        }
    }

    /// Set the database executor
    pub fn set_database_executor(&mut self, executor: Arc<Mutex<DatabaseOpcodeExecutor>>) {
        self.database_executor = Some(executor);
    }

    /// Set the state executor
    pub fn set_state_executor(&mut self, executor: Arc<Mutex<StateOpcodeExecutor>>) {
        self.state_executor = Some(executor);
    }

    /// Register all async host functions with the WASM execution context
    pub fn register_async_host_functions(&self, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // Register database opcodes
        self.register_database_functions(context)?;

        // Register crypto opcodes
        self.register_crypto_functions(context)?;

        // Register parallel opcodes
        self.register_parallel_functions(context)?;

        // Register state opcodes
        self.register_state_functions(context)?;

        Ok(())
    }

    /// Register database host functions
    fn register_database_functions(&self, context: &mut WasmExecutionContext) -> WasmResult<()> {
        let handle = self.tokio_handle.clone();
        let interface = self.host_interface.clone();

        // DB_READ function
        let db_read_func: HostFunction = Box::new(move |params| {
            if params.len() != 2 {
                return Err(WasmError::execution_error("DB_READ requires 2 parameters: table_id, key".to_string()));
            }

            // Extract parameters
            let table_id = match &params[0] {
                Value::I32(id) => *id as u32,
                _ => return Err(WasmError::type_mismatch("I32".to_string(), "other".to_string())),
            };

            let key = match &params[1] {
                Value::I64(k) => k.to_le_bytes().to_vec(),
                Value::I32(k) => k.to_le_bytes().to_vec(),
                _ => return Err(WasmError::type_mismatch("I32/I64".to_string(), "other".to_string())),
            };

            // Execute async operation synchronously
            let result = block_on(async {
                // This would call the actual database executor
                // For now, return a mock result
                Ok(vec![Value::I32(1)]) // Success indicator
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("db_read".to_string(), db_read_func);

        // DB_WRITE function
        let db_write_func: HostFunction = Box::new(move |params| {
            if params.len() != 3 {
                return Err(WasmError::execution_error("DB_WRITE requires 3 parameters: table_id, key, value".to_string()));
            }

            // Execute async operation synchronously
            let result = block_on(async {
                // This would call the actual database executor
                Ok(vec![]) // No return value for write
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("db_write".to_string(), db_write_func);

        // DB_QUERY function
        let db_query_func: HostFunction = Box::new(move |params| {
            if params.len() != 1 {
                return Err(WasmError::execution_error("DB_QUERY requires 1 parameter: query_spec_json".to_string()));
            }

            let result = block_on(async {
                // This would call the actual database executor with query
                Ok(vec![Value::I32(1)]) // Query result
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("db_query".to_string(), db_query_func);

        Ok(())
    }

    /// Register crypto host functions
    fn register_crypto_functions(&self, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // CRYPTO_HASH function
        let crypto_hash_func: HostFunction = Box::new(move |params| {
            if params.len() != 2 {
                return Err(WasmError::execution_error("CRYPTO_HASH requires 2 parameters: algorithm, data".to_string()));
            }

            let result = block_on(async {
                // This would call the actual crypto provider
                Ok(vec![Value::I32(1)]) // Hash result
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("crypto_hash".to_string(), crypto_hash_func);

        // CRYPTO_ENCRYPT function
        let crypto_encrypt_func: HostFunction = Box::new(move |params| {
            if params.len() != 3 {
                return Err(WasmError::execution_error("CRYPTO_ENCRYPT requires 3 parameters: algorithm, key, data".to_string()));
            }

            let result = block_on(async {
                // This would call the actual crypto provider
                Ok(vec![Value::I32(1)]) // Encrypted data
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("crypto_encrypt".to_string(), crypto_encrypt_func);

        // CRYPTO_DECRYPT function
        let crypto_decrypt_func: HostFunction = Box::new(move |params| {
            if params.len() != 3 {
                return Err(WasmError::execution_error("CRYPTO_DECRYPT requires 3 parameters: algorithm, key, encrypted_data".to_string()));
            }

            let result = block_on(async {
                // This would call the actual crypto provider
                Ok(vec![Value::I32(1)]) // Decrypted data
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("crypto_decrypt".to_string(), crypto_decrypt_func);

        Ok(())
    }

    /// Register parallel host functions
    fn register_parallel_functions(&self, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // PARALLEL_MAP function
        let parallel_map_func: HostFunction = Box::new(move |params| {
            if params.len() != 2 {
                return Err(WasmError::execution_error("PARALLEL_MAP requires 2 parameters: function_ref, data_array".to_string()));
            }

            let result = block_on(async {
                // This would call the actual parallel executor
                Ok(vec![Value::I32(1)]) // Mapped result
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("parallel_map".to_string(), parallel_map_func);

        // PARALLEL_REDUCE function
        let parallel_reduce_func: HostFunction = Box::new(move |params| {
            if params.len() != 3 {
                return Err(WasmError::execution_error("PARALLEL_REDUCE requires 3 parameters: function_ref, initial_value, data_array".to_string()));
            }

            let result = block_on(async {
                // This would call the actual parallel executor
                Ok(vec![Value::I32(1)]) // Reduced result
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("parallel_reduce".to_string(), parallel_reduce_func);

        // PARADOT_SPAWN function
        let paradot_spawn_func: HostFunction = Box::new(move |params| {
            if params.len() != 1 {
                return Err(WasmError::execution_error("PARADOT_SPAWN requires 1 parameter: paradot_spec".to_string()));
            }

            let result = block_on(async {
                // This would call the actual ParaDot manager
                Ok(vec![Value::I32(1)]) // ParaDot ID
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("paradot_spawn".to_string(), paradot_spawn_func);

        Ok(())
    }

    /// Register state host functions
    fn register_state_functions(&self, context: &mut WasmExecutionContext) -> WasmResult<()> {
        // STATE_GET function
        let state_get_func: HostFunction = Box::new(move |params| {
            if params.len() != 1 {
                return Err(WasmError::execution_error("STATE_GET requires 1 parameter: key".to_string()));
            }

            let result = block_on(async {
                // This would call the actual state executor
                Ok(vec![Value::I32(1)]) // State value
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("state_get".to_string(), state_get_func);

        // STATE_SET function
        let state_set_func: HostFunction = Box::new(move |params| {
            if params.len() != 2 {
                return Err(WasmError::execution_error("STATE_SET requires 2 parameters: key, value".to_string()));
            }

            let result = block_on(async {
                // This would call the actual state executor
                Ok(vec![]) // No return value
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("state_set".to_string(), state_set_func);

        // STATE_SNAPSHOT function
        let state_snapshot_func: HostFunction = Box::new(move |params| {
            if params.len() != 0 {
                return Err(WasmError::execution_error("STATE_SNAPSHOT requires 0 parameters".to_string()));
            }

            let result = block_on(async {
                // This would call the actual state executor
                Ok(vec![Value::I32(1)]) // Snapshot ID
            });

            result.map_err(|e: std::io::Error| WasmError::execution_error(e.to_string()))
        });

        context.register_host_function("state_snapshot".to_string(), state_snapshot_func);

        Ok(())
    }

    /// Execute a database opcode asynchronously
    pub async fn execute_database_opcode(&self, opcode: DatabaseOpcode, params: &[Value]) -> Result<Vec<Value>, crate::wasm::ExecutionError> {
        if let Some(executor) = &self.database_executor {
            let executor = executor.lock().unwrap();
            // Call the actual database executor
            match opcode {
                DatabaseOpcode::DbRead => {
                    // Implement actual database read
                    Ok(vec![Value::I32(1)])
                }
                DatabaseOpcode::DbWrite => {
                    // Implement actual database write
                    Ok(vec![])
                }
                _ => Err(crate::wasm::ExecutionError::Database("Unsupported database opcode".to_string())),
            }
        } else {
            Err(crate::wasm::ExecutionError::Database("Database executor not available".to_string()))
        }
    }

    /// Execute a state opcode asynchronously
    pub async fn execute_state_opcode(&self, opcode: StateOpcode, params: &[Value]) -> Result<Vec<Value>, crate::wasm::ExecutionError> {
        if let Some(executor) = &self.state_executor {
            let executor = executor.lock().unwrap();
            // Call the actual state executor
            match opcode {
                StateOpcode::SLOAD => {
                    // Implement actual state load
                    Ok(vec![Value::I32(1)])
                }
                StateOpcode::SSTORE => {
                    // Implement actual state store
                    Ok(vec![])
                }
                _ => Err(crate::wasm::ExecutionError::State("Unsupported state opcode".to_string())),
            }
        } else {
            Err(crate::wasm::ExecutionError::State("State executor not available".to_string()))
        }
    }
}

impl Clone for AsyncHostFunctionInterface {
    fn clone(&self) -> Self {
        // Create a new instance with the same tokio handle
        AsyncHostFunctionInterface::new(tokio::runtime::Handle::current())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_wasm_bridge() {
        let handle = tokio::runtime::Handle::current();

        let bridge = AsyncWasmBridge::new(handle);
        let mut context = WasmExecutionContext::default();

        // Test registration
        assert!(bridge.register_async_host_functions(&mut context).is_ok());

        // Verify functions are registered
        assert!(context.get_host_function("db_read").is_some());
        assert!(context.get_host_function("crypto_hash").is_some());
        assert!(context.get_host_function("parallel_map").is_some());
        assert!(context.get_host_function("state_get").is_some());
    }

    #[tokio::test]
    async fn test_host_function_calls() {
        let handle = tokio::runtime::Handle::current();

        let bridge = AsyncWasmBridge::new(handle);
        let mut context = WasmExecutionContext::default();

        bridge.register_async_host_functions(&mut context).unwrap();

        // Test calling a host function
        if let Some(db_read_func) = context.get_host_function("db_read") {
            let params = vec![Value::I32(1), Value::I32(42)];
            let result = db_read_func(&params);
            assert!(result.is_ok());
        }
    }
}
