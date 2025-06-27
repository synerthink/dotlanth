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

//! DotVM bytecode generator
//!
//! This module generates the final DotVM bytecode from transpiled modules,
//! including serialization, optimization passes, and architecture-specific
//! bytecode generation.

use crate::transpiler::engine::{ExportInfo, ExportKind, ImportInfo, ImportKind, TranspiledFunction, TranspiledInstruction, TranspiledModule};
use dotvm_core::bytecode::{BytecodeHeader, VmArchitecture};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during bytecode generation
#[derive(Error, Debug)]
pub enum BytecodeGenerationError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid instruction at offset {offset}: {reason}")]
    InvalidInstruction { offset: u32, reason: String },
    #[error("Label resolution failed: {0}")]
    LabelResolutionFailed(String),
    #[error("Function index out of bounds: {0}")]
    FunctionIndexOutOfBounds(u32),
    #[error("Memory layout error: {0}")]
    MemoryLayoutError(String),
    #[error("Export resolution error: {0}")]
    ExportResolutionError(String),
    #[error("Import resolution error: {0}")]
    ImportResolutionError(String),
    #[error("Optimization error: {0}")]
    OptimizationError(String),
}

/// Configuration for bytecode generation
#[derive(Debug, Clone)]
pub struct BytecodeGenerationConfig {
    /// Whether to enable bytecode optimizations
    pub enable_optimizations: bool,
    /// Whether to include debug information
    pub include_debug_info: bool,
    /// Whether to compress the bytecode
    pub enable_compression: bool,
    /// Target architecture for optimization
    pub target_architecture: VmArchitecture,
    /// Maximum bytecode size (None for unlimited)
    pub max_bytecode_size: Option<usize>,
}

impl Default for BytecodeGenerationConfig {
    fn default() -> Self {
        Self {
            enable_optimizations: true,
            include_debug_info: false,
            enable_compression: false,
            target_architecture: VmArchitecture::Arch64,
            max_bytecode_size: None,
        }
    }
}

/// Generated DotVM bytecode with metadata
#[derive(Debug)]
pub struct GeneratedBytecode {
    /// The complete bytecode including header
    pub bytecode: Vec<u8>,
    /// Function table for runtime lookup
    pub function_table: FunctionTable,
    /// Export table for external access
    pub export_table: ExportTable,
    /// Import table for external dependencies
    pub import_table: ImportTable,
    /// Debug information (if enabled)
    pub debug_info: Option<DebugInfo>,
    /// Generation statistics
    pub stats: GenerationStats,
}

/// Function table for runtime function lookup
#[derive(Debug, Clone)]
pub struct FunctionTable {
    /// Function entries indexed by function ID
    pub entries: Vec<FunctionEntry>,
    /// Lookup map from WASM function index to DotVM function ID
    pub wasm_to_dotvm_map: HashMap<u32, u32>,
}

/// Single function entry in the function table
#[derive(Debug, Clone)]
pub struct FunctionEntry {
    /// Function ID in DotVM
    pub id: u32,
    /// Original WASM function index
    pub wasm_index: u32,
    /// Bytecode offset where function starts
    pub offset: u32,
    /// Function size in bytes
    pub size: u32,
    /// Number of local variables
    pub local_count: u32,
    /// Maximum stack depth required
    pub max_stack_depth: u32,
    /// Function flags
    pub flags: FunctionFlags,
}

/// Function flags for runtime optimization
#[derive(Debug, Clone, Default)]
pub struct FunctionFlags {
    /// Function is recursive
    pub is_recursive: bool,
    /// Function has complex control flow
    pub has_complex_control_flow: bool,
    /// Function accesses memory
    pub accesses_memory: bool,
    /// Function makes external calls
    pub makes_external_calls: bool,
    /// Function is a leaf function (makes no calls)
    pub is_leaf: bool,
}

/// Export table for external function/memory access
#[derive(Debug, Clone)]
pub struct ExportTable {
    /// Export entries
    pub entries: Vec<ExportEntry>,
    /// Name to index lookup
    pub name_lookup: HashMap<String, u32>,
}

/// Single export entry
#[derive(Debug, Clone)]
pub struct ExportEntry {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
    /// Internal index (function ID, memory ID, etc.)
    pub internal_index: u32,
    /// Bytecode offset (for functions)
    pub offset: Option<u32>,
}

/// Import table for external dependencies
#[derive(Debug, Clone)]
pub struct ImportTable {
    /// Import entries
    pub entries: Vec<ImportEntry>,
    /// Module and name to index lookup
    pub lookup: HashMap<(String, String), u32>,
}

/// Single import entry
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// Module name
    pub module: String,
    /// Import name
    pub name: String,
    /// Import kind
    pub kind: ImportKind,
    /// Internal index assigned
    pub internal_index: u32,
}

/// Debug information for debugging and profiling
#[derive(Debug, Clone)]
pub struct DebugInfo {
    /// Source map from bytecode offset to WASM instruction
    pub source_map: HashMap<u32, SourceLocation>,
    /// Function names
    pub function_names: HashMap<u32, String>,
    /// Local variable names
    pub local_names: HashMap<u32, HashMap<u32, String>>,
}

/// Source location information
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// WASM function index
    pub wasm_function: u32,
    /// WASM instruction index within function
    pub wasm_instruction: u32,
    /// Original WASM instruction for reference
    pub original_instruction: String,
}

/// Statistics about the bytecode generation process
#[derive(Debug, Clone, Default)]
pub struct GenerationStats {
    /// Total bytecode size
    pub total_size: usize,
    /// Header size
    pub header_size: usize,
    /// Code section size
    pub code_size: usize,
    /// Data section size
    pub data_size: usize,
    /// Number of functions
    pub function_count: u32,
    /// Number of instructions
    pub instruction_count: u32,
    /// Number of optimizations applied
    pub optimizations_applied: u32,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
}

/// DotVM bytecode generator
pub struct BytecodeGenerator {
    /// Generation configuration
    config: BytecodeGenerationConfig,
    /// Current bytecode buffer
    bytecode: Vec<u8>,
    /// Current offset in bytecode
    current_offset: u32,
    /// Label resolution table
    label_table: HashMap<String, u32>,
    /// Pending label references
    pending_labels: Vec<LabelReference>,
}

/// Pending label reference for resolution
#[derive(Debug)]
struct LabelReference {
    /// Label name
    label: String,
    /// Bytecode offset where the reference is located
    reference_offset: u32,
    /// Size of the reference (2 or 4 bytes)
    reference_size: u32,
}

impl BytecodeGenerator {
    /// Create a new bytecode generator with the given configuration
    pub fn new(config: BytecodeGenerationConfig) -> Self {
        Self {
            config,
            bytecode: Vec::new(),
            current_offset: 0,
            label_table: HashMap::new(),
            pending_labels: Vec::new(),
        }
    }

    /// Generate DotVM bytecode from a transpiled module
    pub fn generate(&mut self, module: &TranspiledModule) -> Result<GeneratedBytecode, BytecodeGenerationError> {
        let start_time = std::time::Instant::now();
        let mut stats = GenerationStats::default();

        // Clear previous state
        self.bytecode.clear();
        self.current_offset = 0;
        self.label_table.clear();
        self.pending_labels.clear();

        // Generate header
        self.generate_header(&module.header)?;
        stats.header_size = self.current_offset as usize;

        // Generate function table
        let function_table = self.generate_function_table(&module.functions)?;
        stats.function_count = function_table.entries.len() as u32;

        // Generate code section
        let code_start_offset = self.current_offset;
        self.generate_code_section(&module.functions)?;
        stats.code_size = (self.current_offset - code_start_offset) as usize;

        // Generate data section
        let data_start_offset = self.current_offset;
        self.generate_data_section(module)?;
        stats.data_size = (self.current_offset - data_start_offset) as usize;

        // Resolve labels
        self.resolve_labels()?;

        // Apply optimizations if enabled
        if self.config.enable_optimizations {
            stats.optimizations_applied = self.apply_optimizations()?;
        }

        // Generate export and import tables
        let export_table = self.generate_export_table(&module.exports, &function_table)?;
        let import_table = self.generate_import_table(&module.imports)?;

        // Generate debug info if enabled
        let debug_info = if self.config.include_debug_info {
            Some(self.generate_debug_info(&module.functions)?)
        } else {
            None
        };

        // Finalize statistics
        stats.total_size = self.bytecode.len();
        stats.generation_time_ms = start_time.elapsed().as_millis() as u64;

        // Count total instructions
        stats.instruction_count = module.functions.iter().map(|f| f.instructions.len() as u32).sum();

        Ok(GeneratedBytecode {
            bytecode: self.bytecode.clone(),
            function_table,
            export_table,
            import_table,
            debug_info,
            stats,
        })
    }

    /// Generate the bytecode header
    fn generate_header(&mut self, header: &BytecodeHeader) -> Result<(), BytecodeGenerationError> {
        let header_bytes = header.to_bytes();
        self.bytecode.extend_from_slice(&header_bytes);
        self.current_offset += header_bytes.len() as u32;
        Ok(())
    }

    /// Generate the function table
    fn generate_function_table(&mut self, functions: &[TranspiledFunction]) -> Result<FunctionTable, BytecodeGenerationError> {
        let mut entries = Vec::new();
        let mut wasm_to_dotvm_map = HashMap::new();

        // Write function table header
        self.write_u32(functions.len() as u32)?;

        for (dotvm_id, function) in functions.iter().enumerate() {
            let entry = FunctionEntry {
                id: dotvm_id as u32,
                wasm_index: dotvm_id as u32, // Use dotvm_id as wasm_index for simplicity
                offset: 0,                   // Will be filled during code generation
                size: 0,                     // Will be calculated during code generation
                local_count: function.local_count as u32,
                max_stack_depth: 32, // Default stack depth
                flags: FunctionFlags {
                    is_recursive: false, // Default values for simplified structure
                    has_complex_control_flow: false,
                    accesses_memory: false,
                    makes_external_calls: false,
                    is_leaf: true,
                },
            };

            // Write function table entry
            self.write_u32(entry.wasm_index)?;
            self.write_u32(entry.local_count)?;
            self.write_u32(entry.max_stack_depth)?;
            self.write_function_flags(&entry.flags)?;

            wasm_to_dotvm_map.insert(dotvm_id as u32, dotvm_id as u32);
            entries.push(entry);
        }

        Ok(FunctionTable { entries, wasm_to_dotvm_map })
    }

    /// Generate the code section with all function bytecode
    fn generate_code_section(&mut self, functions: &[TranspiledFunction]) -> Result<(), BytecodeGenerationError> {
        // Write code section header
        self.write_u32(functions.len() as u32)?;

        for function in functions {
            let function_start_offset = self.current_offset;

            // Generate function prologue
            self.generate_function_prologue(function)?;

            // Generate function body
            for instruction in &function.instructions {
                self.generate_instruction(instruction)?;
            }

            // Generate function epilogue
            self.generate_function_epilogue(function)?;

            let function_size = self.current_offset - function_start_offset;
            // TODO: Update function table with actual offset and size
        }

        Ok(())
    }

    /// Generate function prologue (local variable setup, etc.)
    fn generate_function_prologue(&mut self, function: &TranspiledFunction) -> Result<(), BytecodeGenerationError> {
        // Reserve space for local variables
        if function.local_count > 0 {
            // Generate instruction to allocate local variable space
            self.write_u16(0x0200)?; // Memory opcode base
            self.write_u16(0x10)?; // AllocateLocals sub-opcode
            self.write_u32(function.local_count as u32)?;
        }

        Ok(())
    }

    /// Generate bytecode for a single instruction
    fn generate_instruction(&mut self, instruction: &TranspiledInstruction) -> Result<(), BytecodeGenerationError> {
        // Write opcode (simplified - just write a placeholder)
        let opcode_hash = self.hash_opcode(&instruction.opcode);
        self.write_u16(opcode_hash)?;

        // Write operands
        for operand in &instruction.operands {
            // Parse operand string to u64 (simplified)
            let operand_value = operand.parse::<u64>().unwrap_or(0);
            self.write_u64(operand_value)?;
        }

        Ok(())
    }

    /// Hash an opcode string to a u16 value (simplified)
    fn hash_opcode(&self, opcode: &str) -> u16 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        opcode.hash(&mut hasher);
        (hasher.finish() as u16) % 0x1000 // Keep it in a reasonable range
    }

    /// Generate function epilogue
    fn generate_function_epilogue(&mut self, _function: &TranspiledFunction) -> Result<(), BytecodeGenerationError> {
        // Generate return instruction if not already present
        // This is a safety measure for functions that don't end with explicit return
        // TODO: Check if last instruction is already a return
        Ok(())
    }

    /// Generate the data section
    fn generate_data_section(&mut self, module: &TranspiledModule) -> Result<(), BytecodeGenerationError> {
        // Write memory layout information
        self.write_u32(module.memory_layout.initial_pages)?;
        self.write_u32(module.memory_layout.maximum_pages.unwrap_or(0))?;
        self.write_u32(module.memory_layout.page_size)?;

        // Write global variables
        self.write_u32(module.globals.len() as u32)?;
        for global in &module.globals {
            self.write_u32(global.index)?;
            self.write_u8(global.var_type as u8)?;
            self.write_u8(if global.is_mutable { 1 } else { 0 })?;
            self.write_u64(global.initial_value.unwrap_or(0))?;
        }

        Ok(())
    }

    /// Resolve all pending label references
    fn resolve_labels(&mut self) -> Result<(), BytecodeGenerationError> {
        for label_ref in &self.pending_labels {
            let target_offset = self
                .label_table
                .get(&label_ref.label)
                .ok_or_else(|| BytecodeGenerationError::LabelResolutionFailed(label_ref.label.clone()))?;

            // Calculate relative offset
            let relative_offset = target_offset.wrapping_sub(label_ref.reference_offset);

            // Write the resolved offset back to the bytecode
            match label_ref.reference_size {
                2 => {
                    let bytes = (relative_offset as u16).to_le_bytes();
                    self.bytecode[label_ref.reference_offset as usize] = bytes[0];
                    self.bytecode[label_ref.reference_offset as usize + 1] = bytes[1];
                }
                4 => {
                    let bytes = relative_offset.to_le_bytes();
                    for (i, byte) in bytes.iter().enumerate() {
                        self.bytecode[label_ref.reference_offset as usize + i] = *byte;
                    }
                }
                _ => {
                    return Err(BytecodeGenerationError::InvalidInstruction {
                        offset: label_ref.reference_offset,
                        reason: format!("Invalid label reference size: {}", label_ref.reference_size),
                    });
                }
            }
        }

        Ok(())
    }

    /// Apply bytecode optimizations
    fn apply_optimizations(&mut self) -> Result<u32, BytecodeGenerationError> {
        let mut optimizations_applied = 0;

        // TODO: Implement various optimization passes:
        // - Dead code elimination
        // - Constant folding
        // - Peephole optimizations
        // - Jump optimization
        // - Register allocation optimization

        Ok(optimizations_applied)
    }

    /// Generate export table
    fn generate_export_table(&mut self, exports: &[ExportInfo], function_table: &FunctionTable) -> Result<ExportTable, BytecodeGenerationError> {
        let mut entries = Vec::new();
        let mut name_lookup = HashMap::new();

        for (index, export) in exports.iter().enumerate() {
            let internal_index = match export.kind {
                ExportKind::Function => function_table
                    .wasm_to_dotvm_map
                    .get(&export.index)
                    .copied()
                    .ok_or_else(|| BytecodeGenerationError::ExportResolutionError(format!("Function {} not found in function table", export.index)))?,
                _ => export.index, // For memory, globals, tables, use direct index
            };

            let offset = match export.kind {
                ExportKind::Function => function_table.entries.get(internal_index as usize).map(|entry| entry.offset),
                _ => None,
            };

            let entry = ExportEntry {
                name: export.name.clone(),
                kind: export.kind.clone(),
                internal_index,
                offset,
            };

            name_lookup.insert(export.name.clone(), index as u32);
            entries.push(entry);
        }

        Ok(ExportTable { entries, name_lookup })
    }

    /// Generate import table
    fn generate_import_table(&mut self, imports: &[ImportInfo]) -> Result<ImportTable, BytecodeGenerationError> {
        let mut entries = Vec::new();
        let mut lookup = HashMap::new();

        for (index, import) in imports.iter().enumerate() {
            let entry = ImportEntry {
                module: import.module.clone(),
                name: import.name.clone(),
                kind: import.kind.clone(),
                internal_index: index as u32, // Assign sequential internal indices
            };

            lookup.insert((import.module.clone(), import.name.clone()), index as u32);
            entries.push(entry);
        }

        Ok(ImportTable { entries, lookup })
    }

    /// Generate debug information
    fn generate_debug_info(&self, functions: &[TranspiledFunction]) -> Result<DebugInfo, BytecodeGenerationError> {
        let mut source_map = HashMap::new();
        let mut function_names = HashMap::new();
        let mut local_names = HashMap::new();

        // TODO: Implement debug info generation
        // This would include mapping bytecode offsets to original WASM instructions,
        // function names, local variable names, etc.

        Ok(DebugInfo {
            source_map,
            function_names,
            local_names,
        })
    }

    /// Write function flags to bytecode
    fn write_function_flags(&mut self, flags: &FunctionFlags) -> Result<(), BytecodeGenerationError> {
        let mut flag_byte = 0u8;
        if flags.is_recursive {
            flag_byte |= 0x01;
        }
        if flags.has_complex_control_flow {
            flag_byte |= 0x02;
        }
        if flags.accesses_memory {
            flag_byte |= 0x04;
        }
        if flags.makes_external_calls {
            flag_byte |= 0x08;
        }
        if flags.is_leaf {
            flag_byte |= 0x10;
        }
        self.write_u8(flag_byte)
    }

    /// Write a u8 to the bytecode
    fn write_u8(&mut self, value: u8) -> Result<(), BytecodeGenerationError> {
        self.bytecode.push(value);
        self.current_offset += 1;
        Ok(())
    }

    /// Write a u16 to the bytecode (little-endian)
    fn write_u16(&mut self, value: u16) -> Result<(), BytecodeGenerationError> {
        self.bytecode.extend_from_slice(&value.to_le_bytes());
        self.current_offset += 2;
        Ok(())
    }

    /// Write a u32 to the bytecode (little-endian)
    fn write_u32(&mut self, value: u32) -> Result<(), BytecodeGenerationError> {
        self.bytecode.extend_from_slice(&value.to_le_bytes());
        self.current_offset += 4;
        Ok(())
    }

    /// Write a u64 to the bytecode (little-endian)
    fn write_u64(&mut self, value: u64) -> Result<(), BytecodeGenerationError> {
        self.bytecode.extend_from_slice(&value.to_le_bytes());
        self.current_offset += 8;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::engine::{FunctionMetadata, LocalVariable, MemoryLayout, TranspiledFunction, TranspiledInstruction, TranspiledModule, VariableType};
    use crate::wasm::{
        ast::WasmInstruction,
        opcode_mapper::{InstructionMetadata, MappedInstruction, MappedOpcode},
    };
    use dotvm_core::opcode::{architecture_opcodes::Opcode64, arithmetic_opcodes::ArithmeticOpcode};

    #[test]
    fn test_generator_creation() {
        let config = BytecodeGenerationConfig::default();
        let generator = BytecodeGenerator::new(config);
        assert_eq!(generator.bytecode.len(), 0);
        assert_eq!(generator.current_offset, 0);
    }

    #[test]
    fn test_header_generation() {
        let config = BytecodeGenerationConfig::default();
        let mut generator = BytecodeGenerator::new(config);
        let header = BytecodeHeader::new(VmArchitecture::Arch64);

        generator.generate_header(&header).unwrap();
        assert_eq!(generator.bytecode.len(), BytecodeHeader::size());
        assert_eq!(generator.current_offset, BytecodeHeader::size() as u32);
    }

    #[test]
    fn test_write_operations() {
        let config = BytecodeGenerationConfig::default();
        let mut generator = BytecodeGenerator::new(config);

        generator.write_u8(0x42).unwrap();
        generator.write_u16(0x1234).unwrap();
        generator.write_u32(0x12345678).unwrap();
        generator.write_u64(0x123456789ABCDEF0).unwrap();

        assert_eq!(generator.current_offset, 1 + 2 + 4 + 8);
        assert_eq!(generator.bytecode[0], 0x42);
        assert_eq!(generator.bytecode[1..3], [0x34, 0x12]); // Little-endian u16
        assert_eq!(generator.bytecode[3..7], [0x78, 0x56, 0x34, 0x12]); // Little-endian u32
    }

    // TODO: Add more comprehensive tests with actual transpiled modules
}

/// Type alias for backward compatibility
pub type DotVMGenerator = BytecodeGenerator;

impl DotVMGenerator {
    /// Create a new DotVM generator with default configuration for the given architecture
    pub fn with_architecture(target_arch: VmArchitecture) -> Self {
        let config = BytecodeGenerationConfig {
            target_architecture: target_arch,
            enable_optimizations: true,
            include_debug_info: false,
            enable_compression: false,
            max_bytecode_size: None,
        };
        Self::new(config)
    }

    /// Generate bytecode from a transpiled module (convenience method for testing)
    pub fn generate_bytecode(&mut self, module: TranspiledModule) -> Result<Vec<u8>, BytecodeGenerationError> {
        let generated = self.generate(&module)?;
        Ok(generated.bytecode)
    }
}
