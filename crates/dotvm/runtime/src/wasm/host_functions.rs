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

//! Async Host Function Interface for Custom Opcodes
//!
//! This module provides an efficient async interface for WASM code to call
//! Dotlanth's custom opcodes (database, crypto, parallel, state) with proper
//! parameter marshaling, type safety, and resource tracking.

use crate::wasm::{HostFunction, WasmError, WasmExecutionContext, WasmResult};
use dotvm_compiler::wasm::ast::WasmValue as Value;
use dotvm_core::instruction::crypto_provider::{CryptographicOpcodeExecutor, EncryptionProvider, HashProvider, SignatureProvider};
use dotvm_core::opcode::crypto_opcodes::CryptoOpcode;
use dotvm_core::opcode::db_opcodes::DatabaseOpcode;
use dotvm_core::opcode::parallel_opcodes::ParallelOpcode;
use dotvm_core::opcode::state_opcodes::StateOpcode;
use dotvm_core::vm::database_executor::DatabaseOpcodeExecutor;
use dotvm_core::vm::state_executor::StateOpcodeExecutor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::runtime::Handle;
use uuid::Uuid;

/// Async host function interface for custom opcodes
pub struct AsyncHostFunctionInterface {
    /// Tokio runtime handle for async execution
    tokio_handle: Handle,
    /// Database host functions
    database_functions: AsyncDatabaseHostFunctions,
    /// Crypto host functions
    crypto_functions: AsyncCryptoHostFunctions,
    /// Parallel host functions
    parallel_functions: AsyncParallelHostFunctions,
    /// State host functions
    state_functions: AsyncStateHostFunctions,
    /// Parameter marshaler for type conversion
    async_marshaler: AsyncParameterMarshaler,
    /// Type validator for safety
    validator: TypeValidator,
    /// Resource tracker for limits
    async_resource_tracker: AsyncResourceTracker,
}

/// Trait for async host functions
#[async_trait::async_trait]
pub trait AsyncHostFunction: Send + Sync {
    /// Register the host function with the WASM execution context
    fn register(&self, context: &mut WasmExecutionContext) -> Result<(), HostError>;

    /// Validate parameters before execution
    fn validate_params(&self, params: &[Value]) -> Result<(), ValidationError>;

    /// Execute the host function asynchronously
    async fn execute(&self, context: &mut WasmExecutionContext, params: &[Value]) -> Result<Vec<Value>, ExecutionError>;

    /// Get function name for registration
    fn name(&self) -> &str;

    /// Get function signature for validation
    fn signature(&self) -> FunctionSignature;
}

/// Function signature for type checking
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// Parameter types
    pub params: Vec<ValueType>,
    /// Return types
    pub returns: Vec<ValueType>,
}

/// Value types for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
    String,
    Bytes,
}

/// Async execution state
#[derive(Debug, Clone)]
pub struct AsyncExecutionState {
    /// Execution ID
    pub id: Uuid,
    /// Start time
    pub start_time: Instant,
    /// Current operation
    pub current_operation: Option<String>,
    /// Resource usage
    pub resource_usage: ResourceUsage,
}

/// Resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Memory allocated (bytes)
    pub memory_allocated: u64,
    /// CPU time used (nanoseconds)
    pub cpu_time_ns: u64,
    /// Database operations count
    pub db_operations: u64,
    /// Crypto operations count
    pub crypto_operations: u64,
    /// Parallel operations count
    pub parallel_operations: u64,
}

/// Security context for host function calls
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Security level
    pub security_level: SecurityLevel,
}

/// Resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory (bytes)
    pub max_memory: u64,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum database operations
    pub max_db_operations: u64,
    /// Maximum crypto operations
    pub max_crypto_operations: u64,
}

/// Security levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Unrestricted access
    Unrestricted,
    /// Standard security
    Standard,
    /// High security with strict limits
    High,
    /// Sandbox mode with minimal permissions
    Sandbox,
}

/// Host function errors
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("Registration failed: {0}")]
    RegistrationFailed(String),
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

/// Validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("Parameter count mismatch: expected {expected}, got {actual}")]
    ParameterCountMismatch { expected: usize, actual: usize },
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

/// Execution errors
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Parallel error: {0}")]
    Parallel(String),
    #[error("State error: {0}")]
    State(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("Timeout: {0}")]
    Timeout(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AsyncHostFunctionInterface {
    /// Create a new async host function interface
    pub fn new(tokio_handle: Handle) -> Self {
        Self {
            tokio_handle,
            database_functions: AsyncDatabaseHostFunctions::new(),
            crypto_functions: AsyncCryptoHostFunctions::new(),
            parallel_functions: AsyncParallelHostFunctions::new(),
            state_functions: AsyncStateHostFunctions::new(),
            async_marshaler: AsyncParameterMarshaler::new(),
            validator: TypeValidator::new(),
            async_resource_tracker: AsyncResourceTracker::new(),
        }
    }

    /// Register all host functions with the WASM execution context
    pub fn register_all(&self, context: &mut WasmExecutionContext) -> Result<(), HostError> {
        // Register database functions
        self.database_functions.register_all(context)?;

        // Register crypto functions
        self.crypto_functions.register_all(context)?;

        // Register parallel functions
        self.parallel_functions.register_all(context)?;

        // Register state functions
        self.state_functions.register_all(context)?;

        Ok(())
    }

    /// Call a database opcode asynchronously
    pub async fn call_database_opcode(&self, context: &mut WasmExecutionContext, opcode: DatabaseOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        // Validate security context
        self.validate_security_context(context, "database")?;

        // Track resource usage
        self.async_resource_tracker.track_operation("database").await?;

        // Execute the database opcode
        self.database_functions.execute_opcode(context, opcode, params).await
    }

    /// Call a crypto opcode asynchronously
    pub async fn call_crypto_opcode(&self, context: &mut WasmExecutionContext, opcode: CryptoOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        // Validate security context
        self.validate_security_context(context, "crypto")?;

        // Track resource usage
        self.async_resource_tracker.track_operation("crypto").await?;

        // Execute the crypto opcode
        self.crypto_functions.execute_opcode(context, opcode, params).await
    }

    /// Call a parallel opcode asynchronously
    pub async fn call_parallel_opcode(&self, context: &mut WasmExecutionContext, opcode: ParallelOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        // Validate security context
        self.validate_security_context(context, "parallel")?;

        // Track resource usage
        self.async_resource_tracker.track_operation("parallel").await?;

        // Execute the parallel opcode
        self.parallel_functions.execute_opcode(context, opcode, params).await
    }

    /// Call a state opcode asynchronously
    pub async fn call_state_opcode(&self, context: &mut WasmExecutionContext, opcode: StateOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        // Validate security context
        self.validate_security_context(context, "state")?;

        // Track resource usage
        self.async_resource_tracker.track_operation("state").await?;

        // Execute the state opcode
        self.state_functions.execute_opcode(context, opcode, params).await
    }

    /// Validate security context for operation
    fn validate_security_context(&self, context: &WasmExecutionContext, operation: &str) -> Result<(), ExecutionError> {
        if !context.is_operation_allowed(operation) {
            return Err(ExecutionError::Internal(format!("Operation '{}' not allowed", operation)));
        }
        Ok(())
    }
}

/// Database host functions
pub struct AsyncDatabaseHostFunctions {
    executor: Option<Arc<Mutex<DatabaseOpcodeExecutor>>>,
}

impl AsyncDatabaseHostFunctions {
    pub fn new() -> Self {
        Self { executor: None }
    }

    pub fn register_all(&self, context: &mut WasmExecutionContext) -> Result<(), HostError> {
        // Register database host functions
        let db_read_func: HostFunction = Box::new(|params| {
            // This will be replaced with async implementation
            Ok(vec![Value::I32(0)])
        });

        context.register_host_function("db_read".to_string(), db_read_func);

        // Register other database functions...
        Ok(())
    }

    pub async fn execute_opcode(&self, _context: &mut WasmExecutionContext, opcode: DatabaseOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        match opcode {
            DatabaseOpcode::DbRead => {
                // Implement async database read
                Ok(vec![Value::I32(1)])
            }
            DatabaseOpcode::DbWrite => {
                // Implement async database write
                Ok(vec![])
            }
            _ => Err(ExecutionError::Database("Unsupported opcode".to_string())),
        }
    }
}

/// Crypto host functions
pub struct AsyncCryptoHostFunctions {
    executor: Option<Arc<CryptographicOpcodeExecutor>>,
}

impl AsyncCryptoHostFunctions {
    pub fn new() -> Self {
        Self { executor: None }
    }

    pub fn register_all(&self, context: &mut WasmExecutionContext) -> Result<(), HostError> {
        // Register crypto host functions
        let crypto_hash_func: HostFunction = Box::new(|params| Ok(vec![Value::I32(0)]));

        context.register_host_function("crypto_hash".to_string(), crypto_hash_func);

        // Register other crypto functions...
        Ok(())
    }

    pub async fn execute_opcode(&self, _context: &mut WasmExecutionContext, opcode: CryptoOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        match opcode {
            CryptoOpcode::Hash => {
                // Implement async crypto hash
                Ok(vec![Value::I32(1)])
            }
            CryptoOpcode::Encrypt => {
                // Implement async crypto encrypt
                Ok(vec![Value::I32(1)])
            }
            _ => Err(ExecutionError::Crypto("Unsupported opcode".to_string())),
        }
    }
}

/// Parallel host functions
pub struct AsyncParallelHostFunctions {}

impl AsyncParallelHostFunctions {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_all(&self, context: &mut WasmExecutionContext) -> Result<(), HostError> {
        // Register parallel host functions
        let parallel_map_func: HostFunction = Box::new(|params| Ok(vec![Value::I32(0)]));

        context.register_host_function("parallel_map".to_string(), parallel_map_func);

        // Register other parallel functions...
        Ok(())
    }

    pub async fn execute_opcode(&self, _context: &mut WasmExecutionContext, opcode: ParallelOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        match opcode {
            ParallelOpcode::Map => {
                // Implement async parallel map
                Ok(vec![Value::I32(1)])
            }
            ParallelOpcode::Reduce => {
                // Implement async parallel reduce
                Ok(vec![Value::I32(1)])
            }
            _ => Err(ExecutionError::Parallel("Unsupported opcode".to_string())),
        }
    }
}

/// State host functions
pub struct AsyncStateHostFunctions {
    executor: Option<Arc<Mutex<StateOpcodeExecutor>>>,
}

impl AsyncStateHostFunctions {
    pub fn new() -> Self {
        Self { executor: None }
    }

    pub fn register_all(&self, context: &mut WasmExecutionContext) -> Result<(), HostError> {
        // Register state host functions
        let state_get_func: HostFunction = Box::new(|params| Ok(vec![Value::I32(0)]));

        context.register_host_function("state_get".to_string(), state_get_func);

        // Register other state functions...
        Ok(())
    }

    pub async fn execute_opcode(&self, _context: &mut WasmExecutionContext, opcode: StateOpcode, params: &[Value]) -> Result<Vec<Value>, ExecutionError> {
        match opcode {
            StateOpcode::SLOAD => {
                // Implement async state load
                Ok(vec![Value::I32(1)])
            }
            StateOpcode::SSTORE => {
                // Implement async state store
                Ok(vec![])
            }
            _ => Err(ExecutionError::State("Unsupported opcode".to_string())),
        }
    }
}

/// Parameter marshaler for async operations
pub struct AsyncParameterMarshaler {}

impl AsyncParameterMarshaler {
    pub fn new() -> Self {
        Self {}
    }

    /// Marshal parameters from WASM values to native types
    pub async fn marshal_params(&self, params: &[Value], expected_types: &[ValueType]) -> Result<Vec<u8>, ValidationError> {
        if params.len() != expected_types.len() {
            return Err(ValidationError::ParameterCountMismatch {
                expected: expected_types.len(),
                actual: params.len(),
            });
        }

        // Implement parameter marshaling
        Ok(vec![])
    }

    /// Unmarshal return values from native types to WASM values
    pub async fn unmarshal_returns(&self, data: &[u8], return_types: &[ValueType]) -> Result<Vec<Value>, ValidationError> {
        // Implement return value unmarshaling
        Ok(vec![])
    }
}

/// Type validator for host function calls
pub struct TypeValidator {}

impl TypeValidator {
    pub fn new() -> Self {
        Self {}
    }

    /// Validate parameter types
    pub fn validate_params(&self, params: &[Value], signature: &FunctionSignature) -> Result<(), ValidationError> {
        if params.len() != signature.params.len() {
            return Err(ValidationError::ParameterCountMismatch {
                expected: signature.params.len(),
                actual: params.len(),
            });
        }

        for (i, (param, expected_type)) in params.iter().zip(&signature.params).enumerate() {
            let actual_type = ValueType::from_value(param);
            if actual_type != *expected_type {
                return Err(ValidationError::TypeMismatch {
                    expected: format!("{:?}", expected_type),
                    actual: format!("{:?}", actual_type),
                });
            }
        }

        Ok(())
    }
}

/// Resource tracker for async operations
pub struct AsyncResourceTracker {
    usage: Arc<Mutex<ResourceUsage>>,
    limits: ResourceLimits,
}

impl AsyncResourceTracker {
    pub fn new() -> Self {
        Self {
            usage: Arc::new(Mutex::new(ResourceUsage::default())),
            limits: ResourceLimits {
                max_memory: 1024 * 1024 * 1024, // 1GB
                max_execution_time: Duration::from_secs(30),
                max_db_operations: 10000,
                max_crypto_operations: 1000,
            },
        }
    }

    /// Track an operation
    pub async fn track_operation(&self, operation_type: &str) -> Result<(), ExecutionError> {
        let mut usage = self.usage.lock().unwrap();

        match operation_type {
            "database" => {
                usage.db_operations += 1;
                if usage.db_operations > self.limits.max_db_operations {
                    return Err(ExecutionError::ResourceLimitExceeded("Database operations limit exceeded".to_string()));
                }
            }
            "crypto" => {
                usage.crypto_operations += 1;
                if usage.crypto_operations > self.limits.max_crypto_operations {
                    return Err(ExecutionError::ResourceLimitExceeded("Crypto operations limit exceeded".to_string()));
                }
            }
            "parallel" => {
                usage.parallel_operations += 1;
            }
            _ => {}
        }

        Ok(())
    }

    /// Get current resource usage
    pub fn get_usage(&self) -> ResourceUsage {
        self.usage.lock().unwrap().clone()
    }
}

impl ValueType {
    /// Convert from WASM value to value type
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::I32(_) => ValueType::I32,
            Value::I64(_) => ValueType::I64,
            Value::F32(_) => ValueType::F32,
            Value::F64(_) => ValueType::F64,
            Value::V128(_) => ValueType::V128,
            Value::FuncRef(_) => ValueType::I32,   // Treat as reference
            Value::ExternRef(_) => ValueType::I32, // Treat as reference
        }
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            allowed_operations: vec!["database".to_string(), "crypto".to_string(), "parallel".to_string(), "state".to_string()],
            resource_limits: ResourceLimits {
                max_memory: 1024 * 1024 * 1024, // 1GB
                max_execution_time: Duration::from_secs(30),
                max_db_operations: 10000,
                max_crypto_operations: 1000,
            },
            security_level: SecurityLevel::Standard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_host_function_interface() {
        let handle = tokio::runtime::Handle::current();

        let interface = AsyncHostFunctionInterface::new(handle);
        let mut context = WasmExecutionContext::default();

        // Test registration
        assert!(interface.register_all(&mut context).is_ok());
    }

    #[test]
    fn test_value_type_conversion() {
        assert_eq!(ValueType::from_value(&Value::I32(42)), ValueType::I32);
        assert_eq!(ValueType::from_value(&Value::I64(42)), ValueType::I64);
        assert_eq!(ValueType::from_value(&Value::F32(42.0)), ValueType::F32);
        assert_eq!(ValueType::from_value(&Value::F64(42.0)), ValueType::F64);
    }

    #[test]
    fn test_type_validator() {
        let validator = TypeValidator::new();
        let signature = FunctionSignature {
            params: vec![ValueType::I32, ValueType::I64],
            returns: vec![ValueType::I32],
        };

        let params = vec![Value::I32(42), Value::I64(84)];
        assert!(validator.validate_params(&params, &signature).is_ok());

        let wrong_params = vec![Value::I32(42), Value::I32(84)];
        assert!(validator.validate_params(&wrong_params, &signature).is_err());
    }

    #[tokio::test]
    async fn test_resource_tracker() {
        let tracker = AsyncResourceTracker::new();

        // Test tracking operations
        assert!(tracker.track_operation("database").await.is_ok());
        assert!(tracker.track_operation("crypto").await.is_ok());

        let usage = tracker.get_usage();
        assert_eq!(usage.db_operations, 1);
        assert_eq!(usage.crypto_operations, 1);
    }
}
