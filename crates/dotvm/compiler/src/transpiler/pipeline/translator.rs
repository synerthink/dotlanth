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

//! Translation stage for converting WASM to DotVM bytecode

use super::{
    super::{
        config::TranspilationConfig,
        error::{TranspilationError, TranspilationResult},
        processors::{ExportsProcessor, FunctionProcessor, GlobalsProcessor, MemoryProcessor, ModuleProcessor},
        types::TranspiledModule,
    },
    PipelineStage,
    analyzer::AnalysisResult,
};
use dotvm_core::bytecode::BytecodeHeader;

/// Translation stage that converts analyzed WASM to DotVM bytecode
pub struct Translator {
    /// Function processor
    function_processor: FunctionProcessor,
    /// Module processor
    module_processor: ModuleProcessor,
    /// Globals processor
    globals_processor: GlobalsProcessor,
    /// Memory processor
    memory_processor: MemoryProcessor,
    /// Exports processor
    exports_processor: ExportsProcessor,
}

impl Translator {
    /// Create a new translator
    pub fn new(config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self {
            function_processor: FunctionProcessor::new(config)?,
            module_processor: ModuleProcessor::new(config)?,
            globals_processor: GlobalsProcessor::new(config)?,
            memory_processor: MemoryProcessor::new(config)?,
            exports_processor: ExportsProcessor::new(config)?,
        })
    }
}

impl PipelineStage for Translator {
    type Input = AnalysisResult;
    type Output = TranspiledModule;

    fn execute(&mut self, input: Self::Input, config: &TranspilationConfig) -> TranspilationResult<Self::Output> {
        // Create module header
        let header = BytecodeHeader::new(config.target_architecture);
        let mut transpiled_module = TranspiledModule::new(header);

        // Process the module structure
        self.module_processor.process_module(&input.module, &mut transpiled_module, config)?;

        // Process functions
        let functions = self.function_processor.process_functions(&input.module.functions, &input.function_analyses, config)?;

        for function in functions {
            transpiled_module.add_function(function);
        }

        // Process globals
        let globals = self.globals_processor.process_globals(&input.module.globals, config)?;
        for global in globals {
            transpiled_module.add_global(global);
        }

        // Process memory layout
        let memory_layout = self.memory_processor.process_memory(&input.module.memories, config)?;
        transpiled_module.set_memory_layout(memory_layout);

        // Process exports and imports
        let (exports, imports) = self.exports_processor.process_exports_imports(&input.module.exports, &input.module.imports, config)?;

        for export in exports {
            transpiled_module.add_export(export);
        }

        for import in imports {
            transpiled_module.add_import(import);
        }

        // Set module metadata from analysis
        transpiled_module.metadata.set_complexity_score(input.function_analyses.iter().map(|f| f.complexity_score).sum());

        transpiled_module.metadata.estimated_size = input.performance_profile.estimated_memory_usage;

        if input.performance_profile.is_cpu_intensive {
            transpiled_module.metadata.add_optimization_hint("cpu_intensive".to_string());
        }

        if input.performance_profile.is_memory_intensive {
            transpiled_module.metadata.add_optimization_hint("memory_intensive".to_string());
        }

        Ok(transpiled_module)
    }

    fn name(&self) -> &'static str {
        "translator"
    }

    fn estimated_duration(&self, input_size: usize) -> std::time::Duration {
        // Translation is the most intensive stage, roughly 10ms per KB
        std::time::Duration::from_millis((input_size * 10 / 1024).max(5) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_translator_creation() {
        let config = TranspilationConfig::default();
        let translator = Translator::new(&config);
        assert!(translator.is_ok());
    }
}
