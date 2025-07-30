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

//! WASM Instance Implementation

use crate::wasm::execution::{CallFrame, FrameMetadata};
use crate::wasm::management::SecurityContext;
use crate::wasm::{ValueType, WasmError, WasmExecutionContext, WasmMemory, WasmModule, WasmResult};
use dotvm_compiler::wasm::ast::WasmValue as Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// WASM Instance
pub struct WasmInstance {
    /// Unique instance identifier
    pub id: Uuid,
    /// Reference to the module
    pub module: Arc<WasmModule>,
    /// Instance memory
    pub memory: Option<Arc<RwLock<WasmMemory>>>,
    /// Instance tables
    pub tables: HashMap<String, Arc<RwLock<WasmTable>>>,
    /// Instance globals
    pub globals: HashMap<String, Arc<RwLock<WasmGlobal>>>,
    /// Exported functions
    pub exports: HashMap<String, ExportedFunction>,
    /// Instance state
    pub state: InstanceState,
    /// Security context
    pub security: SecurityContext,
    /// Instance metadata
    pub metadata: InstanceMetadata,
    /// Host functions registry
    pub host_functions: HashMap<String, Box<dyn Fn(Vec<Value>) -> WasmResult<Vec<Value>> + Send + Sync>>,
}

impl std::fmt::Debug for WasmInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmInstance")
            .field("id", &self.id)
            .field("module", &self.module)
            .field("memory", &self.memory)
            .field("tables", &self.tables)
            .field("globals", &self.globals)
            .field("exports", &self.exports)
            .field("state", &self.state)
            .field("security", &self.security)
            .field("metadata", &self.metadata)
            .field("host_functions", &format!("<{} host functions>", self.host_functions.len()))
            .finish()
    }
}

/// WASM Table
#[derive(Debug)]
pub struct WasmTable {
    /// Element type
    pub element_type: ValueType,
    /// Table elements
    pub elements: Vec<Option<TableElement>>,
    /// Maximum size
    pub max_size: Option<usize>,
}

/// Table element
#[derive(Debug, Clone)]
pub enum TableElement {
    FuncRef(u32),
    ExternRef(u32),
}

/// WASM Global
#[derive(Debug)]
pub struct WasmGlobal {
    /// Value type
    pub value_type: ValueType,
    /// Current value
    pub value: Value,
    /// Mutability
    pub mutable: bool,
}

/// Exported function
#[derive(Debug, Clone)]
pub struct ExportedFunction {
    /// Function index
    pub index: u32,
    /// Function name
    pub name: String,
    /// Parameter types
    pub param_types: Vec<ValueType>,
    /// Return types
    pub return_types: Vec<ValueType>,
    /// Security level required
    pub security_level: u8,
}

/// Instance state
#[derive(Debug, Clone, PartialEq)]
pub enum InstanceState {
    /// Instance is being created
    Creating,
    /// Instance is ready for execution
    Ready,
    /// Instance is currently executing
    Executing,
    /// Instance is paused
    Paused,
    /// Instance has been terminated
    Terminated,
    /// Instance has encountered an error
    Error { message: String },
}

/// Instance metadata
#[derive(Debug, Clone)]
pub struct InstanceMetadata {
    /// Creation timestamp
    pub created_at: std::time::Instant,
    /// Last execution timestamp
    pub last_executed: Option<std::time::Instant>,
    /// Total execution count
    pub execution_count: u64,
    /// Total execution time
    pub total_execution_time: std::time::Duration,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Error count
    pub error_count: u64,
    /// Instance tags
    pub tags: HashMap<String, String>,
}

impl WasmInstance {
    /// Create a new WASM instance
    pub fn new(module: Arc<WasmModule>, security: SecurityContext) -> WasmResult<Self> {
        let id = Uuid::new_v4();

        // Initialize memory if required
        let memory = if module.memory_requirements.initial_pages > 0 {
            let mem = WasmMemory::new(module.memory_requirements.initial_pages, module.memory_requirements.max_pages)?;
            Some(Arc::new(RwLock::new(mem)))
        } else {
            None
        };

        // Initialize tables
        let mut tables = HashMap::new();
        for (i, table_req) in module.table_requirements.iter().enumerate() {
            let table = WasmTable::new(table_req.element_type, table_req.initial as usize, table_req.maximum.map(|m| m as usize))?;
            let name = table_req.name.clone().unwrap_or_else(|| format!("table_{}", i));
            tables.insert(name, Arc::new(RwLock::new(table)));
        }

        // Initialize globals
        let mut globals = HashMap::new();
        for global_meta in &module.globals {
            let global = WasmGlobal::new(global_meta.value_type, global_meta.initial_value.clone().into(), global_meta.mutable);
            let name = global_meta.name.clone().unwrap_or_else(|| format!("global_{}", global_meta.index));
            globals.insert(name, Arc::new(RwLock::new(global)));
        }

        // Initialize exports
        let mut exports = HashMap::new();
        for (name, export_meta) in &module.exports {
            if let Some(func_meta) = module.get_function(name) {
                let exported_func = ExportedFunction {
                    index: func_meta.index,
                    name: name.clone(),
                    param_types: func_meta.signature.params.clone(),
                    return_types: func_meta.signature.returns.clone(),
                    security_level: func_meta.security_level,
                };
                exports.insert(name.clone(), exported_func);
            }
        }

        Ok(Self {
            id,
            module,
            memory,
            tables,
            globals,
            exports,
            state: InstanceState::Creating,
            security,
            metadata: InstanceMetadata::new(),
            host_functions: HashMap::new(),
        })
    }

    /// Initialize the instance
    pub fn initialize(&mut self) -> WasmResult<()> {
        // Validate security requirements
        self.validate_security()?;

        // Initialize memory protection if enabled
        if self.security.sandbox_enabled {
            self.setup_memory_protection()?;
        }

        // Set state to ready
        self.state = InstanceState::Ready;

        Ok(())
    }

    /// Execute a function by name
    pub fn execute_function(&mut self, function_name: &str, args: &[Value], context: &mut WasmExecutionContext) -> WasmResult<Vec<Value>> {
        // Check if instance is ready
        if self.state != InstanceState::Ready {
            return Err(WasmError::execution_error(format!("Instance not ready for execution: {:?}", self.state)));
        }

        // Get exported function and clone it to avoid borrowing issues
        let func = self.exports.get(function_name).ok_or_else(|| WasmError::function_not_found(function_name))?.clone();

        // Validate arguments
        self.validate_arguments(&func, args)?;

        // Check security permissions
        self.check_execution_permissions(&func, context)?;

        // Update state
        self.state = InstanceState::Executing;
        self.metadata.last_executed = Some(std::time::Instant::now());
        self.metadata.execution_count += 1;

        // Execute function (placeholder implementation)
        let result = self.execute_function_internal(&func, args, context);

        // Update state based on result
        match &result {
            Ok(_) => self.state = InstanceState::Ready,
            Err(e) => {
                self.state = InstanceState::Error { message: e.to_string() };
                self.metadata.error_count += 1;
            }
        }

        // Update metrics
        self.update_execution_metrics(context);

        result
    }

    /// Get exported function
    pub fn get_export(&self, name: &str) -> Option<&ExportedFunction> {
        self.exports.get(name)
    }

    /// Get memory reference
    pub fn memory(&self) -> Option<&Arc<RwLock<WasmMemory>>> {
        self.memory.as_ref()
    }

    /// Get table reference
    pub fn table(&self, name: &str) -> Option<&Arc<RwLock<WasmTable>>> {
        self.tables.get(name)
    }

    /// Get global reference
    pub fn global(&self, name: &str) -> Option<&Arc<RwLock<WasmGlobal>>> {
        self.globals.get(name)
    }

    /// Pause execution
    pub fn pause(&mut self) -> WasmResult<()> {
        if self.state == InstanceState::Executing {
            self.state = InstanceState::Paused;
            Ok(())
        } else {
            Err(WasmError::execution_error("Cannot pause non-executing instance"))
        }
    }

    /// Resume execution
    pub fn resume(&mut self) -> WasmResult<()> {
        if self.state == InstanceState::Paused {
            self.state = InstanceState::Executing;
            Ok(())
        } else {
            Err(WasmError::execution_error("Cannot resume non-paused instance"))
        }
    }

    /// Terminate instance
    pub fn terminate(&mut self) {
        self.state = InstanceState::Terminated;
    }

    /// Check if instance is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, InstanceState::Ready | InstanceState::Executing | InstanceState::Paused)
    }

    /// Get instance statistics
    pub fn statistics(&self) -> InstanceStatistics {
        InstanceStatistics {
            id: self.id,
            state: self.state.clone(),
            execution_count: self.metadata.execution_count,
            total_execution_time: self.metadata.total_execution_time,
            peak_memory_usage: self.metadata.peak_memory_usage,
            error_count: self.metadata.error_count,
            uptime: self.metadata.created_at.elapsed(),
        }
    }

    // Private helper methods

    fn validate_security(&self) -> WasmResult<()> {
        // Check if module meets security requirements
        for requirement in &self.module.security_metadata.required_permissions {
            if !self.security.has_permission(requirement) {
                return Err(WasmError::security_violation(format!("Missing required permission: {}", requirement)));
            }
        }

        Ok(())
    }

    fn setup_memory_protection(&mut self) -> WasmResult<()> {
        // Configure memory protection and guards
        if let Some(memory) = &mut self.memory {
            let mut mem = memory.write().map_err(|_| WasmError::ExecutionError {
                message: "Failed to acquire memory lock".to_string(),
            })?;

            // Set up memory guards to prevent buffer overflows
            let memory_size = mem.size_bytes();
            let page_size = 65536; // WASM page size

            // Ensure memory is aligned to page boundaries
            if memory_size % page_size != 0 {
                return Err(WasmError::ExecutionError {
                    message: "Memory size not aligned to page boundary".to_string(),
                });
            }

            // Configure memory protection using existing method
            mem.enable_protection();

            // Note: Advanced memory protection features like guard pages and
            // permission settings would be implemented in a production system
        }

        Ok(())
    }

    fn validate_arguments(&self, func: &ExportedFunction, args: &[Value]) -> WasmResult<()> {
        if args.len() != func.param_types.len() {
            return Err(WasmError::type_mismatch(format!("{} parameters", func.param_types.len()), format!("{} arguments", args.len())));
        }

        for (i, (arg, expected_type)) in args.iter().zip(&func.param_types).enumerate() {
            let actual_type = ValueType::from_value(arg);
            if actual_type != *expected_type {
                return Err(WasmError::type_mismatch(format!("{:?}", expected_type), format!("{:?}", actual_type)));
            }
        }

        Ok(())
    }

    fn check_execution_permissions(&self, func: &ExportedFunction, context: &WasmExecutionContext) -> WasmResult<()> {
        // Check if function execution is allowed
        if !context.is_operation_allowed("function.call") {
            return Err(WasmError::security_violation("Function calls not allowed"));
        }

        // Check security level
        if func.security_level > context.wasm.security.security_level as u8 {
            return Err(WasmError::SecurityViolation {
                message: format!(
                    "Function call to {} requires security level {}, but context has level {}",
                    func.name, func.security_level, context.wasm.security.security_level as u8
                ),
            });
        }

        Ok(())
    }

    fn execute_function_internal(&mut self, func: &ExportedFunction, args: &[Value], context: &mut WasmExecutionContext) -> WasmResult<Vec<Value>> {
        // Create function call frame
        let frame = CallFrame {
            function_index: func.index,
            return_arity: func.return_types.len(),
            locals_start: 0, // Will be set by interpreter
            metadata: FrameMetadata {
                function_name: func.name.clone(),
                call_time: std::time::Instant::now(),
                instructions_executed: 0,
                tags: std::collections::HashMap::new(),
            },
        };
        context.push_frame(frame)?;

        let mut results = Vec::new();

        // If this is a host function, call it directly
        if let Some(host_func) = self.host_functions.get(&func.name) {
            let host_args: Vec<_> = args.iter().cloned().collect();
            let host_results = host_func(host_args)?;
            results = host_results;
        } else {
            // Get function from compiled module by index
            let compiled_func = self
                .module
                .compiled
                .functions
                .get(func.index as usize)
                .ok_or_else(|| WasmError::FunctionNotFound { name: func.name.clone() })?;

            // Use WASM interpreter for native WASM instruction execution
            if let Some(mut interpreter) = context.wasm.interpreter.take() {
                // Check if function has body (not a host function)
                if !compiled_func.body.is_empty() {
                    // Initialize locals based on function signature and parameters
                    let mut locals = args.to_vec();

                    // Add uninitialized locals for function-local variables
                    for _ in args.len()..compiled_func.locals.len() {
                        locals.push(Value::I32(0)); // Default initialization
                    }

                    // Execute WASM instructions using interpreter with locals
                    let result = interpreter.execute_instructions_with_locals(&compiled_func.body, context, locals)?;

                    // Validate result count matches function signature
                    if result.len() != func.return_types.len() {
                        return Err(WasmError::ExecutionError {
                            message: format!("Function returned {} values, expected {}", result.len(), func.return_types.len()),
                        });
                    }

                    results = result;
                }

                // Put interpreter back
                context.wasm.interpreter = Some(interpreter);
            }
        }

        // Pop function frame
        context.pop_frame()?;

        // Validate result types match function signature
        if results.len() != func.return_types.len() {
            return Err(WasmError::TypeMismatch {
                expected: format!("{} results", func.return_types.len()),
                actual: format!("{} results", results.len()),
            });
        }

        Ok(results)
    }

    fn update_execution_metrics(&mut self, context: &WasmExecutionContext) {
        self.metadata.total_execution_time += context.wasm.start_time.elapsed();

        if let Some(memory) = &self.memory {
            if let Ok(mem) = memory.read() {
                let current_usage = mem.size_bytes();
                if current_usage > self.metadata.peak_memory_usage {
                    self.metadata.peak_memory_usage = current_usage;
                }
            }
        }
    }
}

impl WasmTable {
    /// Create a new table
    pub fn new(element_type: ValueType, initial: usize, max_size: Option<usize>) -> WasmResult<Self> {
        if let Some(max) = max_size {
            if initial > max {
                return Err(WasmError::validation_error("Initial size exceeds maximum"));
            }
        }

        Ok(Self {
            element_type,
            elements: vec![None; initial],
            max_size,
        })
    }

    /// Get table element
    pub fn get(&self, index: usize) -> WasmResult<Option<&TableElement>> {
        self.elements.get(index).ok_or_else(|| WasmError::memory_error("Table index out of bounds")).map(|elem| elem.as_ref())
    }

    /// Set table element
    pub fn set(&mut self, index: usize, element: Option<TableElement>) -> WasmResult<()> {
        if index >= self.elements.len() {
            return Err(WasmError::memory_error("Table index out of bounds"));
        }
        self.elements[index] = element;
        Ok(())
    }

    /// Grow table
    pub fn grow(&mut self, delta: usize, init: Option<TableElement>) -> WasmResult<usize> {
        let old_size = self.elements.len();
        let new_size = old_size + delta;

        if let Some(max) = self.max_size {
            if new_size > max {
                return Err(WasmError::memory_error("Table growth exceeds maximum"));
            }
        }

        self.elements.resize(new_size, init);
        Ok(old_size)
    }

    /// Get table size
    pub fn size(&self) -> usize {
        self.elements.len()
    }
}

impl WasmGlobal {
    /// Create a new global
    pub fn new(value_type: ValueType, value: Value, mutable: bool) -> Self {
        Self { value_type, value, mutable }
    }

    /// Get global value
    pub fn get(&self) -> &Value {
        &self.value
    }

    /// Set global value
    pub fn set(&mut self, value: Value) -> WasmResult<()> {
        if !self.mutable {
            return Err(WasmError::execution_error("Cannot modify immutable global"));
        }

        let value_type = ValueType::from_value(&value);
        if value_type != self.value_type {
            return Err(WasmError::type_mismatch(format!("{:?}", self.value_type), format!("{:?}", value_type)));
        }

        self.value = value;
        Ok(())
    }
}

impl InstanceMetadata {
    /// Create new instance metadata
    pub fn new() -> Self {
        Self {
            created_at: std::time::Instant::now(),
            last_executed: None,
            execution_count: 0,
            total_execution_time: std::time::Duration::default(),
            peak_memory_usage: 0,
            error_count: 0,
            tags: HashMap::new(),
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    /// Get tag value
    pub fn get_tag(&self, key: &str) -> Option<&String> {
        self.tags.get(key)
    }
}

impl From<crate::wasm::module::GlobalValue> for Value {
    fn from(global_value: crate::wasm::module::GlobalValue) -> Self {
        match global_value {
            crate::wasm::module::GlobalValue::I32(v) => Value::I32(v),
            crate::wasm::module::GlobalValue::I64(v) => Value::I64(v),
            crate::wasm::module::GlobalValue::F32(v) => Value::F32(v),
            crate::wasm::module::GlobalValue::F64(v) => Value::F64(v),
            crate::wasm::module::GlobalValue::V128(v) => Value::V128(v),
            crate::wasm::module::GlobalValue::FuncRef(_) => Value::I32(0),   // Placeholder
            crate::wasm::module::GlobalValue::ExternRef(_) => Value::I32(0), // Placeholder
        }
    }
}

/// Instance statistics
#[derive(Debug, Clone)]
pub struct InstanceStatistics {
    pub id: Uuid,
    pub state: InstanceState,
    pub execution_count: u64,
    pub total_execution_time: std::time::Duration,
    pub peak_memory_usage: usize,
    pub error_count: u64,
    pub uptime: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::SecurityContext;
    use dotvm_compiler::wasm::WasmModule as CompilerModule;

    fn create_test_module() -> Arc<WasmModule> {
        use dotvm_compiler::wasm::ast::*;

        // Create a simple WASM function: (i32) -> i32 that adds 1 to input
        // local.get 0  ; get first parameter
        // i32.const 1  ; push constant 1
        // i32.add      ; add them
        let instructions = vec![WasmInstruction::LocalGet { local_index: 0 }, WasmInstruction::I32Const { value: 1 }, WasmInstruction::I32Add];

        let function_type = WasmFunctionType {
            params: vec![WasmValueType::I32],
            results: vec![WasmValueType::I32],
        };

        let wasm_function = WasmFunction {
            signature: function_type.clone(),
            locals: vec![], // No additional locals beyond parameters
            body: instructions,
        };

        let mut compiled = CompilerModule::new();
        compiled.functions.push(wasm_function);

        // Create runtime module with compiled data - use the correct constructor
        let mut module = crate::wasm::module::WasmModule::new(
            "test".to_string(),
            vec![1, 2, 3], // bytecode
            compiled,
        );

        // Add function metadata
        let func_meta = crate::wasm::module::FunctionMetadata::new(0, crate::wasm::FunctionSignature::new(vec![ValueType::I32], vec![ValueType::I32])).with_name("test_func".to_string());

        let export_meta = crate::wasm::module::ExportMetadata::new("test_func".to_string(), crate::wasm::module::ExportType::Function, 0);

        module.functions.insert("test_func".to_string(), func_meta);
        module.exports.insert("test_func".to_string(), export_meta);

        Arc::new(module)
    }

    #[test]
    fn test_instance_creation() {
        let module = create_test_module();
        let security = SecurityContext::default();

        let instance = WasmInstance::new(module, security);
        assert!(instance.is_ok());

        let instance = instance.unwrap();
        assert_eq!(instance.state, InstanceState::Creating);
        assert!(instance.exports.contains_key("test_func"));
    }

    #[test]
    fn test_instance_initialization() {
        let module = create_test_module();
        let security = SecurityContext::default();

        let mut instance = WasmInstance::new(module, security).unwrap();
        assert!(instance.initialize().is_ok());
        assert_eq!(instance.state, InstanceState::Ready);
    }

    #[test]
    fn test_function_execution() {
        let module = create_test_module();
        let security = SecurityContext::default();

        let mut instance = WasmInstance::new(module, security).unwrap();
        instance.initialize().unwrap();

        let mut context = WasmExecutionContext::new(1000000, 100, std::time::Duration::from_secs(10));
        let args = vec![Value::I32(42)];

        let result = instance.execute_function("test_func", &args, &mut context);
        // Debug: print error if execution fails
        if let Err(ref e) = result {
            println!("Function execution failed: {:?}", e);
        } else {
            println!("Function execution succeeded: {:?}", result);
        }

        // Debug: Check function metadata
        if let Some(func_meta) = instance.module.functions.get("test_func") {
            println!("Function signature: params={:?}, returns={:?}", func_meta.signature.params, func_meta.signature.returns);
        }

        // Should successfully execute: 42 + 1 = 43
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], Value::I32(43));
    }

    #[test]
    fn test_invalid_function_call() {
        let module = create_test_module();
        let security = SecurityContext::default();

        let mut instance = WasmInstance::new(module, security).unwrap();
        instance.initialize().unwrap();

        let mut context = WasmExecutionContext::new(1000000, 100, std::time::Duration::from_secs(10));
        let args = vec![];

        let result = instance.execute_function("nonexistent", &args, &mut context);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WasmError::FunctionNotFound { .. }));
    }

    #[test]
    fn test_argument_validation() {
        let module = create_test_module();
        let security = SecurityContext::default();

        let mut instance = WasmInstance::new(module, security).unwrap();
        instance.initialize().unwrap();

        let mut context = WasmExecutionContext::new(1000000, 100, std::time::Duration::from_secs(10));

        // Wrong number of arguments
        let args = vec![];
        let result = instance.execute_function("test_func", &args, &mut context);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WasmError::TypeMismatch { .. }));
    }

    #[test]
    fn test_table_operations() {
        let mut table = WasmTable::new(ValueType::FuncRef, 5, Some(10)).unwrap();

        assert_eq!(table.size(), 5);
        assert!(table.get(0).unwrap().is_none());

        table.set(0, Some(TableElement::FuncRef(42))).unwrap();
        assert!(table.get(0).unwrap().is_some());

        let old_size = table.grow(3, None).unwrap();
        assert_eq!(old_size, 5);
        assert_eq!(table.size(), 8);
    }

    #[test]
    fn test_global_operations() {
        let mut global = WasmGlobal::new(ValueType::I32, Value::I32(42), true);

        assert_eq!(*global.get(), Value::I32(42));

        assert!(global.set(Value::I32(100)).is_ok());
        assert_eq!(*global.get(), Value::I32(100));

        // Test immutable global
        let mut immutable_global = WasmGlobal::new(ValueType::I32, Value::I32(42), false);
        assert!(immutable_global.set(Value::I32(100)).is_err());
    }

    #[test]
    fn test_instance_state_transitions() {
        let module = create_test_module();
        let security = SecurityContext::default();

        let mut instance = WasmInstance::new(module, security).unwrap();
        assert_eq!(instance.state, InstanceState::Creating);

        instance.initialize().unwrap();
        assert_eq!(instance.state, InstanceState::Ready);

        // Set to executing state before pause
        instance.state = InstanceState::Executing;

        instance.pause().unwrap();
        assert_eq!(instance.state, InstanceState::Paused);

        instance.resume().unwrap();
        assert_eq!(instance.state, InstanceState::Executing);

        instance.terminate();
        assert_eq!(instance.state, InstanceState::Terminated);
        assert!(!instance.is_active());
    }

    #[test]
    fn test_instance_statistics() {
        let module = create_test_module();
        let security = SecurityContext::default();

        let instance = WasmInstance::new(module, security).unwrap();
        let stats = instance.statistics();

        assert_eq!(stats.execution_count, 0);
        assert_eq!(stats.error_count, 0);
    }
}
