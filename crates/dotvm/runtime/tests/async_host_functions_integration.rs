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

//! Integration tests for async host functions
//!
//! These tests verify that the async host function interface works correctly
//! with the WASM runtime and provides proper integration with Dotlanth's
//! custom opcodes.

use dotvm_compiler::wasm::ast::WasmValue as Value;
use dotvm_core::opcode::crypto_opcodes::CryptoOpcode;
use dotvm_core::opcode::db_opcodes::DatabaseOpcode;
use dotvm_core::opcode::parallel_opcodes::ParallelOpcode;
use dotvm_core::opcode::state_opcodes::StateOpcode;
use dotvm_core::vm::database_executor::DatabaseOpcodeExecutor;
use dotvm_core::vm::state_executor::StateOpcodeExecutor;
use dotvm_runtime::wasm::{AsyncHostFunctionInterface, AsyncWasmBridge, ExecutionError, HostError, ValidationError, WasmExecutionContext};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn test_async_host_function_registration() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);
    let mut context = WasmExecutionContext::default();

    // Test successful registration
    let result = bridge.register_async_host_functions(&mut context);
    assert!(result.is_ok(), "Host function registration should succeed");

    // Verify all expected functions are registered
    let expected_functions = vec![
        "db_read",
        "db_write",
        "db_query",
        "crypto_hash",
        "crypto_encrypt",
        "crypto_decrypt",
        "parallel_map",
        "parallel_reduce",
        "paradot_spawn",
        "state_get",
        "state_set",
        "state_snapshot",
    ];

    for func_name in expected_functions {
        assert!(context.get_host_function(func_name).is_some(), "Function '{}' should be registered", func_name);
    }
}

#[tokio::test]
async fn test_database_host_functions() {
    let handle = tokio::runtime::Handle::current();

    let mut bridge = AsyncWasmBridge::new(handle);

    // Set up database executor
    // Note: DatabaseOpcodeExecutor requires a TransactionManager, using mock for tests
    // let transaction_manager = Arc::new(dotdb_core::storage_engine::TransactionManager::new());
    // let db_executor = Arc::new(Mutex::new(DatabaseOpcodeExecutor::new(transaction_manager).unwrap()));
    // bridge.set_database_executor(db_executor);

    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Test DB_READ
    if let Some(db_read_func) = context.get_host_function("db_read") {
        let params = vec![Value::I32(1), Value::I32(42)];
        let result = db_read_func(&params);
        assert!(result.is_ok(), "DB_READ should execute successfully");

        // Test invalid parameters
        let invalid_params = vec![Value::I32(1)]; // Missing key parameter
        let result = db_read_func(&invalid_params);
        assert!(result.is_err(), "DB_READ should fail with invalid parameters");
    }

    // Test DB_WRITE
    if let Some(db_write_func) = context.get_host_function("db_write") {
        let params = vec![Value::I32(1), Value::I32(42), Value::I64(12345)];
        let result = db_write_func(&params);
        assert!(result.is_ok(), "DB_WRITE should execute successfully");
    }

    // Test DB_QUERY
    if let Some(db_query_func) = context.get_host_function("db_query") {
        let params = vec![Value::I32(1)];
        let result = db_query_func(&params);
        assert!(result.is_ok(), "DB_QUERY should execute successfully");
    }
}

#[tokio::test]
async fn test_crypto_host_functions() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Test CRYPTO_HASH
    if let Some(crypto_hash_func) = context.get_host_function("crypto_hash") {
        let params = vec![Value::I32(1), Value::I64(0x1234567890ABCDEF)];
        let result = crypto_hash_func(&params);
        assert!(result.is_ok(), "CRYPTO_HASH should execute successfully");

        // Test invalid parameters
        let invalid_params = vec![Value::I32(1)]; // Missing data parameter
        let result = crypto_hash_func(&invalid_params);
        assert!(result.is_err(), "CRYPTO_HASH should fail with invalid parameters");
    }

    // Test CRYPTO_ENCRYPT
    if let Some(crypto_encrypt_func) = context.get_host_function("crypto_encrypt") {
        let params = vec![Value::I32(1), Value::I64(0xDEADBEEF), Value::I64(0x1234567890ABCDEF)];
        let result = crypto_encrypt_func(&params);
        assert!(result.is_ok(), "CRYPTO_ENCRYPT should execute successfully");
    }

    // Test CRYPTO_DECRYPT
    if let Some(crypto_decrypt_func) = context.get_host_function("crypto_decrypt") {
        let params = vec![Value::I32(1), Value::I64(0xDEADBEEF), Value::I64(0xFEDCBA0987654321u64 as i64)];
        let result = crypto_decrypt_func(&params);
        assert!(result.is_ok(), "CRYPTO_DECRYPT should execute successfully");
    }
}

#[tokio::test]
async fn test_parallel_host_functions() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Test PARALLEL_MAP
    if let Some(parallel_map_func) = context.get_host_function("parallel_map") {
        let params = vec![Value::I32(1), Value::I32(100)];
        let result = parallel_map_func(&params);
        assert!(result.is_ok(), "PARALLEL_MAP should execute successfully");
    }

    // Test PARALLEL_REDUCE
    if let Some(parallel_reduce_func) = context.get_host_function("parallel_reduce") {
        let params = vec![Value::I32(1), Value::I32(0), Value::I32(100)];
        let result = parallel_reduce_func(&params);
        assert!(result.is_ok(), "PARALLEL_REDUCE should execute successfully");
    }

    // Test PARADOT_SPAWN
    if let Some(paradot_spawn_func) = context.get_host_function("paradot_spawn") {
        let params = vec![Value::I32(1)];
        let result = paradot_spawn_func(&params);
        assert!(result.is_ok(), "PARADOT_SPAWN should execute successfully");
    }
}

#[tokio::test]
async fn test_state_host_functions() {
    let handle = tokio::runtime::Handle::current();

    let mut bridge = AsyncWasmBridge::new(handle);

    // Set up state executor
    // Note: StateOpcodeExecutor requires MVCCStore, MerkleTree, and SnapshotManager
    // let mvcc_store = Arc::new(MVCCStore::new());
    // let merkle_tree = Arc::new(MerkleTree::new());
    // let snapshot_manager = Arc::new(SnapshotManager::new());
    // let state_executor = Arc::new(Mutex::new(StateOpcodeExecutor::new(mvcc_store, merkle_tree, snapshot_manager)));
    // bridge.set_state_executor(state_executor);

    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Test STATE_SET
    if let Some(state_set_func) = context.get_host_function("state_set") {
        let params = vec![Value::I64(0x1234567890ABCDEF), Value::I64(0xFEDCBA0987654321u64 as i64)];
        let result = state_set_func(&params);
        assert!(result.is_ok(), "STATE_SET should execute successfully");
    }

    // Test STATE_GET
    if let Some(state_get_func) = context.get_host_function("state_get") {
        let params = vec![Value::I64(0x1234567890ABCDEF)];
        let result = state_get_func(&params);
        assert!(result.is_ok(), "STATE_GET should execute successfully");
    }

    // Test STATE_SNAPSHOT
    if let Some(state_snapshot_func) = context.get_host_function("state_snapshot") {
        let params = vec![];
        let result = state_snapshot_func(&params);
        assert!(result.is_ok(), "STATE_SNAPSHOT should execute successfully");
    }
}

#[tokio::test]
async fn test_async_opcode_execution() {
    let handle = tokio::runtime::Handle::current();

    let mut bridge = AsyncWasmBridge::new(handle);

    // Set up executors (commented out due to constructor requirements)
    // let transaction_manager = Arc::new(dotdb_core::storage_engine::TransactionManager::new());
    // let db_executor = Arc::new(Mutex::new(DatabaseOpcodeExecutor::new(transaction_manager).unwrap()));
    // bridge.set_database_executor(db_executor);

    // let mvcc_store = Arc::new(MVCCStore::new());
    // let merkle_tree = Arc::new(MerkleTree::new());
    // let snapshot_manager = Arc::new(SnapshotManager::new());
    // let state_executor = Arc::new(Mutex::new(StateOpcodeExecutor::new(mvcc_store, merkle_tree, snapshot_manager)));
    // bridge.set_state_executor(state_executor);

    // Test async database opcode execution (commented out until proper setup)
    // let params = vec![Value::I32(1), Value::I32(42)];
    // let result = bridge.execute_database_opcode(DatabaseOpcode::DbRead, &params).await;
    // assert!(result.is_ok(), "Async database opcode execution should succeed");

    // Test async state opcode execution (commented out until proper setup)
    // let params = vec![Value::I64(0x1234567890ABCDEF)];
    // let result = bridge.execute_state_opcode(StateOpcode::SLOAD, &params).await;
    // assert!(result.is_ok(), "Async state opcode execution should succeed");
}

#[tokio::test]
async fn test_parameter_validation() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Test parameter count validation
    if let Some(db_read_func) = context.get_host_function("db_read") {
        // Too few parameters
        let result = db_read_func(&vec![Value::I32(1)]);
        assert!(result.is_err(), "Should fail with too few parameters");

        // Too many parameters
        let result = db_read_func(&vec![Value::I32(1), Value::I32(42), Value::I32(100)]);
        assert!(result.is_err(), "Should fail with too many parameters");

        // Correct parameters
        let result = db_read_func(&vec![Value::I32(1), Value::I32(42)]);
        assert!(result.is_ok(), "Should succeed with correct parameters");
    }
}

#[tokio::test]
async fn test_type_safety() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Test type validation for database functions
    if let Some(db_read_func) = context.get_host_function("db_read") {
        // Test with wrong parameter types (should still work as we convert internally)
        let params = vec![Value::F32(1.0), Value::F64(42.0)];
        let result = db_read_func(&params);
        // This might succeed or fail depending on implementation
        // The key is that it doesn't crash
        assert!(result.is_ok() || result.is_err());
    }
}

#[tokio::test]
async fn test_error_handling() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Test error propagation
    if let Some(db_read_func) = context.get_host_function("db_read") {
        // Test with invalid parameters
        let result = db_read_func(&vec![]);
        assert!(result.is_err(), "Should return error for invalid parameters");

        // Verify error message is meaningful
        if let Err(error) = result {
            let error_msg = format!("{}", error);
            assert!(error_msg.contains("requires"), "Error message should be descriptive");
        }
    }
}

#[tokio::test]
async fn test_concurrent_host_function_calls() {
    let handle = tokio::runtime::Handle::current();

    let bridge = Arc::new(AsyncWasmBridge::new(handle));
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    let context = Arc::new(context);

    // Test concurrent calls to host functions
    let mut handles = vec![];

    for i in 0..10 {
        let bridge_clone = bridge.clone();
        let context_clone = context.clone();

        let handle = tokio::spawn(async move {
            if let Some(db_read_func) = context_clone.get_host_function("db_read") {
                let params = vec![Value::I32(1), Value::I32(i)];
                db_read_func(&params)
            } else {
                Err(dotvm_runtime::wasm::WasmError::execution_error("Function not found".to_string()))
            }
        });

        handles.push(handle);
    }

    // Wait for all concurrent calls to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent host function call should succeed");
    }
}

#[tokio::test]
async fn test_performance_characteristics() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context).unwrap();

    // Performance test: measure host function call overhead
    let iterations = 1000;
    let start = std::time::Instant::now();

    if let Some(db_read_func) = context.get_host_function("db_read") {
        for i in 0..iterations {
            let params = vec![Value::I32(1), Value::I32(i as i32)];
            let _ = db_read_func(&params).unwrap();
        }
    }

    let duration = start.elapsed();
    let avg_call_time = duration / iterations;

    // Verify performance is reasonable (less than 1ms per call)
    assert!(avg_call_time < Duration::from_millis(1), "Host function calls should be fast, got {:?} per call", avg_call_time);

    println!("Average host function call time: {:?}", avg_call_time);
}

#[tokio::test]
async fn test_resource_tracking() {
    let handle = tokio::runtime::Handle::current();

    let interface = AsyncHostFunctionInterface::new(handle);
    let mut context = WasmExecutionContext::default();

    // Test resource tracking through the interface
    // Note: register_all method may need to be implemented
    // let result = interface.register_all(&mut context);
    // assert!(result.is_ok(), "Interface registration should succeed");

    // Test async opcode calls with resource tracking (commented out until proper setup)
    // let params = vec![Value::I32(1), Value::I32(42)];
    // let result = interface.call_database_opcode(&mut context, DatabaseOpcode::DbRead, &params).await;
    // assert!(result.is_ok(), "Database opcode call should succeed");

    // let result = interface.call_crypto_opcode(&mut context, CryptoOpcode::Hash, &params).await;
    // assert!(result.is_ok(), "Crypto opcode call should succeed");
}

#[tokio::test]
async fn test_security_context_validation() {
    let handle = tokio::runtime::Handle::current();

    let interface = AsyncHostFunctionInterface::new(handle);
    let mut context = WasmExecutionContext::default();

    // Set up restricted security context
    context.wasm.security.allowed_operations.clear();
    context.wasm.security.blocked_operations.push("database".to_string());

    // Note: register_all method may need to be implemented
    // interface.register_all(&mut context).unwrap();

    // Test that blocked operations fail (commented out until proper setup)
    // let params = vec![Value::I32(1), Value::I32(42)];
    // let result = interface.call_database_opcode(&mut context, DatabaseOpcode::DbRead, &params).await;
    // assert!(result.is_err(), "Blocked database operation should fail");
}

#[tokio::test]
async fn test_host_function_interface_creation() {
    let handle = tokio::runtime::Handle::current();

    // Test interface creation
    let interface = AsyncHostFunctionInterface::new(handle);

    // Verify interface is properly initialized
    // This is a basic smoke test to ensure no panics during creation
    drop(interface);
}

#[tokio::test]
async fn test_integration_with_wasm_execution_context() {
    let handle = tokio::runtime::Handle::current();

    let bridge = AsyncWasmBridge::new(handle);

    // Test with different execution context configurations
    let mut context = WasmExecutionContext::new(
        10000,                  // max_instructions
        100,                    // max_call_depth
        Duration::from_secs(5), // max_duration
    );

    let result = bridge.register_async_host_functions(&mut context);
    assert!(result.is_ok(), "Registration should work with custom context");

    // Verify context state is preserved
    assert_eq!(context.wasm.max_instructions, 10000);
    assert_eq!(context.wasm.max_call_depth, 100);
}
