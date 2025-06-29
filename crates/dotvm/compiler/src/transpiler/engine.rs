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

//! Core transpilation engine for converting WASM to DotVM bytecode
//!
//! This module provides the main transpilation logic that converts WebAssembly
//! modules into DotVM bytecode, handling control flow translation, memory model
//! adaptation, and architecture-specific optimizations.

use crate::wasm::{
    ast::{WasmFunction, WasmInstruction, WasmModule},
    opcode_mapper::{OpcodeMapper, OpcodeMappingError},
    parser::{WasmParseError, WasmParser},
};
use dotvm_core::bytecode::{BytecodeHeader, VmArchitecture};
use thiserror::Error;

/// Errors that can occur during transpilation
#[derive(Error, Debug)]
pub enum TranspilationError {
    #[error("WASM parsing error: {0}")]
    ParseError(#[from] WasmParseError),
    #[error("Opcode mapping error: {0}")]
    MappingError(#[from] OpcodeMappingError),
    #[error("Control flow error: {0}")]
    ControlFlowError(String),
    #[error("Memory model incompatibility: {0}")]
    MemoryModelError(String),
    #[error("Function not found: {0}")]
    FunctionNotFound(u32),
    #[error("Type mismatch in function {function}: {details}")]
    TypeMismatch { function: u32, details: String },
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
    #[error("Architecture incompatibility: {0}")]
    ArchitectureIncompatibility(String),
}

/// Represents a transpiled DotVM function
#[derive(Debug, Clone)]
pub struct TranspiledFunction {
    /// Function name
    pub name: String,
    /// DotVM bytecode instructions
    pub instructions: Vec<TranspiledInstruction>,
    /// Parameter count
    pub param_count: usize,
    /// Local variable count
    pub local_count: usize,
    /// Whether this function is exported
    pub is_exported: bool,
    /// Debug information (source file, line numbers, etc.)
    pub debug_info: Option<String>,
}

/// A single transpiled instruction with additional information
#[derive(Debug, Clone)]
pub struct TranspiledInstruction {
    /// Opcode string
    pub opcode: String,
    /// Operands
    pub operands: Vec<Operand>,
    /// Optional label for this instruction
    pub label: Option<String>,
}

/// Operand types for instructions
#[derive(Debug, Clone)]
pub enum Operand {
    /// Immediate value
    Immediate(u32),
    /// Register reference
    Register(u16),
    /// Label reference for jumps
    Label(String),
    /// Memory reference with base register and offset
    Memory { base: u16, offset: u32 },
}

/// Local variable information
#[derive(Debug, Clone)]
pub struct LocalVariable {
    /// Variable index
    pub index: u32,
    /// Variable type
    pub var_type: VariableType,
    /// Whether the variable is a parameter
    pub is_parameter: bool,
}

/// Variable types in DotVM
#[derive(Debug, Clone, Copy)]
pub enum VariableType {
    I32,
    I64,
    F32,
    F64,
    V128,
    Pointer,
}

/// Function metadata for optimization and debugging
#[derive(Debug, Clone, Default)]
pub struct FunctionMetadata {
    /// Whether the function uses complex control flow
    pub has_complex_control_flow: bool,
    /// Maximum stack depth required
    pub max_stack_depth: u32,
    /// Memory access patterns
    pub memory_accesses: Vec<MemoryAccessPattern>,
    /// Function calls made by this function
    pub function_calls: Vec<u32>,
    /// Whether the function is recursive
    pub is_recursive: bool,
}

/// Memory access pattern for optimization
#[derive(Debug, Clone)]
pub struct MemoryAccessPattern {
    pub offset: u64,
    pub size: u32,
    pub is_write: bool,
    pub frequency: u32,
}

/// Complete transpiled module
#[derive(Debug, Clone)]
pub struct TranspiledModule {
    /// Module header with architecture information
    pub header: BytecodeHeader,
    /// Transpiled functions
    pub functions: Vec<TranspiledFunction>,
    /// Global variables
    pub globals: Vec<GlobalVariable>,
    /// Memory layout information
    pub memory_layout: MemoryLayout,
    /// Export information
    pub exports: Vec<ExportInfo>,
    /// Import information
    pub imports: Vec<ImportInfo>,
}

/// Global variable information
#[derive(Debug, Clone)]
pub struct GlobalVariable {
    pub index: u32,
    pub var_type: VariableType,
    pub is_mutable: bool,
    pub initial_value: Option<u64>,
}

/// Memory layout information
#[derive(Debug, Clone)]
pub struct MemoryLayout {
    pub initial_pages: u32,
    pub maximum_pages: Option<u32>,
    pub page_size: u32,
}

/// Export information
#[derive(Debug, Clone)]
pub struct ExportInfo {
    pub name: String,
    pub kind: ExportKind,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportKind {
    Function,
    Memory,
    Global,
    Table,
}

/// Import information
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub name: String,
    pub module_name: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ImportKind {
    Function { type_index: u32 },
    Memory,
    Global,
    Table,
}

impl ImportKind {
    /// Convert to u8 for serialization
    pub fn to_u8(&self) -> u8 {
        match self {
            ImportKind::Function { .. } => 0,
            ImportKind::Memory => 1,
            ImportKind::Global => 2,
            ImportKind::Table => 3,
        }
    }
}

/// Configuration for the transpilation process
#[derive(Debug, Clone)]
pub struct TranspilationConfig {
    /// Target architecture for the generated bytecode
    pub target_architecture: VmArchitecture,
    /// Whether to enable optimizations
    pub enable_optimizations: bool,
    /// Whether to preserve debug information
    pub preserve_debug_info: bool,
    /// Maximum function size before splitting
    pub max_function_size: Option<u32>,
    /// Whether to enable architecture-specific features
    pub enable_arch_features: bool,
}

impl Default for TranspilationConfig {
    fn default() -> Self {
        Self {
            target_architecture: VmArchitecture::Arch64,
            enable_optimizations: true,
            preserve_debug_info: false,
            max_function_size: Some(65536), // 64KB
            enable_arch_features: true,
        }
    }
}

/// Main transpilation engine
pub struct TranspilationEngine {
    /// Configuration for transpilation
    config: TranspilationConfig,
    /// WASM parser
    parser: WasmParser,
    /// Opcode mapper
    mapper: OpcodeMapper,
    /// Control flow analyzer
    control_flow_analyzer: ControlFlowAnalyzer,
}

impl TranspilationEngine {
    /// Create a new transpilation engine with the given configuration
    pub fn new(config: TranspilationConfig) -> Self {
        let parser = WasmParser::new();
        let mapper = OpcodeMapper::new(config.target_architecture);
        let control_flow_analyzer = ControlFlowAnalyzer::new();

        Self {
            config,
            parser,
            mapper,
            control_flow_analyzer,
        }
    }

    /// Create a new transpilation engine with default configuration for the given architecture
    pub fn with_architecture(target_arch: VmArchitecture) -> Self {
        let config = TranspilationConfig {
            target_architecture: target_arch,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Transpile a WASM module directly (convenience method for testing)
    pub fn transpile_module(&mut self, module: WasmModule) -> Result<TranspiledModule, TranspilationError> {
        // Analyze architecture requirements
        let required_arch = self.analyze_architecture_requirements(&module)?;
        if !self.is_architecture_compatible(required_arch) {
            return Err(TranspilationError::ArchitectureIncompatibility(format!(
                "Module requires {:?} but target is {:?}",
                required_arch, self.config.target_architecture
            )));
        }

        // Create module header
        let header = BytecodeHeader::new(self.config.target_architecture);

        // Transpile functions
        let functions = self.transpile_functions(&module)?;

        // Process globals
        let globals = self.process_globals(&module)?;

        // Process memory layout
        let memory_layout = self.process_memory_layout(&module)?;

        // Process exports and imports
        let exports = self.process_exports(&module)?;
        let imports = self.process_imports(&module)?;

        Ok(TranspiledModule {
            header,
            functions,
            globals,
            memory_layout,
            exports,
            imports,
        })
    }

    /// Transpile a WASM binary to DotVM bytecode
    pub fn transpile(&mut self, wasm_bytes: &[u8]) -> Result<TranspiledModule, TranspilationError> {
        // Parse WASM binary
        let wasm_module = self.parser.parse(wasm_bytes)?;

        // Analyze architecture requirements
        let required_arch = self.analyze_architecture_requirements(&wasm_module)?;
        if !self.is_architecture_compatible(required_arch) {
            return Err(TranspilationError::ArchitectureIncompatibility(format!(
                "Module requires {:?} but target is {:?}",
                required_arch, self.config.target_architecture
            )));
        }

        // Create module header
        let header = BytecodeHeader::new(self.config.target_architecture);

        // Transpile functions
        let functions = self.transpile_functions(&wasm_module)?;

        // Process globals
        let globals = self.process_globals(&wasm_module)?;

        // Process memory layout
        let memory_layout = self.process_memory_layout(&wasm_module)?;

        // Process exports and imports
        let exports = self.process_exports(&wasm_module)?;
        let imports = self.process_imports(&wasm_module)?;

        Ok(TranspiledModule {
            header,
            functions,
            globals,
            memory_layout,
            exports,
            imports,
        })
    }

    /// Transpile all functions in the WASM module
    fn transpile_functions(&mut self, wasm_module: &WasmModule) -> Result<Vec<TranspiledFunction>, TranspilationError> {
        let mut transpiled_functions = Vec::new();

        for (index, wasm_function) in wasm_module.functions.iter().enumerate() {
            let transpiled = self.transpile_function(index as u32, wasm_function)?;
            transpiled_functions.push(transpiled);
        }
        Ok(transpiled_functions)
    }

    /// Transpile a single WASM function
    fn transpile_function(&mut self, index: u32, wasm_function: &WasmFunction) -> Result<TranspiledFunction, TranspilationError> {
        let mut instructions = Vec::new();

        // Process each instruction
        for wasm_instruction in &wasm_function.body {
            let mapped_instructions = self.mapper.map_instruction(wasm_instruction)?;

            for mapped in mapped_instructions {
                let transpiled = TranspiledInstruction {
                    opcode: format!("{:?}", mapped.opcode), // Convert to string representation
                    operands: mapped
                        .operands
                        .iter()
                        .map(|&op| {
                            // Convert u64 operands to proper Operand enum
                            // For now, treat all as immediate values
                            // TODO: Implement proper operand type detection based on instruction context
                            if op <= u32::MAX as u64 {
                                Operand::Immediate(op as u32)
                            } else {
                                // For large values, we might need to split or handle differently
                                Operand::Immediate((op & 0xFFFFFFFF) as u32)
                            }
                        })
                        .collect(),
                    label: None, // TODO: Extract labels from control flow analysis
                };

                instructions.push(transpiled);
            }
        }

        // Calculate local and parameter counts
        let param_count = wasm_function.signature.params.len();
        let local_count = param_count + wasm_function.locals.len();

        Ok(TranspiledFunction {
            name: format!("func_{index}"), // Generate a name since WasmFunction doesn't have one
            instructions,
            param_count,
            local_count,
            is_exported: false, // Will be set during export processing
            debug_info: if self.config.preserve_debug_info { Some(format!("wasm_function_{index}")) } else { None },
        })
    }

    // Note: Metadata and local variable processing methods removed for simplified structure
    // These would be re-added in a more complete implementation that tracks optimization metadata

    /// Analyze architecture requirements for the WASM module
    fn analyze_architecture_requirements(&self, wasm_module: &WasmModule) -> Result<VmArchitecture, TranspilationError> {
        let mut required_arch = VmArchitecture::Arch64;

        // Analyze all functions for architecture requirements
        for function in &wasm_module.functions {
            for instruction in &function.body {
                let inst_arch = OpcodeMapper::required_architecture(instruction);
                if (inst_arch as u8) > (required_arch as u8) {
                    required_arch = inst_arch;
                }
            }
        }

        Ok(required_arch)
    }

    /// Check if the required architecture is compatible with the target
    fn is_architecture_compatible(&self, required: VmArchitecture) -> bool {
        (self.config.target_architecture as u8) >= (required as u8)
    }

    /// Process global variables (simplified for now)
    fn process_globals(&self, wasm_module: &WasmModule) -> Result<Vec<GlobalVariable>, TranspilationError> {
        let mut globals = Vec::new();

        for (index, global) in wasm_module.globals.iter().enumerate() {
            globals.push(GlobalVariable {
                index: index as u32,
                var_type: VariableType::I32, // Default type for simplified structure
                is_mutable: global.mutable,
                initial_value: None, // TODO: Parse init expression
            });
        }

        Ok(globals)
    }

    /// Process memory layout
    fn process_memory_layout(&self, wasm_module: &WasmModule) -> Result<MemoryLayout, TranspilationError> {
        // WASM typically has one memory, use the first one or default
        if let Some(memory) = wasm_module.memories.first() {
            Ok(MemoryLayout {
                initial_pages: memory.min_pages,
                maximum_pages: memory.max_pages,
                page_size: 65536, // WASM page size is 64KB
            })
        } else {
            // Default memory layout
            Ok(MemoryLayout {
                initial_pages: 1,
                maximum_pages: None,
                page_size: 65536,
            })
        }
    }

    /// Process exports
    fn process_exports(&self, wasm_module: &WasmModule) -> Result<Vec<ExportInfo>, TranspilationError> {
        let mut exports = Vec::new();

        for export in &wasm_module.exports {
            let kind = match export.kind {
                crate::wasm::ast::WasmExportKind::Function => ExportKind::Function,
                crate::wasm::ast::WasmExportKind::Memory => ExportKind::Memory,
                crate::wasm::ast::WasmExportKind::Global => ExportKind::Global,
                crate::wasm::ast::WasmExportKind::Table => ExportKind::Table,
            };

            let index = export.index;

            exports.push(ExportInfo {
                name: export.name.clone(),
                kind,
                index,
            });
        }

        Ok(exports)
    }

    /// Process imports
    fn process_imports(&self, wasm_module: &WasmModule) -> Result<Vec<ImportInfo>, TranspilationError> {
        let mut imports = Vec::new();

        for import in &wasm_module.imports {
            let kind = match &import.kind {
                crate::wasm::ast::WasmImportKind::Function { type_index } => ImportKind::Function { type_index: *type_index },
                crate::wasm::ast::WasmImportKind::Memory(_) => ImportKind::Memory,
                crate::wasm::ast::WasmImportKind::Global { .. } => ImportKind::Global,
                crate::wasm::ast::WasmImportKind::Table(_) => ImportKind::Table,
            };

            imports.push(ImportInfo {
                name: import.name.clone(),
                module_name: import.module.clone(),
                kind,
            });
        }

        Ok(imports)
    }
}

/// Control flow analyzer for WASM functions
struct ControlFlowAnalyzer {
    // TODO: Implement control flow analysis
}

impl ControlFlowAnalyzer {
    fn new() -> Self {
        Self {}
    }

    fn analyze(&self, _instructions: &[WasmInstruction]) -> Result<ControlFlowInfo, TranspilationError> {
        // TODO: Implement proper control flow analysis
        Ok(ControlFlowInfo::new())
    }
}

/// Control flow analysis results
struct ControlFlowInfo {
    // TODO: Add control flow information
}

impl ControlFlowInfo {
    fn new() -> Self {
        Self {}
    }

    fn is_complex(&self) -> bool {
        // TODO: Implement complexity detection
        false
    }

    fn get_label_for_instruction(&self, _index: usize) -> Option<String> {
        // TODO: Generate labels for control flow targets
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpilation_config_default() {
        let config = TranspilationConfig::default();
        assert_eq!(config.target_architecture, VmArchitecture::Arch64);
        assert!(config.enable_optimizations);
        assert!(!config.preserve_debug_info);
        assert_eq!(config.max_function_size, Some(65536));
        assert!(config.enable_arch_features);
    }

    #[test]
    fn test_engine_creation() {
        let config = TranspilationConfig::default();
        let engine = TranspilationEngine::new(config);
        assert_eq!(engine.config.target_architecture, VmArchitecture::Arch64);
    }

    // Note: Variable type conversion test removed due to simplified structure

    #[test]
    fn test_architecture_compatibility() {
        let config = TranspilationConfig {
            target_architecture: VmArchitecture::Arch128,
            ..Default::default()
        };
        let engine = TranspilationEngine::new(config);

        assert!(engine.is_architecture_compatible(VmArchitecture::Arch64));
        assert!(engine.is_architecture_compatible(VmArchitecture::Arch128));
        assert!(!engine.is_architecture_compatible(VmArchitecture::Arch256));
    }
}
