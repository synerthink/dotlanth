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

//! Main WASM Runtime

use crate::wasm::{TranspilerConfig, WasmError, WasmExecutionContext, WasmInstance, WasmModule, WasmResult, WasmTranspiler};
use dotvm_compiler::wasm::WasmParser;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use uuid::Uuid;

/// WASM module validator
#[derive(Debug)]
pub struct WasmValidator {
    /// Validation configuration
    config: ValidationConfig,
    /// Validation statistics
    stats: ValidationStats,
}

/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable strict validation
    pub strict_mode: bool,
    /// Maximum function count
    pub max_functions: u32,
    /// Maximum memory pages
    pub max_memory_pages: u32,
    /// Maximum table elements
    pub max_table_elements: u32,
    /// Maximum globals
    pub max_globals: u32,
    /// Maximum imports
    pub max_imports: u32,
    /// Maximum exports
    pub max_exports: u32,
    /// Enable security checks
    pub enable_security_checks: bool,
}

/// Validation statistics
#[derive(Debug, Default)]
pub struct ValidationStats {
    /// Modules validated
    pub modules_validated: u64,
    /// Validation errors
    pub validation_errors: u64,
    /// Security violations
    pub security_violations: u64,
    /// Warnings issued
    pub warnings_issued: u64,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    /// Validation passed
    pub passed: bool,
    /// Errors found
    pub errors: Vec<ValidationError>,
    /// Warnings
    pub warnings: Vec<ValidationWarning>,
    /// Security issues
    pub security_issues: Vec<SecurityIssue>,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error type
    pub error_type: ValidationErrorType,
    /// Error message
    pub message: String,
    /// Location information
    pub location: Option<ValidationLocation>,
}

/// Validation error types
#[derive(Debug, Clone)]
pub enum ValidationErrorType {
    InvalidSignature,
    InvalidSection,
    InvalidInstruction,
    TypeMismatch,
    ResourceLimit,
    SecurityViolation,
    InvalidImport,
    InvalidExport,
    InvalidMemory,
    InvalidTable,
    InvalidGlobal,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,
    /// Location
    pub location: Option<ValidationLocation>,
}

/// Security issue
#[derive(Debug, Clone)]
pub struct SecurityIssue {
    /// Issue severity
    pub severity: SecuritySeverity,
    /// Issue description
    pub description: String,
    /// Location
    pub location: Option<ValidationLocation>,
}

/// Security severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Validation location
#[derive(Debug, Clone)]
pub struct ValidationLocation {
    /// Section name
    pub section: String,
    /// Offset within section
    pub offset: usize,
    /// Function index (if applicable)
    pub function_index: Option<u32>,
}

/// WASM Runtime Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRuntimeConfig {
    /// Maximum number of memory pages (64KB each)
    pub max_memory_pages: u32,
    /// Maximum number of table elements
    pub max_table_elements: u32,
    /// Maximum number of instances
    pub max_instances: u32,
    /// Maximum stack size in bytes
    pub max_stack_size: usize,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Enable JIT compilation
    pub enable_jit: bool,
    /// Enable optimizations
    pub enable_optimizations: bool,
    /// Enable strict validation
    pub strict_validation: bool,
    /// Enable debugging features
    pub enable_debugging: bool,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    /// Enable custom DotVM opcodes in transpiler
    pub enable_custom_opcodes: Option<bool>,
    /// Maximum function call depth
    pub max_call_depth: usize,
    /// Maximum number of locals per function
    pub max_locals_per_function: u32,
    /// Maximum number of globals
    pub max_globals: u32,
    /// Maximum number of imports
    pub max_imports: u32,
    /// Maximum number of exports
    pub max_exports: u32,
    /// Maximum module size in bytes
    pub max_module_size: usize,
    /// Security policy configuration
    pub security: SecurityConfig,
    /// Memory configuration
    pub memory: MemoryConfig,
    /// Execution configuration
    pub execution: ExecutionConfig,
}

/// Security Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable sandbox mode
    pub enable_sandbox: bool,
    /// Allowed system calls
    pub allowed_syscalls: Vec<String>,
    /// Blocked imports
    pub blocked_imports: Vec<String>,
    /// Enable memory protection
    pub enable_memory_protection: bool,
    /// Enable stack protection
    pub enable_stack_protection: bool,
    /// Maximum file descriptor limit
    pub max_file_descriptors: u32,
    /// Enable network access
    pub allow_network_access: bool,
    /// Enable file system access
    pub allow_filesystem_access: bool,
}

/// Memory Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Initial memory size in pages
    pub initial_pages: u32,
    /// Maximum memory size in pages
    pub max_pages: u32,
    /// Enable memory growth
    pub allow_memory_growth: bool,
    /// Memory alignment requirements
    pub alignment: u32,
    /// Enable memory protection
    pub enable_protection: bool,
    /// Memory pool size for reuse
    pub pool_size: usize,
}

/// Execution Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Enable instruction counting
    pub enable_instruction_counting: bool,
    /// Enable execution tracing
    pub enable_tracing: bool,
    /// Maximum number of instructions
    pub max_instructions: u64,
}

/// Main WASM Runtime
#[derive(Debug)]
pub struct DotVMWasmRuntime {
    /// Runtime configuration
    config: WasmRuntimeConfig,
    /// Module cache (integrated loader functionality)
    module_cache: RwLock<HashMap<String, Arc<WasmModule>>>,
    /// WASM to DotVM transpiler
    transpiler: RwLock<WasmTranspiler>,
    /// Loaded modules (integrated store functionality)
    modules: RwLock<HashMap<Uuid, Arc<WasmModule>>>,
    /// Active instances
    instances: RwLock<HashMap<Uuid, Arc<RwLock<WasmInstance>>>>,
    /// Runtime statistics
    stats: RwLock<RuntimeStats>,
    /// Store configuration
    store_config: StoreConfig,
}

/// Runtime statistics
#[derive(Debug, Default)]
pub struct RuntimeStats {
    /// Total modules loaded
    pub modules_loaded: u64,
    /// Total instances created
    pub instances_created: u64,
    /// Total functions executed
    pub functions_executed: u64,
    /// Total execution time
    pub total_execution_time: std::time::Duration,
    /// Memory usage
    pub memory_usage_bytes: u64,
}

/// Store configuration
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Maximum modules in store
    pub max_modules: usize,
    /// Maximum instances in store
    pub max_instances: usize,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
}

/// Store statistics
#[derive(Debug, Clone)]
pub struct StoreStatistics {
    pub module_count: usize,
    pub instance_count: usize,
    pub max_modules: usize,
    pub max_instances: usize,
}

impl DotVMWasmRuntime {
    /// Create new runtime
    pub fn new(config: WasmRuntimeConfig) -> Self {
        let transpiler_config = TranspilerConfig::default();

        Self {
            config,
            module_cache: RwLock::new(HashMap::new()),
            transpiler: RwLock::new(WasmTranspiler::new(transpiler_config)),
            modules: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            stats: RwLock::new(RuntimeStats::default()),
            store_config: StoreConfig::default(),
        }
    }

    /// Load module from bytes
    pub fn load_module(&self, name: String, bytes: &[u8]) -> WasmResult<Arc<WasmModule>> {
        // Generate cache key from content hash
        let cache_key = format!("{:x}", Sha256::digest(bytes));

        // Check cache first
        {
            let cache = self.module_cache.read().unwrap();
            if let Some(cached_module) = cache.get(&cache_key) {
                return Ok(cached_module.clone());
            }
        }

        // Check module size limit
        if bytes.len() > self.config.max_module_size {
            return Err(WasmError::loading_error(format!("Module size {} exceeds limit {}", bytes.len(), self.config.max_module_size)));
        }

        // Parse WASM module using compiler
        let mut parser = WasmParser::new();
        let compiled_module = parser.parse(bytes).map_err(|e| WasmError::loading_error(format!("Failed to parse WASM module: {}", e)))?;

        // Create runtime module
        let mut module = WasmModule::new(name, bytes.to_vec(), compiled_module);

        // Validate module if strict validation is enabled
        if self.config.strict_validation {
            let mut validator = WasmValidator::new(ValidationConfig::default());
            validator.validate(&module).map_err(|e| WasmError::validation_error(format!("Module validation failed: {}", e)))?;
        }

        // Update performance hints
        module.update_performance_hints();

        let module = Arc::new(module);

        // Cache the module
        {
            let mut cache = self.module_cache.write().unwrap();
            cache.insert(cache_key, module.clone());
        }

        // Store module in internal store
        self.add_module_to_store(module.clone())?;

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.modules_loaded += 1;
        }

        Ok(module)
    }

    /// Load module from file
    pub fn load_module_from_file<P: AsRef<Path>>(&self, path: P) -> WasmResult<Arc<WasmModule>> {
        let path = path.as_ref();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

        // Read file contents
        let bytes = std::fs::read(path).map_err(|e| WasmError::loading_error(format!("Failed to read file {:?}: {}", path, e)))?;

        // Use existing load_module logic
        self.load_module(name, &bytes)
    }

    /// Instantiate module
    pub fn instantiate(&self, module: &Arc<WasmModule>) -> WasmResult<Arc<RwLock<WasmInstance>>> {
        let security_context = crate::wasm::management::SecurityContext::default();
        let instance = WasmInstance::new(module.clone(), security_context)?;
        let instance = Arc::new(RwLock::new(instance));

        // Store instance using store functionality
        self.add_instance_to_store(instance.clone())?;

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.instances_created += 1;
        }

        Ok(instance)
    }

    /// Execute function
    pub fn execute(&self, instance: &Arc<RwLock<WasmInstance>>, function: &str, args: &[dotvm_compiler::wasm::ast::WasmValue]) -> WasmResult<Vec<dotvm_compiler::wasm::ast::WasmValue>> {
        let start_time = std::time::Instant::now();

        // Create execution context
        let mut context = WasmExecutionContext::new(self.config.execution.max_instructions, self.config.max_call_depth, self.config.max_execution_time);

        // Execute function directly on instance
        let result = {
            let mut instance_guard = instance.write().unwrap();
            instance_guard.execute_function(function, args, &mut context)
        };

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.functions_executed += 1;
            stats.total_execution_time += start_time.elapsed();
        }

        result
    }

    /// Get instance by ID
    pub fn get_instance(&self, id: Uuid) -> Option<Arc<RwLock<WasmInstance>>> {
        let instances = self.instances.read().unwrap();
        instances.get(&id).cloned()
    }

    /// Remove instance
    pub fn remove_instance(&self, id: Uuid) -> bool {
        let mut instances = self.instances.write().unwrap();
        instances.remove(&id).is_some()
    }

    /// Get runtime statistics
    pub fn statistics(&self) -> RuntimeStatistics {
        let stats = self.stats.read().unwrap();
        let instances = self.instances.read().unwrap();
        let cache = self.module_cache.read().unwrap();

        RuntimeStatistics {
            modules_loaded: stats.modules_loaded,
            instances_created: stats.instances_created,
            active_instances: instances.len() as u64,
            functions_executed: stats.functions_executed,
            total_execution_time: stats.total_execution_time,
            memory_usage_bytes: stats.memory_usage_bytes,
            cached_modules: cache.len() as u64,
        }
    }

    /// Shutdown runtime
    pub fn shutdown(&self) -> WasmResult<()> {
        // Clear all instances
        {
            let mut instances = self.instances.write().unwrap();
            instances.clear();
        }

        // Clear module cache
        {
            let mut cache = self.module_cache.write().unwrap();
            cache.clear();
        }

        // Clear modules store
        {
            let mut modules = self.modules.write().unwrap();
            modules.clear();
        }

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &WasmRuntimeConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: WasmRuntimeConfig) -> WasmResult<()> {
        // Validate new configuration
        config.validate().map_err(|e| WasmError::validation_error(e))?;

        self.config = config;
        Ok(())
    }

    /// Add module to store
    pub fn add_module_to_store(&self, module: Arc<WasmModule>) -> WasmResult<()> {
        let mut modules = self.modules.write().unwrap();

        if modules.len() >= self.store_config.max_modules {
            return Err(WasmError::resource_limit_exceeded("modules".to_string(), modules.len() as u64, self.store_config.max_modules as u64));
        }

        modules.insert(module.id, module);
        Ok(())
    }

    /// Get module from store
    pub fn get_module_from_store(&self, id: Uuid) -> Option<Arc<WasmModule>> {
        let modules = self.modules.read().unwrap();
        modules.get(&id).cloned()
    }

    /// Add instance to store
    pub fn add_instance_to_store(&self, instance: Arc<RwLock<WasmInstance>>) -> WasmResult<()> {
        let mut instances = self.instances.write().unwrap();

        if instances.len() >= self.store_config.max_instances {
            return Err(WasmError::resource_limit_exceeded(
                "instances".to_string(),
                instances.len() as u64,
                self.store_config.max_instances as u64,
            ));
        }

        let id = instance.read().unwrap().id;
        instances.insert(id, instance);
        Ok(())
    }

    /// Remove module from store
    pub fn remove_module_from_store(&self, id: Uuid) -> bool {
        let mut modules = self.modules.write().unwrap();
        modules.remove(&id).is_some()
    }

    /// Get store statistics
    pub fn store_statistics(&self) -> StoreStatistics {
        let modules = self.modules.read().unwrap();
        let instances = self.instances.read().unwrap();

        StoreStatistics {
            module_count: modules.len(),
            instance_count: instances.len(),
            max_modules: self.store_config.max_modules,
            max_instances: self.store_config.max_instances,
        }
    }

    /// Clear all modules and instances from store
    pub fn clear_store(&self) {
        let mut modules = self.modules.write().unwrap();
        let mut instances = self.instances.write().unwrap();
        modules.clear();
        instances.clear();
    }
}

/// Runtime statistics snapshot
#[derive(Debug, Clone)]
pub struct RuntimeStatistics {
    pub modules_loaded: u64,
    pub instances_created: u64,
    pub active_instances: u64,
    pub functions_executed: u64,
    pub total_execution_time: std::time::Duration,
    pub memory_usage_bytes: u64,
    pub cached_modules: u64,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_modules: 1000,
            max_instances: 10000,
            auto_cleanup: true,
        }
    }
}

impl WasmRuntimeConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict configuration for production
    pub fn strict() -> Self {
        Self {
            max_memory_pages: 256, // 16MB
            max_execution_time: Duration::from_secs(10),
            strict_validation: true,
            enable_debugging: false,
            security: SecurityConfig::strict(),
            memory: MemoryConfig::strict(),
            execution: ExecutionConfig::strict(),
            ..Self::default()
        }
    }

    /// Create a development configuration with relaxed limits
    pub fn development() -> Self {
        Self {
            max_memory_pages: 2048, // 128MB
            max_execution_time: Duration::from_secs(300),
            strict_validation: false,
            enable_debugging: true,
            enable_monitoring: true,
            enable_custom_opcodes: Some(true),
            security: SecurityConfig::development(),
            memory: MemoryConfig::development(),
            execution: ExecutionConfig::development(),
            ..Self::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_memory_pages == 0 {
            return Err("max_memory_pages must be greater than 0".to_string());
        }

        if self.max_stack_size == 0 {
            return Err("max_stack_size must be greater than 0".to_string());
        }

        if self.max_call_depth == 0 {
            return Err("max_call_depth must be greater than 0".to_string());
        }

        if self.max_module_size == 0 {
            return Err("max_module_size must be greater than 0".to_string());
        }

        if self.memory.max_pages < self.memory.initial_pages {
            return Err("memory max_pages must be >= initial_pages".to_string());
        }

        if self.execution.max_instructions == 0 {
            return Err("max_instructions must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Get memory size in bytes
    pub fn max_memory_bytes(&self) -> usize {
        self.max_memory_pages as usize * 65536
    }

    /// Check if feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "jit" => self.enable_jit,
            "optimizations" => self.enable_optimizations,
            "debugging" => self.enable_debugging,
            "monitoring" => self.enable_monitoring,
            "sandbox" => self.security.enable_sandbox,
            _ => false,
        }
    }

    /// Get maximum instructions per execution
    pub fn max_instructions_per_execution(&self) -> u64 {
        self.execution.max_instructions
    }
}

impl SecurityConfig {
    /// Create a strict security configuration
    pub fn strict() -> Self {
        Self {
            enable_sandbox: true,
            allowed_syscalls: vec![],
            blocked_imports: vec!["env".to_string(), "wasi_snapshot_preview1".to_string()],
            enable_memory_protection: true,
            enable_stack_protection: true,
            max_file_descriptors: 0,
            allow_network_access: false,
            allow_filesystem_access: false,
        }
    }

    /// Create a development security configuration
    pub fn development() -> Self {
        Self {
            enable_sandbox: false,
            allowed_syscalls: vec!["read".to_string(), "write".to_string()],
            blocked_imports: vec![],
            enable_memory_protection: false,
            enable_stack_protection: false,
            max_file_descriptors: 100,
            allow_network_access: true,
            allow_filesystem_access: true,
        }
    }
}

impl MemoryConfig {
    /// Create a strict memory configuration
    pub fn strict() -> Self {
        Self {
            initial_pages: 8, // 512KB
            max_pages: 256,   // 16MB
            allow_memory_growth: false,
            alignment: 8,
            enable_protection: true,
            pool_size: 5,
        }
    }

    /// Create a development memory configuration
    pub fn development() -> Self {
        Self {
            initial_pages: 32, // 2MB
            max_pages: 4096,   // 256MB
            allow_memory_growth: true,
            alignment: 8,
            enable_protection: false,
            pool_size: 20,
        }
    }
}

impl ExecutionConfig {
    /// Create a strict execution configuration
    pub fn strict() -> Self {
        Self {
            enable_instruction_counting: true,
            enable_tracing: false,
            max_instructions: 1_000_000,
        }
    }

    /// Create a development execution configuration
    pub fn development() -> Self {
        Self {
            enable_instruction_counting: false,
            enable_tracing: true,
            max_instructions: 100_000_000,
        }
    }
}

impl WasmValidator {
    /// Create new validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            stats: ValidationStats::default(),
        }
    }

    /// Validate module
    pub fn validate(&mut self, module: &WasmModule) -> WasmResult<ValidationResult> {
        let mut result = ValidationResult {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            security_issues: Vec::new(),
        };

        // Validate basic structure
        self.validate_structure(module, &mut result)?;

        // Validate resource limits
        self.validate_resource_limits(module, &mut result)?;

        // Validate security constraints
        if self.config.enable_security_checks {
            self.validate_security(module, &mut result)?;
        }

        // Validate function signatures
        self.validate_functions(module, &mut result)?;

        // Validate imports and exports
        self.validate_imports_exports(module, &mut result)?;

        // Update statistics
        self.stats.modules_validated += 1;
        if !result.passed {
            self.stats.validation_errors += 1;
        }
        self.stats.warnings_issued += result.warnings.len() as u64;
        self.stats.security_violations += result.security_issues.len() as u64;

        Ok(result)
    }

    /// Get validation statistics
    pub fn statistics(&self) -> &ValidationStats {
        &self.stats
    }

    // Private validation methods

    fn validate_structure(&self, module: &WasmModule, result: &mut ValidationResult) -> WasmResult<()> {
        // Check magic number and version in bytecode
        if module.bytecode.len() < 8 {
            result.errors.push(ValidationError {
                error_type: ValidationErrorType::InvalidSignature,
                message: "Module too small".to_string(),
                location: None,
            });
            result.passed = false;
        }

        // Check WASM magic number
        if &module.bytecode[0..4] != b"\0asm" {
            result.errors.push(ValidationError {
                error_type: ValidationErrorType::InvalidSignature,
                message: "Invalid WASM magic number".to_string(),
                location: None,
            });
            result.passed = false;
        }

        // Check version
        let version = u32::from_le_bytes([module.bytecode[4], module.bytecode[5], module.bytecode[6], module.bytecode[7]]);

        if version != 1 {
            result.warnings.push(ValidationWarning {
                message: format!("Unsupported WASM version: {}", version),
                location: None,
            });
        }

        Ok(())
    }

    fn validate_resource_limits(&self, module: &WasmModule, result: &mut ValidationResult) -> WasmResult<()> {
        // Check function count
        if module.functions.len() > self.config.max_functions as usize {
            result.errors.push(ValidationError {
                error_type: ValidationErrorType::ResourceLimit,
                message: format!("Too many functions: {} > {}", module.functions.len(), self.config.max_functions),
                location: None,
            });
            result.passed = false;
        }

        // Check memory requirements
        if module.memory_requirements.initial_pages > self.config.max_memory_pages {
            result.errors.push(ValidationError {
                error_type: ValidationErrorType::ResourceLimit,
                message: format!("Memory requirement too high: {} > {}", module.memory_requirements.initial_pages, self.config.max_memory_pages),
                location: None,
            });
            result.passed = false;
        }

        // Check globals count
        if module.globals.len() > self.config.max_globals as usize {
            result.errors.push(ValidationError {
                error_type: ValidationErrorType::ResourceLimit,
                message: format!("Too many globals: {} > {}", module.globals.len(), self.config.max_globals),
                location: None,
            });
            result.passed = false;
        }

        Ok(())
    }

    fn validate_security(&self, module: &WasmModule, result: &mut ValidationResult) -> WasmResult<()> {
        // Check for suspicious imports
        for (name, import) in &module.imports {
            if import.module == "env" && import.name.contains("exec") {
                result.security_issues.push(SecurityIssue {
                    severity: SecuritySeverity::High,
                    description: format!("Suspicious import: {}", name),
                    location: Some(ValidationLocation {
                        section: "import".to_string(),
                        offset: 0,
                        function_index: None,
                    }),
                });
            }
        }

        // Check security level
        if module.security_metadata.security_level > 5 {
            result.security_issues.push(SecurityIssue {
                severity: SecuritySeverity::Medium,
                description: "High security level required".to_string(),
                location: None,
            });
        }

        // Check for unsafe permissions
        for permission in &module.security_metadata.required_permissions {
            if permission.contains("unsafe") || permission.contains("system") {
                result.security_issues.push(SecurityIssue {
                    severity: SecuritySeverity::Critical,
                    description: format!("Unsafe permission: {}", permission),
                    location: None,
                });
            }
        }

        Ok(())
    }

    fn validate_functions(&self, module: &WasmModule, result: &mut ValidationResult) -> WasmResult<()> {
        let mut function_names = HashSet::new();

        for (name, function) in &module.functions {
            // Check for duplicate function names
            if !function_names.insert(name.clone()) {
                result.errors.push(ValidationError {
                    error_type: ValidationErrorType::InvalidExport,
                    message: format!("Duplicate function name: {}", name),
                    location: Some(ValidationLocation {
                        section: "function".to_string(),
                        offset: 0,
                        function_index: Some(function.index),
                    }),
                });
                result.passed = false;
            }

            // Validate function signature
            if function.signature.params.len() > 100 {
                result.warnings.push(ValidationWarning {
                    message: format!("Function {} has many parameters", name),
                    location: Some(ValidationLocation {
                        section: "function".to_string(),
                        offset: 0,
                        function_index: Some(function.index),
                    }),
                });
            }
        }

        Ok(())
    }

    fn validate_imports_exports(&self, module: &WasmModule, result: &mut ValidationResult) -> WasmResult<()> {
        // Check import count
        if module.imports.len() > self.config.max_imports as usize {
            result.errors.push(ValidationError {
                error_type: ValidationErrorType::ResourceLimit,
                message: format!("Too many imports: {} > {}", module.imports.len(), self.config.max_imports),
                location: None,
            });
            result.passed = false;
        }

        // Check export count
        if module.exports.len() > self.config.max_exports as usize {
            result.errors.push(ValidationError {
                error_type: ValidationErrorType::ResourceLimit,
                message: format!("Too many exports: {} > {}", module.exports.len(), self.config.max_exports),
                location: None,
            });
            result.passed = false;
        }

        Ok(())
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            max_functions: 10000,
            max_memory_pages: 1024,
            max_table_elements: 10000,
            max_globals: 1000,
            max_imports: 100,
            max_exports: 100,
            enable_security_checks: true,
        }
    }
}

impl ValidationConfig {
    /// Create strict validation config
    pub fn strict() -> Self {
        Self {
            strict_mode: true,
            max_functions: 1000,
            max_memory_pages: 64,
            max_table_elements: 1000,
            max_globals: 100,
            max_imports: 10,
            max_exports: 10,
            enable_security_checks: true,
        }
    }

    /// Create lenient validation config
    pub fn lenient() -> Self {
        Self {
            strict_mode: false,
            max_functions: 100000,
            max_memory_pages: 10000,
            max_table_elements: 100000,
            max_globals: 10000,
            max_imports: 1000,
            max_exports: 1000,
            enable_security_checks: false,
        }
    }
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 1024, // 64MB
            max_table_elements: 10000,
            max_instances: 100,
            max_stack_size: 1024 * 1024, // 1MB
            max_execution_time: Duration::from_secs(30),
            enable_jit: true,
            enable_optimizations: true,
            strict_validation: true,
            enable_debugging: false,
            enable_monitoring: true,
            enable_custom_opcodes: Some(true),
            max_call_depth: 1024,
            max_locals_per_function: 1024,
            max_globals: 1000,
            max_imports: 1000,
            max_exports: 1000,
            max_module_size: 10 * 1024 * 1024, // 10MB
            security: SecurityConfig::default(),
            memory: MemoryConfig::default(),
            execution: ExecutionConfig::default(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_sandbox: true,
            allowed_syscalls: vec![],
            blocked_imports: vec![],
            enable_memory_protection: true,
            enable_stack_protection: true,
            max_file_descriptors: 10,
            allow_network_access: false,
            allow_filesystem_access: false,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            initial_pages: 16, // 1MB
            max_pages: 1024,   // 64MB
            allow_memory_growth: true,
            alignment: 8,
            enable_protection: true,
            pool_size: 10,
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            enable_instruction_counting: true,
            enable_tracing: false,
            max_instructions: 10_000_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_runtime() -> DotVMWasmRuntime {
        DotVMWasmRuntime::new(WasmRuntimeConfig::default())
    }

    fn create_minimal_wasm() -> Vec<u8> {
        vec![
            0x00, 0x61, 0x73, 0x6d, // Magic number
            0x01, 0x00, 0x00, 0x00, // Version
        ]
    }

    #[test]
    fn test_runtime_creation() {
        let runtime = create_test_runtime();
        let stats = runtime.statistics();
        assert_eq!(stats.modules_loaded, 0);
        assert_eq!(stats.instances_created, 0);
    }

    #[test]
    fn test_runtime_shutdown() {
        let runtime = create_test_runtime();
        assert!(runtime.shutdown().is_ok());
    }

    #[test]
    fn test_instance_management() {
        let runtime = create_test_runtime();
        let instances = runtime.instances.read().unwrap();
        assert_eq!(instances.len(), 0);
    }

    #[test]
    fn test_statistics() {
        let runtime = create_test_runtime();
        let stats = runtime.statistics();

        assert_eq!(stats.active_instances, 0);
        assert_eq!(stats.functions_executed, 0);
    }

    #[test]
    fn test_store_functionality() {
        let runtime = create_test_runtime();
        let store_stats = runtime.store_statistics();
        assert_eq!(store_stats.module_count, 0);
        assert_eq!(store_stats.instance_count, 0);
        assert_eq!(store_stats.max_modules, 1000);
        assert_eq!(store_stats.max_instances, 10000);
    }

    #[test]
    fn test_store_clear() {
        let runtime = create_test_runtime();
        runtime.clear_store();
        let store_stats = runtime.store_statistics();
        assert_eq!(store_stats.module_count, 0);
        assert_eq!(store_stats.instance_count, 0);
    }

    #[test]
    fn test_store_config() {
        let config = StoreConfig::default();
        assert_eq!(config.max_modules, 1000);
        assert_eq!(config.max_instances, 10000);
        assert!(config.auto_cleanup);
    }

    #[test]
    fn test_config_access() {
        let runtime = create_test_runtime();
        let config = runtime.config();
    }

    #[test]
    fn test_max_instructions_calculation() {
        let config = WasmRuntimeConfig::default();
        let max_instructions = config.max_instructions_per_execution();
        assert!(max_instructions > 0);
    }

    #[test]
    fn test_validator_creation() {
        let config = ValidationConfig::default();
        let validator = WasmValidator::new(config);
        assert_eq!(validator.stats.modules_validated, 0);
    }

    #[test]
    fn test_validation_config_presets() {
        let strict = ValidationConfig::strict();
        assert!(strict.strict_mode);
        assert_eq!(strict.max_functions, 1000);

        let lenient = ValidationConfig::lenient();
        assert!(!lenient.strict_mode);
        assert_eq!(lenient.max_functions, 100000);
    }

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError {
            error_type: ValidationErrorType::InvalidSignature,
            message: "Test error".to_string(),
            location: None,
        };

        match error.error_type {
            ValidationErrorType::InvalidSignature => (),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_security_severity_ordering() {
        assert!(SecuritySeverity::Critical > SecuritySeverity::High);
        assert!(SecuritySeverity::High > SecuritySeverity::Medium);
        assert!(SecuritySeverity::Medium > SecuritySeverity::Low);
    }

    #[test]
    fn test_validation_location() {
        let location = ValidationLocation {
            section: "function".to_string(),
            offset: 42,
            function_index: Some(1),
        };

        assert_eq!(location.section, "function");
        assert_eq!(location.offset, 42);
        assert_eq!(location.function_index, Some(1));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            security_issues: Vec::new(),
        };

        assert!(result.passed);
        assert!(result.errors.is_empty());

        result.errors.push(ValidationError {
            error_type: ValidationErrorType::TypeMismatch,
            message: "Test".to_string(),
            location: None,
        });

        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_default_config() {
        let config = WasmRuntimeConfig::default();
        assert_eq!(config.max_memory_pages, 1024);
        assert_eq!(config.max_stack_size, 1024 * 1024);
        assert!(config.enable_jit);
        assert!(config.strict_validation);
    }

    #[test]
    fn test_strict_config() {
        let config = WasmRuntimeConfig::strict();
        assert_eq!(config.max_memory_pages, 256);
        assert_eq!(config.max_execution_time, Duration::from_secs(10));
        assert!(config.security.enable_sandbox);
        assert!(!config.security.allow_network_access);
    }

    #[test]
    fn test_development_config() {
        let config = WasmRuntimeConfig::development();
        assert_eq!(config.max_memory_pages, 2048);
        assert!(config.enable_debugging);
        assert!(!config.security.enable_sandbox);
        assert!(config.security.allow_network_access);
    }

    #[test]
    fn test_config_validation() {
        let mut config = WasmRuntimeConfig::default();
        assert!(config.validate().is_ok());

        config.max_memory_pages = 0;
        assert!(config.validate().is_err());

        config.max_memory_pages = 1024;
        config.memory.max_pages = 5;
        config.memory.initial_pages = 10;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_memory_bytes_calculation() {
        let config = WasmRuntimeConfig::default();
        assert_eq!(config.max_memory_bytes(), 1024 * 65536);
    }

    #[test]
    fn test_feature_enabled() {
        let config = WasmRuntimeConfig::default();
        assert!(config.is_feature_enabled("jit"));
        assert!(config.is_feature_enabled("monitoring"));
        assert!(!config.is_feature_enabled("unknown_feature"));
    }

    #[test]
    fn test_security_configs() {
        let strict = SecurityConfig::strict();
        assert!(strict.enable_sandbox);
        assert!(!strict.allow_network_access);
        assert_eq!(strict.max_file_descriptors, 0);

        let dev = SecurityConfig::development();
        assert!(!dev.enable_sandbox);
        assert!(dev.allow_network_access);
        assert_eq!(dev.max_file_descriptors, 100);
    }

    #[test]
    fn test_memory_configs() {
        let strict = MemoryConfig::strict();
        assert_eq!(strict.initial_pages, 8);
        assert_eq!(strict.max_pages, 256);
        assert!(!strict.allow_memory_growth);

        let dev = MemoryConfig::development();
        assert_eq!(dev.initial_pages, 32);
        assert_eq!(dev.max_pages, 4096);
        assert!(dev.allow_memory_growth);
    }

    #[test]
    fn test_execution_configs() {
        let strict = ExecutionConfig::strict();
        assert_eq!(strict.max_instructions, 1_000_000);
        assert_eq!(strict.max_instructions, 1_000_000);

        let dev = ExecutionConfig::development();
        assert!(dev.enable_tracing);
        assert_eq!(dev.max_instructions, 100_000_000);
    }
}
