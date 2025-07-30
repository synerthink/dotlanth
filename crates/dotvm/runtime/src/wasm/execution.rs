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

//! WASM Execution Context Module

use crate::wasm::instance::WasmGlobal;
use crate::wasm::interpreter::WasmInterpreter;
use crate::wasm::memory::WasmMemory;
use crate::wasm::{WasmError, WasmResult};
use dotvm_compiler::wasm::ast::WasmValue as Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Call frame for function execution
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Function index being executed
    pub function_index: u32,
    /// Number of return values expected
    pub return_arity: usize,
    /// Start of local variables on stack
    pub locals_start: usize,
    /// Frame metadata
    pub metadata: FrameMetadata,
}

/// Metadata for call frames
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    /// Function name
    pub function_name: String,
    /// Time when call started
    pub call_time: Instant,
    /// Instructions executed in this frame
    pub instructions_executed: u64,
    /// Custom tags for debugging/profiling
    pub tags: std::collections::HashMap<String, String>,
}

/// WASM-specific execution extensions
pub struct WasmExecutionExtensions {
    /// Unique context identifier
    pub id: Uuid,
    /// Start time
    pub start_time: Instant,
    /// Maximum execution duration
    pub max_duration: Duration,
    /// Maximum instructions allowed
    pub max_instructions: u64,
    /// Call depth
    pub call_depth: usize,
    /// Maximum call depth
    pub max_call_depth: usize,
    /// Call stack for function frames
    pub call_stack: Vec<CallFrame>,
    /// Host functions
    pub host_functions: HashMap<String, HostFunction>,
    /// Imported modules
    pub imports: HashMap<String, ImportedModule>,
    /// Context metadata
    pub metadata: HashMap<String, String>,
    /// Execution metrics
    pub metrics: ExecutionMetrics,
    /// Security context
    pub security: SecurityContext,
    /// WASM interpreter for native instruction execution
    pub interpreter: Option<crate::wasm::interpreter::WasmInterpreter>,
    /// Global variables
    pub globals: Option<Vec<WasmGlobal>>,
    /// Memory instance
    pub memory: Option<WasmMemory>,
    /// Function signatures
    pub function_signatures: HashMap<u32, FunctionSignature>,
    /// Function types
    pub function_types: HashMap<u32, FunctionSignature>,
    /// Table functions
    pub table_functions: HashMap<(u32, u32), u32>,
    /// Reference to the WASM module for accessing function metadata
    pub module: Option<std::sync::Arc<crate::wasm::WasmModule>>,
}

impl std::fmt::Debug for WasmExecutionExtensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmExecutionExtensions")
            .field("id", &self.id)
            .field("start_time", &self.start_time)
            .field("max_duration", &self.max_duration)
            .field("max_instructions", &self.max_instructions)
            .field("call_depth", &self.call_depth)
            .field("max_call_depth", &self.max_call_depth)
            .field("call_stack", &self.call_stack)
            .field("host_functions", &format!("<{} functions>", self.host_functions.len()))
            .field("imports", &format!("<{} imports>", self.imports.len()))
            .field("metadata", &self.metadata)
            .field("metrics", &self.metrics)
            .field("security", &self.security)
            .field("interpreter", &self.interpreter.is_some())
            .finish()
    }
}

/// WASM Execution Context - standalone WASM execution environment
pub struct ExecutionContext {
    /// WASM interpreter for instruction execution
    pub interpreter: WasmInterpreter,
    /// WASM-specific execution state and limits
    pub wasm: WasmExecutionExtensions,
    /// Execution state
    pub state: ExecutionState,
    /// Execution metrics
    pub metrics: ExecutionMetrics,
}

/// Execution state
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    /// Ready to execute
    Ready,
    /// Currently executing
    Running,
    /// Execution paused
    Paused,
    /// Execution completed
    Completed,
    /// Execution failed
    Failed,
    /// Execution halted
    Halted,
}

/// Execution statistics
#[derive(Debug, Default)]
pub struct ExecutionStatistics {
    /// Total instructions executed
    pub instructions_executed: u64,
    /// Total execution time
    pub total_time: Duration,
    /// Average instructions per second
    pub avg_instructions_per_second: f64,
}

/// Value type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
}

/// Host function type
pub type HostFunction = Box<dyn Fn(&[Value]) -> WasmResult<Vec<Value>> + Send + Sync>;

/// Imported module
pub struct ImportedModule {
    /// Module name
    pub name: String,
    /// Exported functions
    pub functions: HashMap<String, HostFunction>,
    /// Exported globals
    pub globals: HashMap<String, Value>,
    /// Exported memories
    pub memories: HashMap<String, Vec<u8>>,
    /// Exported tables
    pub tables: HashMap<String, Vec<Value>>,
}

/// Function signature
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// Parameter types
    pub params: Vec<ValueType>,
    /// Return types
    pub returns: Vec<ValueType>,
}

/// Execution metrics
#[derive(Debug, Default)]
pub struct ExecutionMetrics {
    /// Total instructions executed
    pub instructions_executed: u64,
    /// Total function calls
    pub function_calls: u64,
    /// Memory allocations
    pub memory_allocations: u64,
    /// Memory deallocations
    pub memory_deallocations: u64,
    /// Host function calls
    pub host_function_calls: u64,
    /// Exceptions thrown
    pub exceptions: u64,
    /// Traps encountered
    pub traps: u64,
}

/// Security context
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Blocked operations
    pub blocked_operations: Vec<String>,
    /// Resource limits
    pub resource_limits: HashMap<String, u64>,
    /// Security level
    pub security_level: SecurityLevel,
    /// Sandbox enabled
    pub sandbox_enabled: bool,
}

/// Security level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Minimal security (development)
    Minimal,
    /// Standard security (testing)
    Standard,
    /// High security (production)
    High,
    /// Maximum security (critical)
    Maximum,
}

/// WASM Execution Stack
#[derive(Debug)]
pub struct WasmStack {
    /// Value stack
    values: Vec<Value>,
    /// Call stack
    calls: Vec<CallFrame>,
    /// Maximum stack size
    max_size: usize,
    /// Maximum call depth
    max_call_depth: usize,
    /// Stack statistics
    stats: StackStats,
}

/// Stack frame metadata
#[derive(Debug, Clone, Default)]
pub struct StackFrameMetadata {
    /// Entry timestamp
    pub entry_time: Option<std::time::Instant>,
    /// Instructions executed in this frame
    pub instructions_executed: u64,
    /// Frame tags
    pub tags: HashMap<String, String>,
}

/// Stack statistics
#[derive(Debug, Default)]
pub struct StackStats {
    /// Maximum value stack depth reached
    pub max_value_depth: usize,
    /// Maximum call stack depth reached
    pub max_call_depth: usize,
    /// Total pushes
    pub total_pushes: u64,
    /// Total pops
    pub total_pops: u64,
    /// Total function calls
    pub total_calls: u64,
    /// Total returns
    pub total_returns: u64,
    /// Stack overflows
    pub overflows: u64,
    /// Stack underflows
    pub underflows: u64,
}

/// Call trace entry
#[derive(Debug, Clone)]
pub struct CallTraceEntry {
    /// Function index
    pub function_index: u32,
    /// Function name
    pub function_name: Option<String>,
    /// Call timestamp
    pub timestamp: Instant,
}

impl WasmExecutionExtensions {
    /// Get function signature
    pub fn get_function_signature(&self, function_index: u32) -> Option<&FunctionSignature> {
        self.function_signatures.get(&function_index)
    }

    /// Set function signature
    pub fn set_function_signature(&mut self, function_index: u32, signature: FunctionSignature) {
        self.function_signatures.insert(function_index, signature);
    }

    /// Get function type
    pub fn get_function_type(&self, type_index: u32) -> Option<&FunctionSignature> {
        self.function_types.get(&type_index)
    }

    /// Set function type
    pub fn set_function_type(&mut self, type_index: u32, signature: FunctionSignature) {
        self.function_types.insert(type_index, signature);
    }

    /// Get table function
    pub fn get_table_function(&self, table_index: u32, func_index: u32) -> Option<u32> {
        self.table_functions.get(&(table_index, func_index)).copied()
    }

    /// Set table function
    pub fn set_table_function(&mut self, table_index: u32, func_index: u32, actual_func_index: u32) {
        self.table_functions.insert((table_index, func_index), actual_func_index);
    }
}

impl ExecutionContext {
    /// Create a new WASM execution context
    pub fn new(max_instructions: u64, max_call_depth: usize, max_duration: Duration) -> Self {
        Self {
            interpreter: WasmInterpreter::new(),
            wasm: WasmExecutionExtensions {
                id: Uuid::new_v4(),
                start_time: Instant::now(),
                max_duration,
                max_instructions,
                call_depth: 0,
                max_call_depth,
                call_stack: Vec::new(),
                host_functions: HashMap::new(),
                imports: HashMap::new(),
                interpreter: Some(WasmInterpreter::new()),
                metadata: HashMap::new(),
                metrics: ExecutionMetrics::default(),
                security: SecurityContext::default(),
                globals: None,
                memory: None,
                function_signatures: HashMap::new(),
                function_types: HashMap::new(),
                table_functions: HashMap::new(),
                module: None,
            },
            state: ExecutionState::Ready,
            metrics: ExecutionMetrics::default(),
        }
    }

    /// Check if execution should halt (WASM-specific checks)
    pub fn should_halt(&self) -> bool {
        self.wasm.call_depth >= self.wasm.max_call_depth
            || self.wasm.start_time.elapsed() >= self.wasm.max_duration
            || self.metrics.instructions_executed >= self.wasm.max_instructions
            || self.state == ExecutionState::Halted
    }

    /// Check if execution should continue
    pub fn should_continue(&self) -> WasmResult<()> {
        if self.should_halt() {
            // Check timeout
            if self.wasm.start_time.elapsed() > self.wasm.max_duration {
                return Err(WasmError::timeout(self.wasm.max_duration.as_millis() as u64));
            }
        }

        Ok(())
    }

    /// Check execution timeout
    pub fn check_timeout(&self) -> WasmResult<()> {
        let elapsed = self.wasm.start_time.elapsed();
        if elapsed > self.wasm.max_duration {
            return Err(WasmError::Timeout {
                timeout_ms: self.wasm.max_duration.as_millis() as u64,
            });
        }
        Ok(())
    }

    /// Increment call depth
    pub fn enter_call(&mut self) -> WasmResult<()> {
        if self.wasm.call_depth >= self.wasm.max_call_depth {
            return Err(WasmError::StackOverflow {
                current: self.wasm.call_depth,
                max: self.wasm.max_call_depth,
            });
        }
        self.wasm.call_depth += 1;
        self.wasm.metrics.function_calls += 1;
        Ok(())
    }

    /// Decrement call depth
    pub fn exit_call(&mut self) {
        if self.wasm.call_depth > 0 {
            self.wasm.call_depth -= 1;
        }
    }

    /// Count an instruction
    pub fn count_instruction(&mut self) -> WasmResult<()> {
        self.metrics.instructions_executed += 1;

        if self.metrics.instructions_executed >= self.wasm.max_instructions {
            return Err(WasmError::execution_error(format!(
                "Instruction limit exceeded: current={}, limit={}",
                self.metrics.instructions_executed, self.wasm.max_instructions
            )));
        }

        Ok(())
    }

    /// Push a value onto the execution stack
    pub fn push_value(&mut self, value: Value) -> WasmResult<()> {
        self.interpreter.push_value(value).map_err(|e| WasmError::execution_error(format!("Stack push failed: {}", e)))
    }

    /// Pop a value from the execution stack
    pub fn pop_value(&mut self) -> WasmResult<Value> {
        self.interpreter.pop_value().map_err(|e| WasmError::execution_error(format!("Stack pop failed: {}", e)))
    }

    /// Pop two values from the execution stack
    pub fn pop_values(&mut self) -> WasmResult<Vec<Value>> {
        let val1 = self.pop_value()?;
        let val2 = self.pop_value()?;
        Ok(vec![val2, val1]) // Note: reversed order due to stack LIFO
    }

    /// Push a call frame onto the call stack
    pub fn push_frame(&mut self, frame: CallFrame) -> WasmResult<()> {
        if self.wasm.call_stack.len() >= self.wasm.max_call_depth {
            return Err(WasmError::StackOverflow {
                current: self.wasm.call_stack.len(),
                max: self.wasm.max_call_depth,
            });
        }
        self.wasm.call_stack.push(frame);
        self.wasm.call_depth = self.wasm.call_stack.len();
        Ok(())
    }

    /// Pop a call frame from the call stack
    pub fn pop_frame(&mut self) -> WasmResult<CallFrame> {
        if let Some(frame) = self.wasm.call_stack.pop() {
            self.wasm.call_depth = self.wasm.call_stack.len();
            Ok(frame)
        } else {
            Err(WasmError::StackUnderflow)
        }
    }

    /// Increment instruction count (legacy method)
    pub fn increment_instructions(&mut self) {
        let _ = self.count_instruction();
    }

    /// Register a host function
    pub fn register_host_function(&mut self, name: String, function: HostFunction) {
        self.wasm.host_functions.insert(name, function);
    }

    /// Get a host function
    pub fn get_host_function(&self, name: &str) -> Option<&HostFunction> {
        self.wasm.host_functions.get(name)
    }

    /// Register an imported module
    pub fn register_import(&mut self, module_name: String, module: ImportedModule) {
        self.wasm.imports.insert(module_name, module);
    }

    /// Get an imported module
    pub fn get_import(&self, module_name: &str) -> Option<&ImportedModule> {
        self.wasm.imports.get(module_name)
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.wasm.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.wasm.metadata.get(key)
    }

    /// Check if operation is allowed
    pub fn is_operation_allowed(&self, operation: &str) -> bool {
        if self.wasm.security.blocked_operations.contains(&operation.to_string()) {
            return false;
        }

        if !self.wasm.security.allowed_operations.is_empty() {
            return self.wasm.security.allowed_operations.contains(&operation.to_string());
        }

        true
    }

    /// Get current security level
    pub fn security_level(&self) -> SecurityLevel {
        self.wasm.security.security_level
    }

    /// Update security level
    pub fn set_security_level(&mut self, level: SecurityLevel) {
        self.wasm.security.security_level = level;
    }

    /// Get execution duration
    pub fn execution_duration(&self) -> Duration {
        self.wasm.start_time.elapsed()
    }

    /// Get execution time (alias for compatibility)
    pub fn execution_time(&self) -> Duration {
        self.execution_duration()
    }

    /// Get execution statistics
    pub fn get_statistics(&self) -> ExecutionStatistics {
        let elapsed = self.execution_duration();
        let avg_ips = if elapsed.as_secs_f64() > 0.0 {
            self.metrics.instructions_executed as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        ExecutionStatistics {
            instructions_executed: self.metrics.instructions_executed,
            total_time: elapsed,
            avg_instructions_per_second: avg_ips,
        }
    }

    /// Reset the execution context
    pub fn reset(&mut self) {
        self.interpreter = WasmInterpreter::new();
        self.wasm.call_depth = 0;
        self.wasm.start_time = Instant::now();
        self.wasm.metrics = ExecutionMetrics::default();
        self.wasm.metadata.clear();
        self.wasm.id = Uuid::new_v4();
        self.state = ExecutionState::Ready;
        self.metrics = ExecutionMetrics::default();
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new(
            1_000_000,               // 1M instruction limit
            1000,                    // 1000 call depth
            Duration::from_secs(30), // 30 second timeout
        )
    }
}

impl FunctionSignature {
    /// Create new function signature
    pub fn new(params: Vec<ValueType>, returns: Vec<ValueType>) -> Self {
        Self { params, returns }
    }

    /// Get parameter count
    pub fn param_count(&self) -> usize {
        self.params.len()
    }

    /// Get return count
    pub fn return_count(&self) -> usize {
        self.returns.len()
    }
}

impl ValueType {
    /// Get size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            ValueType::I32 => 4,
            ValueType::I64 => 8,
            ValueType::F32 => 4,
            ValueType::F64 => 8,
            ValueType::V128 => 16,
            ValueType::FuncRef => 8,   // Pointer size
            ValueType::ExternRef => 8, // Pointer size
        }
    }

    /// Convert from Value
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::I32(_) => ValueType::I32,
            Value::I64(_) => ValueType::I64,
            Value::F32(_) => ValueType::F32,
            Value::F64(_) => ValueType::F64,
            Value::V128(_) => ValueType::V128,
            Value::FuncRef(_) => ValueType::FuncRef,
            Value::ExternRef(_) => ValueType::ExternRef,
        }
    }
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("id", &self.wasm.id)
            .field("start_time", &self.wasm.start_time)
            .field("max_duration", &self.wasm.max_duration)
            .field("instructions_executed", &self.metrics.instructions_executed)
            .field("call_depth", &self.wasm.call_depth)
            .field("max_call_depth", &self.wasm.max_call_depth)
            .field("state", &self.state)
            .field("host_functions", &format!("<{} functions>", self.wasm.host_functions.len()))
            .field("imports", &format!("<{} imports>", self.wasm.imports.len()))
            .field("metadata", &self.wasm.metadata)
            .field("metrics", &self.metrics)
            .field("security", &self.wasm.security)
            .finish()
    }
}

impl std::fmt::Debug for ImportedModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportedModule")
            .field("name", &self.name)
            .field("functions", &format!("<{} functions>", self.functions.len()))
            .field("globals", &self.globals)
            .field("memories", &format!("<{} memories>", self.memories.len()))
            .field("tables", &format!("<{} tables>", self.tables.len()))
            .finish()
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            allowed_operations: Vec::new(),
            blocked_operations: Vec::new(),
            resource_limits: HashMap::new(),
            security_level: SecurityLevel::Standard,
            sandbox_enabled: true,
        }
    }
}

impl WasmStack {
    /// Create a new stack
    pub fn new(max_size: usize, max_call_depth: usize) -> Self {
        Self {
            values: Vec::new(),
            calls: Vec::new(),
            max_size,
            max_call_depth,
            stats: StackStats::default(),
        }
    }

    /// Push a value onto the stack
    pub fn push(&mut self, value: Value) -> WasmResult<()> {
        if self.values.len() >= self.max_size {
            self.stats.overflows += 1;
            return Err(WasmError::StackOverflow {
                current: self.values.len(),
                max: self.max_size,
            });
        }

        self.values.push(value);
        self.stats.total_pushes += 1;

        if self.values.len() > self.stats.max_value_depth {
            self.stats.max_value_depth = self.values.len();
        }

        Ok(())
    }

    /// Pop a value from the stack
    pub fn pop(&mut self) -> WasmResult<Value> {
        match self.values.pop() {
            Some(value) => {
                self.stats.total_pops += 1;
                Ok(value)
            }
            None => {
                self.stats.underflows += 1;
                Err(WasmError::execution_error("Stack underflow"))
            }
        }
    }

    /// Peek at the top value without removing it
    pub fn peek(&self) -> WasmResult<&Value> {
        self.values.last().ok_or_else(|| WasmError::execution_error("Stack is empty"))
    }

    /// Peek at a value at a specific depth from the top
    pub fn peek_at(&self, depth: usize) -> WasmResult<&Value> {
        if depth >= self.values.len() {
            return Err(WasmError::execution_error("Stack depth out of bounds"));
        }

        let index = self.values.len() - 1 - depth;
        Ok(&self.values[index])
    }

    /// Pop a value of a specific type
    pub fn pop_typed(&mut self, expected_type: ValueType) -> WasmResult<Value> {
        let value = self.pop()?;
        let value_type = ValueType::from_value(&value);

        if value_type != expected_type {
            return Err(WasmError::type_mismatch(format!("{:?}", expected_type), format!("{:?}", value_type)));
        }

        Ok(value)
    }

    /// Enter a function call
    pub fn enter_call(&mut self, function_index: u32, function_name: Option<String>, locals: Vec<Value>, return_address: usize) -> WasmResult<()> {
        if self.calls.len() >= self.max_call_depth {
            self.stats.overflows += 1;
            return Err(WasmError::StackOverflow {
                current: self.calls.len(),
                max: self.max_call_depth,
            });
        }

        let frame = CallFrame {
            function_index,
            return_arity: 0, // Will be set based on function signature
            locals_start: self.values.len(),
            metadata: FrameMetadata {
                function_name: function_name.unwrap_or_else(|| format!("func_{}", function_index)),
                call_time: Instant::now(),
                instructions_executed: 0,
                tags: HashMap::new(),
            },
        };

        self.calls.push(frame);
        self.stats.total_calls += 1;

        if self.calls.len() > self.stats.max_call_depth {
            self.stats.max_call_depth = self.calls.len();
        }

        Ok(())
    }

    /// Exit a function call
    pub fn exit_call(&mut self) -> WasmResult<CallFrame> {
        match self.calls.pop() {
            Some(frame) => {
                self.stats.total_returns += 1;
                Ok(frame)
            }
            None => {
                self.stats.underflows += 1;
                Err(WasmError::execution_error("Call stack underflow"))
            }
        }
    }

    /// Get current call frame
    pub fn current_frame(&self) -> Option<&CallFrame> {
        self.calls.last()
    }

    /// Get mutable current call frame
    pub fn current_frame_mut(&mut self) -> Option<&mut CallFrame> {
        self.calls.last_mut()
    }

    /// Get call depth
    pub fn call_depth(&self) -> usize {
        self.calls.len()
    }

    /// Get value stack depth
    pub fn value_depth(&self) -> usize {
        self.values.len()
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Check if call stack is empty
    pub fn is_call_stack_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// Clear the value stack
    pub fn clear_values(&mut self) {
        self.values.clear();
    }

    /// Clear the call stack
    pub fn clear_calls(&mut self) {
        self.calls.clear();
    }

    /// Get stack statistics
    pub fn statistics(&self) -> StackStatistics {
        StackStatistics {
            max_value_depth: self.stats.max_value_depth,
            max_call_depth: self.stats.max_call_depth,
            total_pushes: self.stats.total_pushes,
            total_pops: self.stats.total_pops,
            total_calls: self.stats.total_calls,
            total_returns: self.stats.total_returns,
            overflows: self.stats.overflows,
            underflows: self.stats.underflows,
            current_value_depth: self.values.len(),
            current_call_depth: self.calls.len(),
        }
    }

    /// Get call stack trace
    pub fn call_trace(&self) -> Vec<CallTraceEntry> {
        self.calls
            .iter()
            .map(|frame| CallTraceEntry {
                function_index: frame.function_index,
                function_name: Some(frame.metadata.function_name.clone()),
                timestamp: frame.metadata.call_time,
            })
            .collect()
    }

    /// Validate stack integrity
    pub fn validate(&self) -> WasmResult<()> {
        // Check stack bounds
        if self.values.len() > self.max_size {
            return Err(WasmError::execution_error("Value stack overflow"));
        }

        if self.calls.len() > self.max_call_depth {
            return Err(WasmError::execution_error("Call stack overflow"));
        }

        Ok(())
    }
}

/// Stack statistics for external consumption
#[derive(Debug, Clone)]
pub struct StackStatistics {
    pub max_value_depth: usize,
    pub max_call_depth: usize,
    pub total_pushes: u64,
    pub total_pops: u64,
    pub total_calls: u64,
    pub total_returns: u64,
    pub overflows: u64,
    pub underflows: u64,
    pub current_value_depth: usize,
    pub current_call_depth: usize,
}

impl StackStatistics {
    /// Calculate push/pop ratio
    pub fn push_pop_ratio(&self) -> f64 {
        if self.total_pops == 0 {
            if self.total_pushes == 0 { 0.0 } else { f64::INFINITY }
        } else {
            self.total_pushes as f64 / self.total_pops as f64
        }
    }

    /// Calculate call/return ratio
    pub fn call_return_ratio(&self) -> f64 {
        if self.total_returns == 0 {
            if self.total_calls == 0 { 0.0 } else { f64::INFINITY }
        } else {
            self.total_calls as f64 / self.total_returns as f64
        }
    }

    /// Calculate error rate
    pub fn error_rate(&self) -> f64 {
        let total_operations = self.total_pushes + self.total_pops + self.total_calls + self.total_returns;
        let total_errors = self.overflows + self.underflows;

        if total_operations == 0 { 0.0 } else { total_errors as f64 / total_operations as f64 }
    }

    /// Calculate utilization percentage
    pub fn utilization_percentage(&self) -> f64 {
        (self.current_value_depth as f64 / self.max_value_depth.max(1) as f64) * 100.0
    }
}

impl CallFrame {
    /// Get frame duration
    pub fn duration(&self) -> Duration {
        self.metadata.call_time.elapsed()
    }

    /// Add instruction count
    pub fn count_instruction(&mut self) {
        self.metadata.instructions_executed += 1;
    }

    /// Set frame tag
    pub fn set_tag(&mut self, key: String, value: String) {
        self.metadata.tags.insert(key, value);
    }

    /// Get frame tag
    pub fn get_tag(&self, key: &str) -> Option<&String> {
        self.metadata.tags.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context() {
        let mut ctx = ExecutionContext::new(1000000, 100, Duration::from_secs(1));
        assert_eq!(ctx.wasm.call_depth, 0);
        assert_eq!(ctx.metrics.instructions_executed, 0);
    }

    #[test]
    fn test_execution_states() {
        assert_eq!(ExecutionState::Ready, ExecutionState::Ready);
        assert_ne!(ExecutionState::Ready, ExecutionState::Running);
    }

    #[test]
    fn test_stack_creation() {
        let stack = WasmStack::new(100, 10);
        assert!(stack.is_empty());
        assert!(stack.is_call_stack_empty());
        assert_eq!(stack.value_depth(), 0);
        assert_eq!(stack.call_depth(), 0);
    }

    #[test]
    fn test_stack_push_pop() {
        let mut stack = WasmStack::new(100, 10);

        // Test push
        assert!(stack.push(Value::I32(42)).is_ok());
        assert_eq!(stack.value_depth(), 1);
        assert!(!stack.is_empty());

        // Test peek
        match stack.peek() {
            Ok(Value::I32(val)) => assert_eq!(*val, 42),
            _ => panic!("Expected I32(42)"),
        }

        // Test pop
        match stack.pop() {
            Ok(Value::I32(val)) => assert_eq!(val, 42),
            _ => panic!("Expected I32(42)"),
        }

        assert!(stack.is_empty());
        assert_eq!(stack.value_depth(), 0);
    }

    #[test]
    fn test_stack_overflow() {
        let mut stack = WasmStack::new(2, 10);

        assert!(stack.push(Value::I32(1)).is_ok());
        assert!(stack.push(Value::I32(2)).is_ok());
        assert!(stack.push(Value::I32(3)).is_err()); // Should overflow
    }

    #[test]
    fn test_stack_underflow() {
        let mut stack = WasmStack::new(100, 10);
        assert!(stack.pop().is_err()); // Should underflow
    }

    #[test]
    fn test_call_stack() {
        let mut stack = WasmStack::new(100, 10);

        // Enter call
        assert!(stack.enter_call(0, Some("test_function".to_string()), vec![Value::I32(1), Value::I32(2)], 0x1000,).is_ok());

        assert_eq!(stack.call_depth(), 1);
        assert!(!stack.is_call_stack_empty());

        // Check current frame
        let frame = stack.current_frame().unwrap();
        assert_eq!(frame.function_index, 0);
        assert_eq!(frame.metadata.function_name, "test_function".to_string());
        // Note: locals are managed on the value stack, not directly in CallFrame

        // Exit call
        let frame = stack.exit_call().unwrap();
        assert_eq!(frame.function_index, 0);
        assert_eq!(stack.call_depth(), 0);
        assert!(stack.is_call_stack_empty());
    }

    #[test]
    fn test_call_stack_overflow() {
        let mut stack = WasmStack::new(100, 2);

        assert!(stack.enter_call(0, None, vec![], 0).is_ok());
        assert!(stack.enter_call(1, None, vec![], 0).is_ok());
        assert!(stack.enter_call(2, None, vec![], 0).is_err()); // Should overflow
    }

    #[test]
    fn test_call_stack_underflow() {
        let mut stack = WasmStack::new(100, 10);
        assert!(stack.exit_call().is_err()); // Should underflow
    }

    #[test]
    fn test_stack_statistics() {
        let mut stack = WasmStack::new(100, 10);

        // Perform some operations
        stack.push(Value::I32(1)).unwrap();
        stack.push(Value::I32(2)).unwrap();
        stack.pop().unwrap();
        stack.enter_call(0, None, vec![], 0).unwrap();
        stack.exit_call().unwrap();

        let stats = stack.statistics();
        assert_eq!(stats.total_pushes, 2);
        assert_eq!(stats.total_pops, 1);
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.total_returns, 1);
        assert_eq!(stats.current_value_depth, 1);
        assert_eq!(stats.current_call_depth, 0);

        // Test statistics calculations
        assert!(stats.push_pop_ratio() > 0.0);
        assert!(stats.call_return_ratio() >= 1.0);
        assert_eq!(stats.error_rate(), 0.0);
    }

    #[test]
    fn test_peek_at_depth() {
        let mut stack = WasmStack::new(100, 10);

        stack.push(Value::I32(1)).unwrap();
        stack.push(Value::I32(2)).unwrap();
        stack.push(Value::I32(3)).unwrap();

        // Test peek at different depths
        match stack.peek_at(0) {
            Ok(Value::I32(val)) => assert_eq!(*val, 3), // Top
            _ => panic!("Expected I32(3)"),
        }

        match stack.peek_at(1) {
            Ok(Value::I32(val)) => assert_eq!(*val, 2), // Middle
            _ => panic!("Expected I32(2)"),
        }

        match stack.peek_at(2) {
            Ok(Value::I32(val)) => assert_eq!(*val, 1), // Bottom
            _ => panic!("Expected I32(1)"),
        }

        // Test out of bounds
        assert!(stack.peek_at(3).is_err());
    }

    #[test]
    fn test_call_trace() {
        let mut stack = WasmStack::new(100, 10);

        stack.enter_call(0, Some("func1".to_string()), vec![], 0).unwrap();
        stack.enter_call(1, Some("func2".to_string()), vec![], 0).unwrap();

        let trace = stack.call_trace();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].function_index, 0);
        assert_eq!(trace[0].function_name, Some("func1".to_string()));
        assert_eq!(trace[1].function_index, 1);
        assert_eq!(trace[1].function_name, Some("func2".to_string()));
    }

    #[test]
    fn test_frame_metadata() {
        let mut stack = WasmStack::new(100, 10);

        stack.enter_call(0, None, vec![], 0).unwrap();

        // Test frame metadata operations
        if let Some(frame) = stack.current_frame_mut() {
            frame.count_instruction();
            frame.set_tag("test".to_string(), "value".to_string());

            assert_eq!(frame.metadata.instructions_executed, 1);
            assert_eq!(frame.get_tag("test"), Some(&"value".to_string()));
        }
    }

    #[test]
    fn test_stack_validation() {
        let stack = WasmStack::new(100, 10);
        assert!(stack.validate().is_ok());
    }

    #[test]
    fn test_execution_context_with_limits() {
        let ctx = ExecutionContext::new(10000, 100, Duration::from_secs(10));

        assert_eq!(ctx.wasm.max_call_depth, 100);
        assert_eq!(ctx.wasm.max_instructions, 10000);
        assert!(ctx.interpreter.is_stack_empty());
        // PC is managed internally by interpreter
    }

    #[test]
    fn test_call_depth_management() {
        let mut ctx = ExecutionContext::new(1000000, 100, Duration::from_secs(1));

        // Test entering calls
        assert!(ctx.enter_call().is_ok());
        assert_eq!(ctx.wasm.call_depth, 1);
        assert_eq!(ctx.wasm.metrics.function_calls, 1);

        // Test exiting calls
        ctx.exit_call();
        assert_eq!(ctx.wasm.call_depth, 0);
    }

    #[test]
    fn test_instruction_counting() {
        let mut ctx = ExecutionContext::new(
            5, // Low instruction limit for testing
            100,
            Duration::from_secs(1),
        );

        // Test instruction counting
        for i in 1..=4 {
            assert!(ctx.count_instruction().is_ok());
            assert_eq!(ctx.metrics.instructions_executed, i);
        }

        // Should exceed limit on 5th instruction
        assert!(ctx.count_instruction().is_err());
    }

    #[test]
    fn test_host_functions() {
        let mut ctx = ExecutionContext::new(1000000, 100, Duration::from_secs(1));

        // Register a host function
        let test_func: HostFunction = Box::new(|_args| Ok(vec![Value::I32(42)]));

        ctx.register_host_function("test".to_string(), test_func);
        assert!(ctx.get_host_function("test").is_some());
        assert!(ctx.get_host_function("nonexistent").is_none());
    }

    #[test]
    fn test_metadata() {
        let mut ctx = ExecutionContext::new(1000000, 100, Duration::from_secs(1));

        ctx.set_metadata("key1".to_string(), "value1".to_string());
        assert_eq!(ctx.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_security_operations() {
        let mut ctx = ExecutionContext::new(1000000, 100, Duration::from_secs(1));

        // Initially all operations allowed
        assert!(ctx.is_operation_allowed("memory.load"));

        // Block specific operation
        ctx.wasm.security.blocked_operations.push("memory.load".to_string());
        assert!(!ctx.is_operation_allowed("memory.load"));

        // Allow list mode
        ctx.wasm.security.allowed_operations.push("memory.store".to_string());
        assert!(!ctx.is_operation_allowed("memory.load")); // Still blocked
        assert!(ctx.is_operation_allowed("memory.store")); // Explicitly allowed
        assert!(!ctx.is_operation_allowed("local.get")); // Not in allow list
    }

    #[test]
    fn test_context_reset() {
        let mut ctx = ExecutionContext::new(1000000, 100, Duration::from_secs(1));

        // Modify context state
        ctx.count_instruction().unwrap();
        ctx.enter_call().unwrap();
        ctx.set_metadata("test".to_string(), "value".to_string());

        let original_id = ctx.wasm.id;

        // Reset context
        ctx.reset();

        // Verify reset
        assert_ne!(ctx.wasm.id, original_id); // New ID
        assert_eq!(ctx.metrics.instructions_executed, 0);
        assert_eq!(ctx.wasm.call_depth, 0);
        assert!(ctx.wasm.metadata.is_empty());
        assert!(ctx.interpreter.is_stack_empty());
    }

    #[test]
    fn test_wasm_execution_context_creation() {
        let ctx = ExecutionContext::new(1000, 100, Duration::from_secs(10));
        assert_eq!(ctx.metrics.instructions_executed, 0);
    }

    #[test]
    fn test_call_depth() {
        let mut ctx = ExecutionContext::new(1000, 2, Duration::from_secs(10));

        // Enter calls
        ctx.enter_call().unwrap();
        assert_eq!(ctx.wasm.call_depth, 1);

        ctx.enter_call().unwrap();
        assert_eq!(ctx.wasm.call_depth, 2);

        // Try to exceed limit
        let result = ctx.enter_call();
        assert!(matches!(result, Err(WasmError::StackOverflow { .. })));

        // Exit calls
        ctx.exit_call();
        assert_eq!(ctx.wasm.call_depth, 1);

        ctx.exit_call();
        assert_eq!(ctx.wasm.call_depth, 0);
    }

    #[test]
    fn test_should_halt() {
        // Use a reasonable timeout to avoid immediate expiration
        let mut ctx = ExecutionContext::new(100, 10, Duration::from_millis(500));

        // Should not halt at start
        assert!(!ctx.should_halt());

        // Exceed instruction limit
        ctx.metrics.instructions_executed = 101;
        assert!(ctx.should_halt());

        // Reset and test timeout (sleep to exceed duration)
        ctx.reset();
        std::thread::sleep(Duration::from_millis(600));
        assert!(ctx.should_halt());
    }
}
