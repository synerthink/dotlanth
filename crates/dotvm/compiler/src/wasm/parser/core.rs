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

//! Core WASM parser implementation

use super::super::{
    ast::*,
    error::{WasmError, WasmResult},
};
use super::{ParserConfig, ParserContext};
use wasmparser::{Parser, Payload, WasmFeatures};

/// Main WebAssembly parser
pub struct WasmParser {
    /// Parser configuration
    config: ParserConfig,
    /// Parser context
    context: ParserContext,
}

impl WasmParser {
    /// Create a new WASM parser with default configuration
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
            context: ParserContext::new(),
        }
    }

    /// Create a new WASM parser with specific configuration
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            context: ParserContext::new(),
        }
    }

    /// Parse a WASM binary into our internal AST representation
    pub fn parse(&mut self, wasm_bytes: &[u8]) -> WasmResult<WasmModule> {
        self.context.reset();
        self.context.start_parsing();
        self.context.metrics.bytes_parsed = wasm_bytes.len();

        // Validate configuration
        self.config.validate()?;

        // Create wasmparser with our features
        let parser = Parser::new(0);
        let mut module = WasmModule::new();

        // Temporary storage for building the module
        let mut type_section: Vec<WasmFunctionType> = Vec::new();
        let mut function_section: Vec<u32> = Vec::new();
        let mut code_section: Vec<WasmFunction> = Vec::new();

        // Parse all sections
        for payload in parser.parse_all(wasm_bytes) {
            let payload = payload.map_err(WasmError::ParserError)?;
            self.parse_payload(payload, &mut module, &mut type_section, &mut function_section, &mut code_section)?;
        }

        // Combine function types with code to create complete functions
        self.finalize_functions(&mut module, type_section, function_section, code_section)?;

        // Validate the final module if requested
        if self.config.validate_structure {
            self.validate_module(&module)?;
        }

        self.context.finish_parsing();
        Ok(module)
    }

    /// Parse a single payload
    fn parse_payload(
        &mut self,
        payload: Payload,
        module: &mut WasmModule,
        type_section: &mut Vec<WasmFunctionType>,
        function_section: &mut Vec<u32>,
        code_section: &mut Vec<WasmFunction>,
    ) -> WasmResult<()> {
        match payload {
            Payload::Version { num, .. } => {
                if num != 1 {
                    return Err(WasmError::UnsupportedVersion { version: num as u32 });
                }
            }

            Payload::TypeSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_type_section(reader, type_section)?;
                self.context.record_section_time(WasmSectionType::Type, section_start.elapsed());
            }

            Payload::ImportSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_import_section(reader, module)?;
                self.context.record_section_time(WasmSectionType::Import, section_start.elapsed());
            }

            Payload::FunctionSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_function_section(reader, function_section)?;
                self.context.record_section_time(WasmSectionType::Function, section_start.elapsed());
            }

            Payload::TableSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_table_section(reader, module)?;
                self.context.record_section_time(WasmSectionType::Table, section_start.elapsed());
            }

            Payload::MemorySection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_memory_section(reader, module)?;
                self.context.record_section_time(WasmSectionType::Memory, section_start.elapsed());
            }

            Payload::GlobalSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_global_section(reader, module)?;
                self.context.record_section_time(WasmSectionType::Global, section_start.elapsed());
            }

            Payload::ExportSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_export_section(reader, module)?;
                self.context.record_section_time(WasmSectionType::Export, section_start.elapsed());
            }

            Payload::StartSection { func, .. } => {
                module.start_function = Some(func);
            }

            Payload::ElementSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_element_section(reader, module)?;
                self.context.record_section_time(WasmSectionType::Element, section_start.elapsed());
            }

            Payload::CodeSectionEntry(body) => {
                let section_start = std::time::Instant::now();
                let function = self.parse_function_body(&body, function_section, type_section)?;
                code_section.push(function);
                self.context.record_section_time(WasmSectionType::Code, section_start.elapsed());
            }

            Payload::DataSection(reader) => {
                let section_start = std::time::Instant::now();
                self.parse_data_section(reader, module)?;
                self.context.record_section_time(WasmSectionType::Data, section_start.elapsed());
            }

            Payload::CustomSection(reader) => {
                if self.config.preserve_custom_sections {
                    let section_start = std::time::Instant::now();
                    self.parse_custom_section(reader, module)?;
                    self.context.record_section_time(WasmSectionType::Custom, section_start.elapsed());
                }
            }

            _ => {
                // Handle other payloads or ignore them
                self.context.add_warning(format!("Unhandled payload type: {:?}", payload));
            }
        }

        Ok(())
    }

    /// Parse type section
    fn parse_type_section(&mut self, reader: wasmparser::TypeSectionReader, type_section: &mut Vec<WasmFunctionType>) -> WasmResult<()> {
        for ty in reader {
            let rec_group = ty.map_err(WasmError::ParserError)?;
            for sub_type in rec_group.types() {
                if let wasmparser::SubType {
                    composite_type: wasmparser::CompositeType::Func(func_type),
                    ..
                } = &sub_type
                {
                    type_section.push(self.convert_function_type(func_type)?);
                }
            }
        }

        // Validate section limits
        self.config.limits.validate_count(WasmSectionType::Type, type_section.len())?;

        Ok(())
    }

    /// Parse import section
    fn parse_import_section(&mut self, reader: wasmparser::ImportSectionReader, module: &mut WasmModule) -> WasmResult<()> {
        for import in reader {
            let import = import.map_err(WasmError::ParserError)?;
            module.imports.push(self.convert_import(&import)?);
        }

        self.config.limits.validate_count(WasmSectionType::Import, module.imports.len())?;
        Ok(())
    }

    /// Parse function section
    fn parse_function_section(&mut self, reader: wasmparser::FunctionSectionReader, function_section: &mut Vec<u32>) -> WasmResult<()> {
        for func in reader {
            let type_index = func.map_err(WasmError::ParserError)?;
            function_section.push(type_index);
        }

        self.config.limits.validate_count(WasmSectionType::Function, function_section.len())?;
        Ok(())
    }

    /// Parse table section
    fn parse_table_section(&mut self, reader: wasmparser::TableSectionReader, module: &mut WasmModule) -> WasmResult<()> {
        for table in reader {
            let table = table.map_err(WasmError::ParserError)?;
            let element_type = match table.ty.element_type {
                wasmparser::RefType::FUNCREF => WasmValueType::FuncRef,
                wasmparser::RefType::EXTERNREF => WasmValueType::ExternRef,
                _ => WasmValueType::FuncRef, // Default fallback
            };

            module.tables.push(WasmTable::new(WasmTableType::new(element_type, table.ty.initial, table.ty.maximum)));
        }

        self.config.limits.validate_count(WasmSectionType::Table, module.tables.len())?;
        Ok(())
    }

    /// Parse memory section
    fn parse_memory_section(&mut self, reader: wasmparser::MemorySectionReader, module: &mut WasmModule) -> WasmResult<()> {
        for memory in reader {
            let memory = memory.map_err(WasmError::ParserError)?;
            module
                .memories
                .push(WasmMemory::new(WasmMemoryType::new(memory.initial as u32, memory.maximum.map(|m| m as u32), memory.shared)));
        }

        self.config.limits.validate_count(WasmSectionType::Memory, module.memories.len())?;
        Ok(())
    }

    /// Parse global section
    fn parse_global_section(&mut self, reader: wasmparser::GlobalSectionReader, module: &mut WasmModule) -> WasmResult<()> {
        for global in reader {
            let global = global.map_err(WasmError::ParserError)?;
            let global_type = WasmGlobalType::new(self.convert_value_type(&global.ty.content_type)?, global.ty.mutable);

            // Parse init expression
            let init_expr = self.parse_init_expression(global.init_expr)?;

            module.globals.push(WasmGlobal::new(global_type, init_expr));
        }

        self.config.limits.validate_count(WasmSectionType::Global, module.globals.len())?;
        Ok(())
    }

    /// Parse export section
    fn parse_export_section(&mut self, reader: wasmparser::ExportSectionReader, module: &mut WasmModule) -> WasmResult<()> {
        for export in reader {
            let export = export.map_err(WasmError::ParserError)?;
            module.exports.push(self.convert_export(&export)?);
        }

        self.config.limits.validate_count(WasmSectionType::Export, module.exports.len())?;
        Ok(())
    }

    /// Parse element section
    fn parse_element_section(&mut self, reader: wasmparser::ElementSectionReader, module: &mut WasmModule) -> WasmResult<()> {
        for element in reader {
            let _element = element.map_err(WasmError::ParserError)?;
            // TODO: Implement element section parsing
            self.context.add_warning("Element section parsing not yet implemented".to_string());
        }

        Ok(())
    }

    /// Parse data section
    fn parse_data_section(&mut self, reader: wasmparser::DataSectionReader, module: &mut WasmModule) -> WasmResult<()> {
        for data in reader {
            let data = data.map_err(WasmError::ParserError)?;
            let memory_index = match &data.kind {
                wasmparser::DataKind::Active { memory_index, .. } => *memory_index,
                wasmparser::DataKind::Passive => 0,
            };

            // Parse offset expression for active data
            let offset = match &data.kind {
                wasmparser::DataKind::Active { offset_expr, .. } => self.parse_init_expression(*offset_expr)?,
                wasmparser::DataKind::Passive => Vec::new(),
            };

            module.data_segments.push(WasmDataSegment::new(memory_index, offset, data.data.to_vec()));
        }

        self.config.limits.validate_count(WasmSectionType::Data, module.data_segments.len())?;
        Ok(())
    }

    /// Parse custom section
    fn parse_custom_section(&mut self, reader: wasmparser::CustomSectionReader, module: &mut WasmModule) -> WasmResult<()> {
        let name = reader.name().to_string();
        let data = reader.data().to_vec();

        module.custom_sections.push(WasmCustomSection::new(name, data));
        Ok(())
    }

    /// Parse function body
    fn parse_function_body(&mut self, body: &wasmparser::FunctionBody, function_section: &[u32], type_section: &[WasmFunctionType]) -> WasmResult<WasmFunction> {
        let function_index = function_section.len().saturating_sub(1);
        let type_index = function_section.get(function_index).ok_or_else(|| WasmError::InvalidFunctionIndex { index: function_index as u32 })?;

        let func_type = type_section.get(*type_index as usize).ok_or_else(|| WasmError::InvalidTypeIndex { index: *type_index })?.clone();

        let mut locals = Vec::new();
        let mut instructions = Vec::new();

        // Parse local variables
        let locals_reader = body.get_locals_reader().map_err(WasmError::ParserError)?;
        for local in locals_reader {
            let (count, val_type) = local.map_err(WasmError::ParserError)?;
            let wasm_type = self.convert_value_type(&val_type)?;
            for _ in 0..count {
                locals.push(wasm_type);
            }
        }

        // Parse instructions
        let operators_reader = body.get_operators_reader().map_err(WasmError::ParserError)?;
        for op in operators_reader {
            let op = op.map_err(WasmError::ParserError)?;
            instructions.push(self.convert_operator(&op)?);
        }

        Ok(WasmFunction::new(func_type, locals, instructions))
    }

    /// Parse initialization expression
    fn parse_init_expression(&self, expr: wasmparser::ConstExpr) -> WasmResult<Vec<WasmInstruction>> {
        let mut instructions = Vec::new();
        let reader = expr.get_operators_reader();

        for op in reader {
            let op = op.map_err(WasmError::ParserError)?;
            instructions.push(self.convert_operator(&op)?);
        }

        Ok(instructions)
    }

    /// Convert wasmparser function type to our type
    fn convert_function_type(&self, func_type: &wasmparser::FuncType) -> WasmResult<WasmFunctionType> {
        let params = func_type.params().iter().map(|t| self.convert_value_type(t)).collect::<Result<Vec<_>, _>>()?;

        let results = func_type.results().iter().map(|t| self.convert_value_type(t)).collect::<Result<Vec<_>, _>>()?;

        Ok(WasmFunctionType::new(params, results))
    }

    /// Convert wasmparser value type to our type
    fn convert_value_type(&self, val_type: &wasmparser::ValType) -> WasmResult<WasmValueType> {
        match val_type {
            wasmparser::ValType::I32 => Ok(WasmValueType::I32),
            wasmparser::ValType::I64 => Ok(WasmValueType::I64),
            wasmparser::ValType::F32 => Ok(WasmValueType::F32),
            wasmparser::ValType::F64 => Ok(WasmValueType::F64),
            wasmparser::ValType::V128 => {
                if !self.config.allow_simd {
                    return Err(WasmError::unsupported_feature("SIMD (V128)"));
                }
                Ok(WasmValueType::V128)
            }
            wasmparser::ValType::Ref(ref_type) => {
                if !self.config.allow_reference_types {
                    return Err(WasmError::unsupported_feature("Reference types"));
                }
                match ref_type.heap_type() {
                    wasmparser::HeapType::Func => Ok(WasmValueType::FuncRef),
                    wasmparser::HeapType::Extern => Ok(WasmValueType::ExternRef),
                    _ => Err(WasmError::unsupported_feature(format!("Reference type: {:?}", ref_type))),
                }
            }
        }
    }

    /// Convert wasmparser import to our type
    fn convert_import(&self, import: &wasmparser::Import) -> WasmResult<WasmImport> {
        let kind = match import.ty {
            wasmparser::TypeRef::Func(type_index) => WasmImportKind::Function { type_index },
            wasmparser::TypeRef::Table(table_type) => {
                let element_type = match table_type.element_type {
                    wasmparser::RefType::FUNCREF => WasmValueType::FuncRef,
                    wasmparser::RefType::EXTERNREF => WasmValueType::ExternRef,
                    _ => WasmValueType::FuncRef,
                };
                WasmImportKind::Table(WasmTable::new(WasmTableType::new(element_type, table_type.initial, table_type.maximum)))
            }
            wasmparser::TypeRef::Memory(memory_type) => WasmImportKind::Memory(WasmMemory::new(WasmMemoryType::new(
                memory_type.initial as u32,
                memory_type.maximum.map(|m| m as u32),
                memory_type.shared,
            ))),
            wasmparser::TypeRef::Global(global_type) => WasmImportKind::Global {
                value_type: self.convert_value_type(&global_type.content_type)?,
                mutable: global_type.mutable,
            },
            wasmparser::TypeRef::Tag(_) => {
                return Err(WasmError::unsupported_feature("Tag imports"));
            }
        };

        Ok(WasmImport::new(import.module.to_string(), import.name.to_string(), kind))
    }

    /// Convert wasmparser export to our type
    fn convert_export(&self, export: &wasmparser::Export) -> WasmResult<WasmExport> {
        let kind = match export.kind {
            wasmparser::ExternalKind::Func => WasmExportKind::Function,
            wasmparser::ExternalKind::Table => WasmExportKind::Table,
            wasmparser::ExternalKind::Memory => WasmExportKind::Memory,
            wasmparser::ExternalKind::Global => WasmExportKind::Global,
            wasmparser::ExternalKind::Tag => {
                return Err(WasmError::unsupported_feature("Tag exports"));
            }
        };

        Ok(WasmExport::new(export.name.to_string(), kind, export.index))
    }

    /// Convert wasmparser operator to our instruction
    fn convert_operator(&self, op: &wasmparser::Operator) -> WasmResult<WasmInstruction> {
        match op {
            // Control flow
            wasmparser::Operator::Unreachable => Ok(WasmInstruction::Unreachable),
            wasmparser::Operator::Nop => Ok(WasmInstruction::Nop),
            wasmparser::Operator::Block { blockty } => Ok(WasmInstruction::Block {
                block_type: self.convert_block_type(blockty)?,
            }),
            wasmparser::Operator::Loop { blockty } => Ok(WasmInstruction::Loop {
                block_type: self.convert_block_type(blockty)?,
            }),
            wasmparser::Operator::If { blockty } => Ok(WasmInstruction::If {
                block_type: self.convert_block_type(blockty)?,
            }),
            wasmparser::Operator::Else => Ok(WasmInstruction::Else),
            wasmparser::Operator::End => Ok(WasmInstruction::End),
            wasmparser::Operator::Br { relative_depth } => Ok(WasmInstruction::Br { label_index: *relative_depth }),
            wasmparser::Operator::BrIf { relative_depth } => Ok(WasmInstruction::BrIf { label_index: *relative_depth }),
            wasmparser::Operator::Return => Ok(WasmInstruction::Return),
            wasmparser::Operator::Call { function_index } => Ok(WasmInstruction::Call { function_index: *function_index }),

            // Stack operations
            wasmparser::Operator::Drop => Ok(WasmInstruction::Drop),

            // Constants
            wasmparser::Operator::I32Const { value } => Ok(WasmInstruction::I32Const { value: *value }),
            wasmparser::Operator::I64Const { value } => Ok(WasmInstruction::I64Const { value: *value }),
            wasmparser::Operator::F32Const { value } => Ok(WasmInstruction::F32Const { value: f32::from_bits(value.bits()) }),
            wasmparser::Operator::F64Const { value } => Ok(WasmInstruction::F64Const { value: f64::from_bits(value.bits()) }),

            // Arithmetic
            wasmparser::Operator::I32Add => Ok(WasmInstruction::I32Add),
            wasmparser::Operator::I32Sub => Ok(WasmInstruction::I32Sub),
            wasmparser::Operator::I32Mul => Ok(WasmInstruction::I32Mul),
            wasmparser::Operator::I64Add => Ok(WasmInstruction::I64Add),
            wasmparser::Operator::I64Sub => Ok(WasmInstruction::I64Sub),
            wasmparser::Operator::I64Mul => Ok(WasmInstruction::I64Mul),

            // Memory
            wasmparser::Operator::I32Load { memarg } => Ok(WasmInstruction::I32Load {
                memarg: MemArg::new(memarg.offset, memarg.align as u32),
            }),
            wasmparser::Operator::I64Load { memarg } => Ok(WasmInstruction::I64Load {
                memarg: MemArg::new(memarg.offset, memarg.align as u32),
            }),
            wasmparser::Operator::I32Store { memarg } => Ok(WasmInstruction::I32Store {
                memarg: MemArg::new(memarg.offset, memarg.align as u32),
            }),
            wasmparser::Operator::I64Store { memarg } => Ok(WasmInstruction::I64Store {
                memarg: MemArg::new(memarg.offset, memarg.align as u32),
            }),

            // Variables
            wasmparser::Operator::LocalGet { local_index } => Ok(WasmInstruction::LocalGet { local_index: *local_index }),
            wasmparser::Operator::LocalSet { local_index } => Ok(WasmInstruction::LocalSet { local_index: *local_index }),
            wasmparser::Operator::GlobalGet { global_index } => Ok(WasmInstruction::GlobalGet { global_index: *global_index }),
            wasmparser::Operator::GlobalSet { global_index } => Ok(WasmInstruction::GlobalSet { global_index: *global_index }),

            // Add more operators as needed...
            _ => Err(WasmError::unsupported_feature(format!("Operator: {:?}", op))),
        }
    }

    /// Convert wasmparser block type to our type
    fn convert_block_type(&self, block_type: &wasmparser::BlockType) -> WasmResult<Option<WasmValueType>> {
        match block_type {
            wasmparser::BlockType::Empty => Ok(None),
            wasmparser::BlockType::Type(val_type) => Ok(Some(self.convert_value_type(val_type)?)),
            wasmparser::BlockType::FuncType(_) => {
                if !self.config.allow_multi_value {
                    return Err(WasmError::unsupported_feature("Multi-value block types"));
                }
                Err(WasmError::unsupported_feature("Multi-value block types not yet implemented"))
            }
        }
    }

    /// Finalize functions by combining types and code
    fn finalize_functions(&mut self, module: &mut WasmModule, type_section: Vec<WasmFunctionType>, function_section: Vec<u32>, code_section: Vec<WasmFunction>) -> WasmResult<()> {
        module.types = type_section;
        module.function_types = function_section;
        module.functions = code_section;
        Ok(())
    }

    /// Validate the parsed module
    fn validate_module(&self, module: &WasmModule) -> WasmResult<()> {
        module.validate().map_err(|e| WasmError::validation_failed(e))?;
        Ok(())
    }

    /// Get the parser context
    pub fn context(&self) -> &ParserContext {
        &self.context
    }

    /// Get mutable access to the parser context
    pub fn context_mut(&mut self) -> &mut ParserContext {
        &mut self.context
    }

    /// Get the parser configuration
    pub fn config(&self) -> &ParserConfig {
        &self.config
    }
}

impl Default for WasmParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = WasmParser::new();
        assert!(parser.config.validate_structure);

        let config = ParserConfig::strict();
        let strict_parser = WasmParser::with_config(config);
        assert!(strict_parser.config.strict_validation);
    }

    #[test]
    fn test_invalid_wasm_version() {
        let mut parser = WasmParser::new();

        // Create a WASM binary with invalid version
        let invalid_wasm = b"\0asm\x02\x00\x00\x00"; // Version 2

        let result = parser.parse(invalid_wasm);
        assert!(result.is_err());

        if let Err(WasmError::UnsupportedVersion { version }) = result {
            assert_eq!(version, 2);
        } else {
            panic!("Expected UnsupportedVersion error");
        }
    }

    #[test]
    fn test_empty_module() {
        let mut parser = WasmParser::new();

        // Minimal valid WASM module (just header)
        let minimal_wasm = b"\0asm\x01\x00\x00\x00";

        let result = parser.parse(minimal_wasm);
        assert!(result.is_ok());

        let module = result.unwrap();
        assert_eq!(module.types.len(), 0);
        assert_eq!(module.functions.len(), 0);
    }
}
