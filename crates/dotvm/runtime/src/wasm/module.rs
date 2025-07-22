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

//! WASM Module Implementation

use crate::wasm::execution::{FunctionSignature, ValueType};
use crate::wasm::{WasmError, WasmResult};
use dotvm_compiler::wasm::{WasmExport, WasmFunction, WasmImport, WasmModule as CompilerModule};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Runtime WASM Module
#[derive(Debug, Clone)]
pub struct WasmModule {
    /// Unique module identifier
    pub id: Uuid,
    /// Module name
    pub name: String,
    /// Module bytecode
    pub bytecode: Vec<u8>,
    /// Compiled module from compiler
    pub compiled: CompilerModule,
    /// Function metadata
    pub functions: HashMap<String, FunctionMetadata>,
    /// Export metadata
    pub exports: HashMap<String, ExportMetadata>,
    /// Import requirements
    pub imports: HashMap<String, ImportMetadata>,
    /// Memory requirements
    pub memory_requirements: MemoryRequirements,
    /// Table requirements
    pub table_requirements: Vec<TableRequirement>,
    /// Global variables
    pub globals: Vec<GlobalMetadata>,
    /// Module validation status
    pub validation_status: ValidationStatus,
    /// Security metadata
    pub security_metadata: SecurityMetadata,
    /// Performance hints
    pub performance_hints: PerformanceHints,
}

/// Function metadata
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Function index
    pub index: u32,
    /// Function name (if available)
    pub name: Option<String>,
    /// Function signature
    pub signature: FunctionSignature,
    /// Local variable count
    pub local_count: u32,
    /// Estimated instruction count
    pub instruction_count: u64,
    /// Gas cost estimate
    pub gas_cost_estimate: u64,
    /// Security level required
    pub security_level: u8,
    /// Performance characteristics
    pub performance_flags: u32,
}

/// Export metadata
#[derive(Debug, Clone)]
pub struct ExportMetadata {
    /// Export name
    pub name: String,
    /// Export type
    pub export_type: ExportType,
    /// Export index
    pub index: u32,
    /// Security permissions required
    pub permissions: Vec<String>,
}

/// Export type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ExportType {
    Function,
    Memory,
    Table,
    Global,
}

/// Import metadata
#[derive(Debug, Clone)]
pub struct ImportMetadata {
    /// Module name
    pub module: String,
    /// Import name
    pub name: String,
    /// Import type
    pub import_type: ImportType,
    /// Required signature (for functions)
    pub signature: Option<FunctionSignature>,
    /// Security requirements
    pub security_requirements: Vec<String>,
}

/// Import type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ImportType {
    Function,
    Memory,
    Table,
    Global,
}

/// Memory requirements
#[derive(Debug, Clone, Default)]
pub struct MemoryRequirements {
    /// Initial memory size in pages
    pub initial_pages: u32,
    /// Maximum memory size in pages
    pub max_pages: Option<u32>,
    /// Shared memory flag
    pub shared: bool,
    /// Memory alignment requirements
    pub alignment: u32,
    /// Memory protection flags
    pub protection_flags: u32,
}

/// Table requirement
#[derive(Debug, Clone)]
pub struct TableRequirement {
    /// Element type
    pub element_type: ValueType,
    /// Initial size
    pub initial: u32,
    /// Maximum size
    pub maximum: Option<u32>,
    /// Table name
    pub name: Option<String>,
}

/// Global metadata
#[derive(Debug, Clone)]
pub struct GlobalMetadata {
    /// Global index
    pub index: u32,
    /// Global name
    pub name: Option<String>,
    /// Value type
    pub value_type: ValueType,
    /// Mutability
    pub mutable: bool,
    /// Initial value
    pub initial_value: GlobalValue,
}

/// Global value
#[derive(Debug, Clone)]
pub enum GlobalValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128([u8; 16]),
    FuncRef(Option<u32>),
    ExternRef(Option<u32>),
}

/// Validation status
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationStatus {
    NotValidated,
    Valid,
    Invalid { errors: Vec<String> },
    PartiallyValid { warnings: Vec<String> },
}

/// Security metadata
#[derive(Debug, Clone, Default)]
pub struct SecurityMetadata {
    /// Required permissions
    pub required_permissions: Vec<String>,
    /// Security level
    pub security_level: u8,
    /// Sandbox requirements
    pub sandbox_required: bool,
    /// Resource limits
    pub resource_limits: HashMap<String, u64>,
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Blocked operations
    pub blocked_operations: Vec<String>,
}

/// Performance hints
#[derive(Debug, Clone, Default)]
pub struct PerformanceHints {
    /// JIT compilation recommended
    pub jit_recommended: bool,
    /// Optimization level
    pub optimization_level: u8,
    /// Memory usage estimate
    pub memory_usage_estimate: u64,
    /// Execution time estimate
    pub execution_time_estimate: u64,
    /// CPU intensive flag
    pub cpu_intensive: bool,
    /// Memory intensive flag
    pub memory_intensive: bool,
}

impl WasmModule {
    /// Create a new WASM module
    pub fn new(name: String, bytecode: Vec<u8>, compiled: CompilerModule) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            bytecode,
            compiled,
            functions: HashMap::new(),
            exports: HashMap::new(),
            imports: HashMap::new(),
            memory_requirements: MemoryRequirements::default(),
            table_requirements: Vec::new(),
            globals: Vec::new(),
            validation_status: ValidationStatus::NotValidated,
            security_metadata: SecurityMetadata::default(),
            performance_hints: PerformanceHints::default(),
        }
    }

    /// Validate the module
    pub fn validate(&mut self) -> WasmResult<()> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate bytecode size
        if self.bytecode.is_empty() {
            errors.push("Empty bytecode".to_string());
        }

        // Validate function signatures
        for (name, func) in &self.functions {
            if func.signature.params.len() > 1000 {
                warnings.push(format!("Function {} has many parameters", name));
            }
        }

        // Validate memory requirements
        if let Some(max_pages) = self.memory_requirements.max_pages {
            if max_pages < self.memory_requirements.initial_pages {
                errors.push("Max memory pages less than initial pages".to_string());
            }
        }

        // Validate imports
        for (name, import) in &self.imports {
            if import.module.is_empty() {
                errors.push(format!("Import {} has empty module name", name));
            }
        }

        // Set validation status
        self.validation_status = if !errors.is_empty() {
            ValidationStatus::Invalid { errors }
        } else if !warnings.is_empty() {
            ValidationStatus::PartiallyValid { warnings }
        } else {
            ValidationStatus::Valid
        };

        if matches!(self.validation_status, ValidationStatus::Invalid { .. }) {
            return Err(WasmError::validation_error("Module validation failed"));
        }

        Ok(())
    }

    /// Check if module is valid
    pub fn is_valid(&self) -> bool {
        matches!(self.validation_status, ValidationStatus::Valid | ValidationStatus::PartiallyValid { .. })
    }

    /// Get function by name
    pub fn get_function(&self, name: &str) -> Option<&FunctionMetadata> {
        self.functions.get(name)
    }

    /// Get export by name
    pub fn get_export(&self, name: &str) -> Option<&ExportMetadata> {
        self.exports.get(name)
    }

    /// Get import by name
    pub fn get_import(&self, name: &str) -> Option<&ImportMetadata> {
        self.imports.get(name)
    }

    /// Add function metadata
    pub fn add_function(&mut self, name: String, metadata: FunctionMetadata) {
        self.functions.insert(name, metadata);
    }

    /// Add export metadata
    pub fn add_export(&mut self, name: String, metadata: ExportMetadata) {
        self.exports.insert(name, metadata);
    }

    /// Add import metadata
    pub fn add_import(&mut self, name: String, metadata: ImportMetadata) {
        self.imports.insert(name, metadata);
    }

    /// Get memory size in bytes
    pub fn memory_size_bytes(&self) -> usize {
        self.memory_requirements.initial_pages as usize * 65536
    }

    /// Get maximum memory size in bytes
    pub fn max_memory_size_bytes(&self) -> Option<usize> {
        self.memory_requirements.max_pages.map(|pages| pages as usize * 65536)
    }

    /// Check if module requires specific permission
    pub fn requires_permission(&self, permission: &str) -> bool {
        self.security_metadata.required_permissions.contains(&permission.to_string())
    }

    /// Get estimated gas cost
    pub fn estimated_gas_cost(&self) -> u64 {
        self.functions.values().map(|f| f.gas_cost_estimate).sum()
    }

    /// Get module statistics
    pub fn statistics(&self) -> ModuleStatistics {
        ModuleStatistics {
            function_count: self.functions.len(),
            export_count: self.exports.len(),
            import_count: self.imports.len(),
            global_count: self.globals.len(),
            bytecode_size: self.bytecode.len(),
            estimated_memory_usage: self.memory_size_bytes(),
            estimated_gas_cost: self.estimated_gas_cost(),
        }
    }

    /// Update performance hints based on analysis
    pub fn update_performance_hints(&mut self) {
        let stats = self.statistics();

        // Determine if JIT is recommended
        self.performance_hints.jit_recommended = stats.function_count > 10 || stats.estimated_gas_cost > 1_000_000;

        // Set optimization level
        self.performance_hints.optimization_level = if stats.bytecode_size > 1_000_000 { 3 } else { 1 };

        // Set resource usage flags
        self.performance_hints.memory_intensive = stats.estimated_memory_usage > 10 * 1024 * 1024; // 10MB
        self.performance_hints.cpu_intensive = stats.estimated_gas_cost > 10_000_000;

        // Set estimates
        self.performance_hints.memory_usage_estimate = stats.estimated_memory_usage as u64;
        self.performance_hints.execution_time_estimate = stats.estimated_gas_cost / 1000; // Rough estimate
    }
}

/// Module statistics
#[derive(Debug, Clone)]
pub struct ModuleStatistics {
    pub function_count: usize,
    pub export_count: usize,
    pub import_count: usize,
    pub global_count: usize,
    pub bytecode_size: usize,
    pub estimated_memory_usage: usize,
    pub estimated_gas_cost: u64,
}

impl FunctionMetadata {
    /// Create new function metadata
    pub fn new(index: u32, signature: FunctionSignature) -> Self {
        Self {
            index,
            name: None,
            signature,
            local_count: 0,
            instruction_count: 0,
            gas_cost_estimate: 0,
            security_level: 0,
            performance_flags: 0,
        }
    }

    /// Set function name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set local count
    pub fn with_locals(mut self, count: u32) -> Self {
        self.local_count = count;
        self
    }

    /// Estimate gas cost based on instruction count
    pub fn estimate_gas_cost(&mut self) {
        self.gas_cost_estimate = self.instruction_count * 2 + self.local_count as u64 * 10;
    }
}

impl ExportMetadata {
    /// Create new export metadata
    pub fn new(name: String, export_type: ExportType, index: u32) -> Self {
        Self {
            name,
            export_type,
            index,
            permissions: Vec::new(),
        }
    }

    /// Add required permission
    pub fn with_permission(mut self, permission: String) -> Self {
        self.permissions.push(permission);
        self
    }
}

impl ImportMetadata {
    /// Create new import metadata
    pub fn new(module: String, name: String, import_type: ImportType) -> Self {
        Self {
            module,
            name,
            import_type,
            signature: None,
            security_requirements: Vec::new(),
        }
    }

    /// Set function signature
    pub fn with_signature(mut self, signature: FunctionSignature) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Add security requirement
    pub fn with_security_requirement(mut self, requirement: String) -> Self {
        self.security_requirements.push(requirement);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotvm_compiler::wasm::WasmModule as CompilerModule;

    fn create_test_module() -> WasmModule {
        let compiled = CompilerModule::new();
        WasmModule::new("test_module".to_string(), vec![0, 1, 2, 3], compiled)
    }

    #[test]
    fn test_module_creation() {
        let module = create_test_module();
        assert_eq!(module.name, "test_module");
        assert_eq!(module.bytecode, vec![0, 1, 2, 3]);
        assert_eq!(module.validation_status, ValidationStatus::NotValidated);
    }

    #[test]
    fn test_module_validation() {
        let mut module = create_test_module();
        assert!(module.validate().is_ok());
        assert!(module.is_valid());
    }

    #[test]
    fn test_function_metadata() {
        let mut module = create_test_module();

        let func_meta = FunctionMetadata::new(0, FunctionSignature::new(vec![ValueType::I32], vec![ValueType::I32]))
            .with_name("test_func".to_string())
            .with_locals(5);

        module.add_function("test_func".to_string(), func_meta);

        let retrieved = module.get_function("test_func").unwrap();
        assert_eq!(retrieved.index, 0);
        assert_eq!(retrieved.name, Some("test_func".to_string()));
        assert_eq!(retrieved.local_count, 5);
    }

    #[test]
    fn test_export_metadata() {
        let mut module = create_test_module();

        let export_meta = ExportMetadata::new("main".to_string(), ExportType::Function, 0).with_permission("execute".to_string());

        module.add_export("main".to_string(), export_meta);

        let retrieved = module.get_export("main").unwrap();
        assert_eq!(retrieved.name, "main");
        assert_eq!(retrieved.export_type, ExportType::Function);
        assert!(retrieved.permissions.contains(&"execute".to_string()));
    }

    #[test]
    fn test_import_metadata() {
        let mut module = create_test_module();

        let import_meta = ImportMetadata::new("env".to_string(), "print".to_string(), ImportType::Function)
            .with_signature(FunctionSignature::new(vec![ValueType::I32], vec![]))
            .with_security_requirement("io_access".to_string());

        module.add_import("env.print".to_string(), import_meta);

        let retrieved = module.get_import("env.print").unwrap();
        assert_eq!(retrieved.module, "env");
        assert_eq!(retrieved.name, "print");
        assert_eq!(retrieved.import_type, ImportType::Function);
        assert!(retrieved.security_requirements.contains(&"io_access".to_string()));
    }

    #[test]
    fn test_memory_requirements() {
        let module = create_test_module();
        assert_eq!(module.memory_size_bytes(), 0);
        assert_eq!(module.max_memory_size_bytes(), None);
    }

    #[test]
    fn test_module_statistics() {
        let mut module = create_test_module();

        // Add some metadata
        module.add_function("func1".to_string(), FunctionMetadata::new(0, FunctionSignature::new(vec![], vec![])));
        module.add_export("main".to_string(), ExportMetadata::new("main".to_string(), ExportType::Function, 0));
        module.add_import("env.print".to_string(), ImportMetadata::new("env".to_string(), "print".to_string(), ImportType::Function));

        let stats = module.statistics();
        assert_eq!(stats.function_count, 1);
        assert_eq!(stats.export_count, 1);
        assert_eq!(stats.import_count, 1);
        assert_eq!(stats.bytecode_size, 4);
    }

    #[test]
    fn test_performance_hints_update() {
        let mut module = create_test_module();

        // Add many functions to trigger JIT recommendation
        for i in 0..15 {
            let mut func_meta = FunctionMetadata::new(i, FunctionSignature::new(vec![], vec![]));
            func_meta.gas_cost_estimate = 1_000_000; // Higher cost to trigger cpu_intensive
            module.add_function(format!("func{}", i), func_meta);
        }

        module.update_performance_hints();
        assert!(module.performance_hints.jit_recommended);
        assert!(module.performance_hints.cpu_intensive);
    }

    #[test]
    fn test_validation_with_errors() {
        let compiled = CompilerModule::new();
        let mut module = WasmModule::new("test".to_string(), vec![], compiled); // Empty bytecode

        let result = module.validate();
        assert!(result.is_err());
        assert!(matches!(module.validation_status, ValidationStatus::Invalid { .. }));
        assert!(!module.is_valid());
    }

    #[test]
    fn test_gas_cost_estimation() {
        let mut func_meta = FunctionMetadata::new(0, FunctionSignature::new(vec![], vec![]));
        func_meta.instruction_count = 100;
        func_meta.local_count = 5;
        func_meta.estimate_gas_cost();

        assert_eq!(func_meta.gas_cost_estimate, 100 * 2 + 5 * 10);
    }

    #[test]
    fn test_permission_checking() {
        let mut module = create_test_module();
        module.security_metadata.required_permissions.push("network_access".to_string());

        assert!(module.requires_permission("network_access"));
        assert!(!module.requires_permission("file_access"));
    }
}
