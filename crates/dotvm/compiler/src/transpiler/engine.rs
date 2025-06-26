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
    opcode_mapper::{MappedInstruction, OpcodeMapper, OpcodeMappingError},
    parser::{WasmParseError, WasmParser},
};
use dotvm_core::bytecode::{BytecodeHeader, VmArchitecture};
use std::collections::HashMap;
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
    /// Function index in the original WASM module
    pub wasm_index: u32,
    /// DotVM bytecode instructions
    pub instructions: Vec<TranspiledInstruction>,
    /// Local variable count and types
    pub locals: Vec<LocalVariable>,
    /// Function metadata
    pub metadata: FunctionMetadata,
}

/// A single transpiled instruction with additional information
#[derive(Debug, Clone)]
pub struct TranspiledInstruction {
    /// The mapped DotVM instruction
    pub mapped: MappedInstruction,
    /// Original WASM instruction for debugging
    pub original_wasm: WasmInstruction,
    /// Bytecode offset (filled during code generation)
    pub offset: Option<u32>,
    /// Label information for control flow
    pub label: Option<String>,
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
#[derive(Debug)]
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

#[derive(Debug, Clone)]
pub enum ExportKind {
    Function,
    Memory,
    Global,
    Table,
}

/// Import information
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub module: String,
    pub name: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Function { type_index: u32 },
    Memory,
    Global,
    Table,
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
        let mut metadata = FunctionMetadata::default();

        // Analyze control flow
        let control_flow = self.control_flow_analyzer.analyze(&wasm_function.body)?;
        metadata.has_complex_control_flow = control_flow.is_complex();

        // Process each instruction
        for (inst_index, wasm_instruction) in wasm_function.body.iter().enumerate() {
            let mapped_instructions = self.mapper.map_instruction(wasm_instruction)?;

            for mapped in mapped_instructions {
                let transpiled = TranspiledInstruction {
                    mapped,
                    original_wasm: wasm_instruction.clone(),
                    offset: None, // Will be filled during code generation
                    label: control_flow.get_label_for_instruction(inst_index),
                };

                // Update metadata before moving transpiled
                self.update_function_metadata(&mut metadata, &transpiled);
                instructions.push(transpiled);
            }
        }

        // Process local variables
        let locals = self.process_local_variables(wasm_function)?;

        Ok(TranspiledFunction {
            wasm_index: index,
            instructions,
            locals,
            metadata,
        })
    }

    /// Update function metadata based on a transpiled instruction
    fn update_function_metadata(&self, metadata: &mut FunctionMetadata, instruction: &TranspiledInstruction) {
        // Update stack depth
        let (consumed, produced) = instruction.mapped.metadata.stack_effect;
        // This is a simplified calculation - a real implementation would track actual stack depth
        metadata.max_stack_depth = metadata.max_stack_depth.max(produced);

        // Track memory accesses
        if let Some(memory_access) = &instruction.mapped.metadata.memory_access {
            metadata.memory_accesses.push(MemoryAccessPattern {
                offset: memory_access.offset,
                size: memory_access.size,
                is_write: matches!(instruction.original_wasm, WasmInstruction::I32Store { .. } | WasmInstruction::I64Store { .. }),
                frequency: 1, // Would be calculated through profiling
            });
        }

        // Track function calls
        if let Some(control_flow) = &instruction.mapped.metadata.control_flow {
            if let crate::wasm::opcode_mapper::ControlFlowInfo::Call { function_index } = control_flow {
                metadata.function_calls.push(*function_index);
            }
        }
    }

    /// Process local variables from WASM function
    fn process_local_variables(&self, wasm_function: &WasmFunction) -> Result<Vec<LocalVariable>, TranspilationError> {
        let mut locals = Vec::new();
        let mut index = 0;

        // Add parameters
        for param_type in &wasm_function.signature.params {
            locals.push(LocalVariable {
                index,
                var_type: self.convert_wasm_type_to_variable_type(param_type)?,
                is_parameter: true,
            });
            index += 1;
        }

        // Add local variables
        for local_type in &wasm_function.locals {
            locals.push(LocalVariable {
                index,
                var_type: self.convert_wasm_type_to_variable_type(local_type)?,
                is_parameter: false,
            });
            index += 1;
        }

        Ok(locals)
    }

    /// Convert WASM value type to DotVM variable type
    fn convert_wasm_type_to_variable_type(&self, wasm_type: &crate::wasm::ast::WasmValueType) -> Result<VariableType, TranspilationError> {
        match wasm_type {
            crate::wasm::ast::WasmValueType::I32 => Ok(VariableType::I32),
            crate::wasm::ast::WasmValueType::I64 => Ok(VariableType::I64),
            crate::wasm::ast::WasmValueType::F32 => Ok(VariableType::F32),
            crate::wasm::ast::WasmValueType::F64 => Ok(VariableType::F64),
            crate::wasm::ast::WasmValueType::V128 => Ok(VariableType::V128),
            crate::wasm::ast::WasmValueType::FuncRef => Ok(VariableType::Pointer),
            crate::wasm::ast::WasmValueType::ExternRef => Ok(VariableType::Pointer),
        }
    }

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

    /// Process global variables
    fn process_globals(&self, wasm_module: &WasmModule) -> Result<Vec<GlobalVariable>, TranspilationError> {
        let mut globals = Vec::new();

        for (index, global) in wasm_module.globals.iter().enumerate() {
            globals.push(GlobalVariable {
                index: index as u32,
                var_type: self.convert_wasm_type_to_variable_type(&global.value_type)?,
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
                module: import.module.clone(),
                name: import.name.clone(),
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

    #[test]
    fn test_variable_type_conversion() {
        let config = TranspilationConfig::default();
        let engine = TranspilationEngine::new(config);

        assert!(matches!(engine.convert_wasm_type_to_variable_type(&crate::wasm::ast::WasmValueType::I32).unwrap(), VariableType::I32));
        assert!(matches!(engine.convert_wasm_type_to_variable_type(&crate::wasm::ast::WasmValueType::I64).unwrap(), VariableType::I64));
    }

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
