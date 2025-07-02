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

//! Preprocessing stage for WASM input validation and normalization

use super::{
    super::{
        config::TranspilationConfig,
        error::{TranspilationError, TranspilationResult},
    },
    PipelineStage,
};
use crate::wasm::{ast::WasmModule, parser::WasmParser};

/// Preprocessor stage for input validation and normalization
pub struct Preprocessor {
    /// WASM parser
    parser: WasmParser,
    /// Validation configuration
    validation_config: ValidationConfig,
}

impl Preprocessor {
    /// Create a new preprocessor
    pub fn new(config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self {
            parser: WasmParser::new(),
            validation_config: ValidationConfig::from_transpilation_config(config),
        })
    }

    /// Validate WASM binary format
    fn validate_binary_format(&self, wasm_bytes: &[u8]) -> TranspilationResult<()> {
        // Check minimum size
        if wasm_bytes.len() < 8 {
            return Err(TranspilationError::preprocessing_error("binary_validation", "WASM binary too small (minimum 8 bytes required)"));
        }

        // Check magic number
        if &wasm_bytes[0..4] != b"\0asm" {
            return Err(TranspilationError::preprocessing_error("binary_validation", "Invalid WASM magic number"));
        }

        // Check version
        let version = u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);

        if version != 1 {
            return Err(TranspilationError::preprocessing_error(
                "binary_validation",
                format!("Unsupported WASM version: {} (expected 1)", version),
            ));
        }

        // Check maximum size
        if let Some(max_size) = self.validation_config.max_input_size {
            if wasm_bytes.len() > max_size {
                return Err(TranspilationError::preprocessing_error(
                    "binary_validation",
                    format!("WASM binary too large: {} bytes (max: {} bytes)", wasm_bytes.len(), max_size),
                ));
            }
        }

        Ok(())
    }

    /// Validate parsed WASM module structure
    fn validate_module_structure(&self, module: &WasmModule) -> TranspilationResult<()> {
        // Check function count limits
        if let Some(max_functions) = self.validation_config.max_functions {
            if module.functions.len() > max_functions {
                return Err(TranspilationError::preprocessing_error(
                    "structure_validation",
                    format!("Too many functions: {} (max: {})", module.functions.len(), max_functions),
                ));
            }
        }

        // Check global count limits
        if let Some(max_globals) = self.validation_config.max_globals {
            if module.globals.len() > max_globals {
                return Err(TranspilationError::preprocessing_error(
                    "structure_validation",
                    format!("Too many globals: {} (max: {})", module.globals.len(), max_globals),
                ));
            }
        }

        // Check memory limits
        if let Some(max_memories) = self.validation_config.max_memories {
            if module.memories.len() > max_memories {
                return Err(TranspilationError::preprocessing_error(
                    "structure_validation",
                    format!("Too many memories: {} (max: {})", module.memories.len(), max_memories),
                ));
            }
        }

        // Validate individual functions
        for (index, function) in module.functions.iter().enumerate() {
            self.validate_function(index as u32, function)?;
        }

        // Validate exports
        self.validate_exports(module)?;

        // Validate imports
        self.validate_imports(module)?;

        Ok(())
    }

    /// Validate a single function
    fn validate_function(&self, index: u32, function: &crate::wasm::ast::WasmFunction) -> TranspilationResult<()> {
        // Check function size limits
        if let Some(max_instructions) = self.validation_config.max_function_instructions {
            if function.body.len() > max_instructions {
                return Err(TranspilationError::preprocessing_error(
                    "function_validation",
                    format!("Function {} too large: {} instructions (max: {})", index, function.body.len(), max_instructions),
                ));
            }
        }

        // Check parameter count
        if let Some(max_params) = self.validation_config.max_function_params {
            if function.signature.params.len() > max_params {
                return Err(TranspilationError::preprocessing_error(
                    "function_validation",
                    format!("Function {} has too many parameters: {} (max: {})", index, function.signature.params.len(), max_params),
                ));
            }
        }

        // Check local variable count
        if let Some(max_locals) = self.validation_config.max_function_locals {
            if function.locals.len() > max_locals {
                return Err(TranspilationError::preprocessing_error(
                    "function_validation",
                    format!("Function {} has too many locals: {} (max: {})", index, function.locals.len(), max_locals),
                ));
            }
        }

        Ok(())
    }

    /// Validate module exports
    fn validate_exports(&self, module: &WasmModule) -> TranspilationResult<()> {
        // Check for duplicate export names
        let mut export_names = std::collections::HashSet::new();
        for export in &module.exports {
            if !export_names.insert(&export.name) {
                return Err(TranspilationError::preprocessing_error("export_validation", format!("Duplicate export name: {}", export.name)));
            }
        }

        // Validate export indices
        for export in &module.exports {
            match export.kind {
                crate::wasm::ast::WasmExportKind::Function => {
                    if export.index as usize >= module.functions.len() {
                        return Err(TranspilationError::preprocessing_error(
                            "export_validation",
                            format!("Export '{}' references non-existent function {}", export.name, export.index),
                        ));
                    }
                }
                crate::wasm::ast::WasmExportKind::Global => {
                    if export.index as usize >= module.globals.len() {
                        return Err(TranspilationError::preprocessing_error(
                            "export_validation",
                            format!("Export '{}' references non-existent global {}", export.name, export.index),
                        ));
                    }
                }
                crate::wasm::ast::WasmExportKind::Memory => {
                    if export.index as usize >= module.memories.len() {
                        return Err(TranspilationError::preprocessing_error(
                            "export_validation",
                            format!("Export '{}' references non-existent memory {}", export.name, export.index),
                        ));
                    }
                }
                crate::wasm::ast::WasmExportKind::Table => {
                    if export.index as usize >= module.tables.len() {
                        return Err(TranspilationError::preprocessing_error(
                            "export_validation",
                            format!("Export '{}' references non-existent table {}", export.name, export.index),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate module imports
    fn validate_imports(&self, module: &WasmModule) -> TranspilationResult<()> {
        // Check for duplicate import names within the same module
        let mut import_keys = std::collections::HashSet::new();
        for import in &module.imports {
            let key = format!("{}::{}", import.module, import.name);
            if !import_keys.insert(key.clone()) {
                return Err(TranspilationError::preprocessing_error("import_validation", format!("Duplicate import: {}", key)));
            }
        }

        Ok(())
    }

    /// Normalize the WASM module (apply transformations for consistency)
    fn normalize_module(&self, mut module: WasmModule) -> TranspilationResult<WasmModule> {
        // Sort exports by name for consistent output
        if self.validation_config.normalize_exports {
            module.exports.sort_by(|a, b| a.name.cmp(&b.name));
        }

        // Sort imports by module and name
        if self.validation_config.normalize_imports {
            module.imports.sort_by(|a, b| a.module.cmp(&b.module).then_with(|| a.name.cmp(&b.name)));
        }

        // Remove unused elements if enabled
        if self.validation_config.remove_unused_elements {
            module = self.remove_unused_elements(module)?;
        }

        Ok(module)
    }

    /// Remove unused elements from the module
    fn remove_unused_elements(&self, mut module: WasmModule) -> TranspilationResult<WasmModule> {
        // This is a simplified implementation
        // In practice, this would involve complex dependency analysis

        // For now, we'll just remove empty function bodies (if any)
        // and unused globals that are not exported or imported

        // Mark exported and imported items as used
        let mut used_functions = std::collections::HashSet::new();
        let mut used_globals = std::collections::HashSet::new();

        // Mark exports as used
        for export in &module.exports {
            match export.kind {
                crate::wasm::ast::WasmExportKind::Function => {
                    used_functions.insert(export.index);
                }
                crate::wasm::ast::WasmExportKind::Global => {
                    used_globals.insert(export.index);
                }
                _ => {}
            }
        }

        // Mark imports as used (they're external dependencies)
        for import in &module.imports {
            match import.kind {
                crate::wasm::ast::WasmImportKind::Function { .. } => {
                    // Import functions are implicitly used
                }
                crate::wasm::ast::WasmImportKind::Global { .. } => {
                    // Import globals are implicitly used
                }
                _ => {}
            }
        }

        // TODO: Implement proper dependency analysis to find all used elements
        // For now, we'll keep all elements to avoid breaking functionality

        Ok(module)
    }
}

impl PipelineStage for Preprocessor {
    type Input = Vec<u8>;
    type Output = WasmModule;

    fn execute(&mut self, input: Self::Input, _config: &TranspilationConfig) -> TranspilationResult<Self::Output> {
        // Step 1: Validate binary format
        self.validate_binary_format(&input)?;

        // Step 2: Parse WASM binary
        let module = self
            .parser
            .parse(&input)
            .map_err(|e| TranspilationError::preprocessing_error("parsing", format!("Failed to parse WASM binary: {}", e)))?;

        // Step 3: Validate module structure
        self.validate_module_structure(&module)?;

        // Step 4: Normalize module
        let normalized_module = self.normalize_module(module)?;

        Ok(normalized_module)
    }

    fn name(&self) -> &'static str {
        "preprocessor"
    }

    fn can_skip(&self, config: &TranspilationConfig) -> bool {
        // Never skip preprocessing - it's essential for safety
        false
    }

    fn estimated_duration(&self, input_size: usize) -> std::time::Duration {
        // Preprocessing is typically fast, roughly 1ms per KB
        std::time::Duration::from_millis((input_size / 1024).max(1) as u64)
    }
}

/// Configuration for validation during preprocessing
#[derive(Debug, Clone)]
struct ValidationConfig {
    /// Maximum input size in bytes
    max_input_size: Option<usize>,
    /// Maximum number of functions
    max_functions: Option<usize>,
    /// Maximum number of globals
    max_globals: Option<usize>,
    /// Maximum number of memories
    max_memories: Option<usize>,
    /// Maximum instructions per function
    max_function_instructions: Option<usize>,
    /// Maximum parameters per function
    max_function_params: Option<usize>,
    /// Maximum locals per function
    max_function_locals: Option<usize>,
    /// Whether to normalize exports
    normalize_exports: bool,
    /// Whether to normalize imports
    normalize_imports: bool,
    /// Whether to remove unused elements
    remove_unused_elements: bool,
}

impl ValidationConfig {
    /// Create validation config from transpilation config
    fn from_transpilation_config(config: &TranspilationConfig) -> Self {
        Self {
            max_input_size: Some(64 * 1024 * 1024), // 64MB default limit
            max_functions: Some(10000),             // Reasonable limit
            max_globals: Some(1000),
            max_memories: Some(1), // WASM spec allows only 1 memory currently
            max_function_instructions: config.max_function_size.map(|s| s as usize),
            max_function_params: Some(100),
            max_function_locals: Some(1000),
            normalize_exports: true,
            normalize_imports: true,
            remove_unused_elements: config.enable_optimizations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_preprocessor_creation() {
        let config = TranspilationConfig::default();
        let preprocessor = Preprocessor::new(&config);
        assert!(preprocessor.is_ok());
    }

    #[test]
    fn test_binary_validation() {
        let config = TranspilationConfig::default();
        let preprocessor = Preprocessor::new(&config).unwrap();

        // Test invalid magic number
        let invalid_wasm = b"invalid";
        assert!(preprocessor.validate_binary_format(invalid_wasm).is_err());

        // Test too small binary
        let too_small = b"abc";
        assert!(preprocessor.validate_binary_format(too_small).is_err());

        // Test valid header
        let valid_header = b"\0asm\x01\x00\x00\x00";
        assert!(preprocessor.validate_binary_format(valid_header).is_ok());
    }

    #[test]
    fn test_validation_config() {
        let config = TranspilationConfig::default();
        let validation_config = ValidationConfig::from_transpilation_config(&config);

        assert!(validation_config.normalize_exports);
        assert!(validation_config.normalize_imports);
        assert_eq!(validation_config.max_memories, Some(1));
    }
}
