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

//! Async Host Functions Demo
//!
//! This example demonstrates how to use the async host function interface
//! to call Dotlanth's custom opcodes from WASM code efficiently and securely.

use dotvm_compiler::wasm::ast::WasmValue as Value;
use dotvm_core::vm::database_executor::DatabaseOpcodeExecutor;
use dotvm_core::vm::state_executor::StateOpcodeExecutor;
use dotvm_runtime::wasm::{AsyncWasmBridge, DotVMWasmRuntime, ExecutionConfig, MemoryConfig, SecurityConfig, WasmExecutionContext, WasmInstance, WasmModule, WasmResult, WasmRuntimeConfig};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Dotlanth Async Host Functions Demo");

    // Initialize Tokio runtime
    let rt = Runtime::new()?;
    let handle = rt.handle().clone();

    // Create async WASM bridge
    let mut bridge = AsyncWasmBridge::new(handle);

    // Set up database executor (commented out for demo - requires proper setup)
    // let transaction_manager = Arc::new(dotdb_core::storage_engine::TransactionManager::new());
    // let db_executor = Arc::new(Mutex::new(DatabaseOpcodeExecutor::new(transaction_manager).unwrap()));
    // bridge.set_database_executor(db_executor);

    // Set up state executor (commented out for demo - requires proper setup)
    // let mvcc_store = Arc::new(MVCCStore::new());
    // let merkle_tree = Arc::new(MerkleTree::new());
    // let snapshot_manager = Arc::new(SnapshotManager::new());
    // let state_executor = Arc::new(Mutex::new(StateOpcodeExecutor::new(mvcc_store, merkle_tree, snapshot_manager)));
    // bridge.set_state_executor(state_executor);

    // Create WASM execution context
    let mut context = WasmExecutionContext::default();

    // Register async host functions
    bridge.register_async_host_functions(&mut context)?;

    println!("✅ Async host functions registered successfully");

    // Demonstrate host function calls
    demonstrate_database_operations(&context).await?;
    demonstrate_crypto_operations(&context).await?;
    demonstrate_parallel_operations(&context).await?;
    demonstrate_state_operations(&context).await?;

    println!("🎉 Demo completed successfully!");

    Ok(())
}

/// Demonstrate database operations through host functions
async fn demonstrate_database_operations(context: &WasmExecutionContext) -> WasmResult<()> {
    println!("\n📊 Database Operations Demo");

    // Test DB_READ
    if let Some(db_read_func) = context.get_host_function("db_read") {
        let params = vec![Value::I32(1), Value::I32(42)]; // table_id=1, key=42
        let result = db_read_func(&params)?;
        println!("  DB_READ(table=1, key=42) -> {:?}", result);
    }

    // Test DB_WRITE
    if let Some(db_write_func) = context.get_host_function("db_write") {
        let params = vec![Value::I32(1), Value::I32(42), Value::I64(12345)]; // table_id=1, key=42, value=12345
        let result = db_write_func(&params)?;
        println!("  DB_WRITE(table=1, key=42, value=12345) -> {:?}", result);
    }

    // Test DB_QUERY
    if let Some(db_query_func) = context.get_host_function("db_query") {
        let params = vec![Value::I32(1)]; // query_spec placeholder
        let result = db_query_func(&params)?;
        println!("  DB_QUERY(spec=1) -> {:?}", result);
    }

    Ok(())
}

/// Demonstrate crypto operations through host functions
async fn demonstrate_crypto_operations(context: &WasmExecutionContext) -> WasmResult<()> {
    println!("\n🔐 Crypto Operations Demo");

    // Test CRYPTO_HASH
    if let Some(crypto_hash_func) = context.get_host_function("crypto_hash") {
        let params = vec![Value::I32(1), Value::I64(0x1234567890ABCDEF)]; // algorithm=1, data=0x1234567890ABCDEF
        let result = crypto_hash_func(&params)?;
        println!("  CRYPTO_HASH(algo=1, data=0x1234567890ABCDEF) -> {:?}", result);
    }

    // Test CRYPTO_ENCRYPT
    if let Some(crypto_encrypt_func) = context.get_host_function("crypto_encrypt") {
        let params = vec![Value::I32(1), Value::I64(0xDEADBEEF), Value::I64(0x1234567890ABCDEF)]; // algorithm=1, key=0xDEADBEEF, data=0x1234567890ABCDEF
        let result = crypto_encrypt_func(&params)?;
        println!("  CRYPTO_ENCRYPT(algo=1, key=0xDEADBEEF, data=0x1234567890ABCDEF) -> {:?}", result);
    }

    // Test CRYPTO_DECRYPT
    if let Some(crypto_decrypt_func) = context.get_host_function("crypto_decrypt") {
        let params = vec![Value::I32(1), Value::I64(0xDEADBEEF), Value::I64(0xFEDCBA0987654321u64 as i64)]; // algorithm=1, key=0xDEADBEEF, encrypted_data=0xFEDCBA0987654321
        let result = crypto_decrypt_func(&params)?;
        println!("  CRYPTO_DECRYPT(algo=1, key=0xDEADBEEF, encrypted=0xFEDCBA0987654321) -> {:?}", result);
    }

    Ok(())
}

/// Demonstrate parallel operations through host functions
async fn demonstrate_parallel_operations(context: &WasmExecutionContext) -> WasmResult<()> {
    println!("\n⚡ Parallel Operations Demo");

    // Test PARALLEL_MAP
    if let Some(parallel_map_func) = context.get_host_function("parallel_map") {
        let params = vec![Value::I32(1), Value::I32(100)]; // function_ref=1, data_array=100
        let result = parallel_map_func(&params)?;
        println!("  PARALLEL_MAP(func=1, data=100) -> {:?}", result);
    }

    // Test PARALLEL_REDUCE
    if let Some(parallel_reduce_func) = context.get_host_function("parallel_reduce") {
        let params = vec![Value::I32(1), Value::I32(0), Value::I32(100)]; // function_ref=1, initial=0, data_array=100
        let result = parallel_reduce_func(&params)?;
        println!("  PARALLEL_REDUCE(func=1, initial=0, data=100) -> {:?}", result);
    }

    // Test PARADOT_SPAWN
    if let Some(paradot_spawn_func) = context.get_host_function("paradot_spawn") {
        let params = vec![Value::I32(1)]; // paradot_spec=1
        let result = paradot_spawn_func(&params)?;
        println!("  PARADOT_SPAWN(spec=1) -> {:?}", result);
    }

    Ok(())
}

/// Demonstrate state operations through host functions
async fn demonstrate_state_operations(context: &WasmExecutionContext) -> WasmResult<()> {
    println!("\n🌳 State Operations Demo");

    // Test SSTORE (state store)
    if let Some(state_set_func) = context.get_host_function("state_set") {
        let params = vec![Value::I64(0x1234567890ABCDEF), Value::I64(0xFEDCBA0987654321u64 as i64)]; // key=0x1234567890ABCDEF, value=0xFEDCBA0987654321
        let result = state_set_func(&params)?;
        println!("  SSTORE(key=0x1234567890ABCDEF, value=0xFEDCBA0987654321) -> {:?}", result);
    }

    // Test SLOAD (state load)
    if let Some(state_get_func) = context.get_host_function("state_get") {
        let params = vec![Value::I64(0x1234567890ABCDEF)]; // key=0x1234567890ABCDEF
        let result = state_get_func(&params)?;
        println!("  SLOAD(key=0x1234567890ABCDEF) -> {:?}", result);
    }

    // Test STATE_SNAPSHOT
    if let Some(state_snapshot_func) = context.get_host_function("state_snapshot") {
        let params = vec![]; // no parameters
        let result = state_snapshot_func(&params)?;
        println!("  STATE_SNAPSHOT() -> {:?}", result);
    }

    Ok(())
}

/// Example of creating a complete WASM runtime with async host functions
async fn create_complete_runtime_example() -> WasmResult<()> {
    println!("\n🏗️  Complete Runtime Example");

    // Create runtime configuration
    let config = WasmRuntimeConfig {
        max_memory_pages: 1024,
        max_table_elements: 10000,
        max_instances: 100,
        max_stack_size: 1024 * 1024,
        max_execution_time: Duration::from_secs(30),
        enable_jit: true,
        enable_optimizations: true,
        strict_validation: false,
        enable_debugging: true,
        enable_monitoring: true,
        enable_custom_opcodes: Some(true),
        max_call_depth: 1000,
        max_locals_per_function: 1024,
        max_globals: 1000,
        max_imports: 1000,
        max_exports: 1000,
        max_module_size: 10 * 1024 * 1024,
        security: SecurityConfig {
            enable_sandbox: true,
            allowed_syscalls: vec![],
            blocked_imports: vec![],
            enable_memory_protection: true,
            enable_stack_protection: true,
            max_file_descriptors: 10,
            allow_network_access: false,
            allow_filesystem_access: false,
        },
        memory: MemoryConfig {
            initial_pages: 1,
            max_pages: 100,
            allow_memory_growth: true,
            alignment: 8,
            enable_protection: true,
            pool_size: 10,
        },
        execution: ExecutionConfig {
            enable_instruction_counting: true,
            enable_tracing: true,
            max_instructions: 1_000_000,
        },
    };

    // Create WASM runtime
    let runtime = DotVMWasmRuntime::new(config);

    // Set up async bridge
    let handle = tokio::runtime::Handle::current();
    let bridge = AsyncWasmBridge::new(handle);

    // Register async host functions with runtime
    let mut context = WasmExecutionContext::default();
    bridge.register_async_host_functions(&mut context)?;

    println!("  ✅ Complete runtime with async host functions created");

    Ok(())
}

/// Performance benchmark for host function calls
async fn benchmark_host_functions(context: &WasmExecutionContext) -> WasmResult<()> {
    println!("\n⏱️  Performance Benchmark");

    let iterations = 1000;
    let start = std::time::Instant::now();

    // Benchmark database operations
    if let Some(db_read_func) = context.get_host_function("db_read") {
        for i in 0..iterations {
            let params = vec![Value::I32(1), Value::I32(i as i32)];
            let _ = db_read_func(&params)?;
        }
    }

    let duration = start.elapsed();
    let ops_per_sec = iterations as f64 / duration.as_secs_f64();

    println!("  📊 Performed {} database operations in {:?}", iterations, duration);
    println!("  🚀 Performance: {:.2} operations/second", ops_per_sec);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_host_functions_integration() {
        let rt = Runtime::new().unwrap();
        let handle = rt.handle().clone();

        let bridge = AsyncWasmBridge::new(handle);
        let mut context = WasmExecutionContext::default();

        // Test registration
        assert!(bridge.register_async_host_functions(&mut context).is_ok());

        // Test that all expected functions are registered
        assert!(context.get_host_function("db_read").is_some());
        assert!(context.get_host_function("db_write").is_some());
        assert!(context.get_host_function("crypto_hash").is_some());
        assert!(context.get_host_function("parallel_map").is_some());
        assert!(context.get_host_function("state_get").is_some());
    }

    #[tokio::test]
    async fn test_host_function_parameter_validation() {
        let rt = Runtime::new().unwrap();
        let handle = rt.handle().clone();

        let bridge = AsyncWasmBridge::new(handle);
        let mut context = WasmExecutionContext::default();
        bridge.register_async_host_functions(&mut context).unwrap();

        // Test parameter validation
        if let Some(db_read_func) = context.get_host_function("db_read") {
            // Valid parameters
            let valid_params = vec![Value::I32(1), Value::I32(42)];
            assert!(db_read_func(&valid_params).is_ok());

            // Invalid parameter count
            let invalid_params = vec![Value::I32(1)];
            assert!(db_read_func(&invalid_params).is_err());
        }
    }

    #[tokio::test]
    async fn test_async_execution_flow() {
        // Test that async operations can be called from host functions
        let rt = Runtime::new().unwrap();
        let handle = rt.handle().clone();

        let bridge = AsyncWasmBridge::new(handle);

        // Test async database operation
        let params = vec![Value::I32(1), Value::I32(42)];
        let result = bridge.execute_database_opcode(dotvm_core::opcode::DatabaseOpcode::DbRead, &params).await;

        assert!(result.is_ok());
    }
}
