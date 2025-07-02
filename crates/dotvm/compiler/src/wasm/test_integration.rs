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

//! Integration tests for the new WASM module

#[cfg(test)]
mod tests {
    use super::super::{
        ast::*,
        error::WasmError,
        features::FeatureDetector,
        mapping::OpcodeMapper,
        parser::{ParserConfig, ParserConfigBuilder, WasmParser},
        validation::WasmValidator,
    };
    use dotvm_core::bytecode::VmArchitecture;

    #[test]
    fn test_parser_with_different_configs() {
        // Test default config
        let default_parser = WasmParser::new();
        assert!(default_parser.config().validate_structure);

        // Test strict config
        let strict_config = ParserConfig::strict();
        let strict_parser = WasmParser::with_config(strict_config);
        assert!(strict_parser.config().strict_validation);

        // Test permissive config
        let permissive_config = ParserConfig::permissive();
        let permissive_parser = WasmParser::with_config(permissive_config);
        assert!(permissive_parser.config().is_feature_enabled("simd"));
    }

    #[test]
    fn test_config_builder() {
        let config = ParserConfigBuilder::new().validation(false).simd(true).bulk_memory(true).max_nesting_depth(500).build().unwrap();

        assert!(!config.validate_structure);
        assert!(config.allow_simd);
        assert!(config.allow_bulk_memory);
        assert_eq!(config.max_nesting_depth, 500);
    }

    #[test]
    fn test_empty_wasm_module() {
        let mut parser = WasmParser::new();

        // Minimal valid WASM module (just header)
        let minimal_wasm = b"\0asm\x01\x00\x00\x00";

        let result = parser.parse(minimal_wasm);
        assert!(result.is_ok());

        let module = result.unwrap();
        assert_eq!(module.types.len(), 0);
        assert_eq!(module.functions.len(), 0);
        assert_eq!(module.imports.len(), 0);
        assert_eq!(module.exports.len(), 0);
    }

    #[test]
    fn test_invalid_wasm_version() {
        let mut parser = WasmParser::new();

        // WASM binary with invalid version
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
    fn test_invalid_magic_number() {
        let mut parser = WasmParser::new();

        // Invalid magic number
        let invalid_wasm = b"WASM\x01\x00\x00\x00";

        let result = parser.parse(invalid_wasm);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasm_value_types() {
        assert_eq!(WasmValueType::I32.size_bytes(), 4);
        assert_eq!(WasmValueType::I64.size_bytes(), 8);
        assert_eq!(WasmValueType::F32.size_bytes(), 4);
        assert_eq!(WasmValueType::F64.size_bytes(), 8);
        assert_eq!(WasmValueType::V128.size_bytes(), 16);

        assert!(WasmValueType::I32.is_numeric());
        assert!(WasmValueType::I32.is_integer());
        assert!(!WasmValueType::I32.is_float());
        assert!(!WasmValueType::I32.is_reference());

        assert!(WasmValueType::F32.is_numeric());
        assert!(!WasmValueType::F32.is_integer());
        assert!(WasmValueType::F32.is_float());

        assert!(WasmValueType::FuncRef.is_reference());
        assert!(!WasmValueType::FuncRef.is_numeric());
    }

    #[test]
    fn test_function_type() {
        let func_type = WasmFunctionType::new(vec![WasmValueType::I32, WasmValueType::I32], vec![WasmValueType::I32]);

        assert_eq!(func_type.param_count(), 2);
        assert_eq!(func_type.result_count(), 1);
        assert!(func_type.has_params());
        assert!(func_type.has_result());

        let signature = func_type.signature_string();
        assert_eq!(signature, "(i32, i32) -> (i32)");
    }

    #[test]
    fn test_wasm_instructions() {
        let add_inst = WasmInstruction::I32Add;
        assert!(add_inst.is_arithmetic());
        assert!(!add_inst.affects_control_flow());
        assert!(!add_inst.accesses_memory());
        assert_eq!(add_inst.result_type(), Some(WasmValueType::I32));
        assert_eq!(add_inst.name(), "i32.add");

        let load_inst = WasmInstruction::I32Load { memarg: MemArg::default() };
        assert!(load_inst.accesses_memory());
        assert!(!load_inst.is_arithmetic());
        assert_eq!(load_inst.result_type(), Some(WasmValueType::I32));

        let br_inst = WasmInstruction::Br { label_index: 0 };
        assert!(br_inst.affects_control_flow());
        assert!(!br_inst.is_arithmetic());
        assert_eq!(br_inst.result_type(), None);

        let const_inst = WasmInstruction::I32Const { value: 42 };
        assert!(const_inst.is_constant());
        assert_eq!(const_inst.result_type(), Some(WasmValueType::I32));

        let simd_inst = WasmInstruction::V128Load { memarg: MemArg::default() };
        assert!(simd_inst.is_simd());
        assert!(simd_inst.accesses_memory());
        assert_eq!(simd_inst.result_type(), Some(WasmValueType::V128));
    }

    #[test]
    fn test_module_structure() {
        let mut module = WasmModule::new();

        // Add an imported function
        module.imports.push(WasmImport::function("env".to_string(), "print".to_string(), 0));

        // Add a defined function
        module.functions.push(WasmFunction::new(WasmFunctionType::empty(), vec![], vec![]));

        // Add an export
        module.exports.push(WasmExport::function("main".to_string(), 1));

        assert_eq!(module.import_function_count(), 1);
        assert_eq!(module.total_function_count(), 2);

        let export = module.find_export("main");
        assert!(export.is_some());
        assert_eq!(export.unwrap().index, 1);

        let missing = module.find_export("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_opcode_mapper() {
        let mapper = OpcodeMapper::new(VmArchitecture::Arch64);

        let add_inst = WasmInstruction::I32Add;
        let mapped = mapper.map_instruction(&add_inst);
        assert!(mapped.is_ok());

        let mapped_instructions = mapped.unwrap();
        assert_eq!(mapped_instructions.len(), 1);
        assert_eq!(mapped_instructions[0].opcode, "i32.add");

        let const_inst = WasmInstruction::I32Const { value: 42 };
        let mapped = mapper.map_instruction(&const_inst);
        assert!(mapped.is_ok());

        let mapped_instructions = mapped.unwrap();
        assert_eq!(mapped_instructions.len(), 1);
        assert_eq!(mapped_instructions[0].opcode, "i32.const");
        assert_eq!(mapped_instructions[0].operands, vec![42]);
    }

    #[test]
    fn test_feature_detection() {
        let simd_inst = WasmInstruction::V128Load { memarg: MemArg::default() };
        let features = FeatureDetector::detect_features(&simd_inst);
        assert!(features.contains(&"simd".to_string()));

        let bulk_inst = WasmInstruction::MemoryCopy;
        let features = FeatureDetector::detect_features(&bulk_inst);
        assert!(features.contains(&"bulk_memory".to_string()));

        let ref_inst = WasmInstruction::RefNull { ref_type: WasmValueType::FuncRef };
        let features = FeatureDetector::detect_features(&ref_inst);
        assert!(features.contains(&"reference_types".to_string()));

        assert!(FeatureDetector::requires_feature(&simd_inst, "simd"));
        assert!(!FeatureDetector::requires_feature(&simd_inst, "bulk_memory"));
    }

    #[test]
    fn test_architecture_requirements() {
        let regular_inst = WasmInstruction::I32Add;
        let arch = OpcodeMapper::required_architecture(&regular_inst);
        assert_eq!(arch, VmArchitecture::Arch64);

        let simd_inst = WasmInstruction::V128Load { memarg: MemArg::default() };
        let arch = OpcodeMapper::required_architecture(&simd_inst);
        assert_eq!(arch, VmArchitecture::Arch128);
    }

    #[test]
    fn test_memory_arg() {
        let memarg = MemArg::new(8, 2);
        assert_eq!(memarg.offset, 8);
        assert_eq!(memarg.align, 2);
        assert_eq!(memarg.alignment_bytes(), 4);
        assert!(memarg.is_valid_alignment());

        let invalid_memarg = MemArg::new(0, 3); // 3 is not a power of 2
        assert!(!invalid_memarg.is_valid_alignment());

        let default_memarg = MemArg::default();
        assert_eq!(default_memarg.offset, 0);
        assert_eq!(default_memarg.align, 0);
    }

    #[test]
    fn test_data_segment() {
        let segment = WasmDataSegment::new(0, vec![WasmInstruction::I32Const { value: 0 }], vec![1, 2, 3, 4]);

        assert_eq!(segment.memory_index, 0);
        assert_eq!(segment.size(), 4);
        assert!(!segment.is_empty());

        let empty_segment = WasmDataSegment::new(0, vec![], vec![]);
        assert!(empty_segment.is_empty());
    }

    #[test]
    fn test_custom_section() {
        let section = WasmCustomSection::new("name".to_string(), vec![1, 2, 3]);
        assert_eq!(section.name, "name");
        assert_eq!(section.size(), 3);
        assert!(section.is_name_section());
        assert!(!section.is_producers_section());

        let producers_section = WasmCustomSection::new("producers".to_string(), vec![]);
        assert!(producers_section.is_producers_section());
        assert!(!producers_section.is_name_section());
    }

    #[test]
    fn test_wasm_validator() {
        let validator = WasmValidator::new(false);
        let module = WasmModule::new();

        let result = validator.validate(&module);
        assert!(result.is_ok());

        let strict_validator = WasmValidator::new(true);
        let result = strict_validator.validate(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_categories() {
        let parse_error = WasmError::invalid_binary("test");
        assert_eq!(parse_error.category().as_str(), "parsing");
        assert!(!parse_error.is_recoverable());

        let validation_error = WasmError::validation_failed("test");
        assert_eq!(validation_error.category().as_str(), "validation");
        assert!(validation_error.is_recoverable());

        let feature_error = WasmError::unsupported_feature("test");
        assert_eq!(feature_error.category().as_str(), "feature");
        assert!(feature_error.is_recoverable());
    }

    #[test]
    fn test_section_limits() {
        use super::super::ast::sections::{SectionLimits, WasmSectionType};

        let limits = SectionLimits::strict();

        // Valid count
        assert!(limits.validate_count(WasmSectionType::Function, 500).is_ok());

        // Invalid count
        assert!(limits.validate_count(WasmSectionType::Function, 2000).is_err());

        // Valid size
        assert!(limits.validate_size(WasmSectionType::Code, 500000).is_ok());

        // Invalid size
        assert!(limits.validate_size(WasmSectionType::Code, 2000000).is_err());
    }

    #[test]
    fn test_parser_context_and_metrics() {
        let mut parser = WasmParser::new();

        // Parse a minimal module to generate some metrics
        let minimal_wasm = b"\0asm\x01\x00\x00\x00";
        let result = parser.parse(minimal_wasm);
        assert!(result.is_ok());

        let context = parser.context();
        // Note: warnings may be generated during parsing, so we just check it's not excessive
        assert!(context.warnings.len() <= 5);
        assert!(context.metrics.total_time.is_some());
        assert_eq!(context.metrics.bytes_parsed, 8);
    }
}
