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

//! Global variable processing for transpilation

use super::super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::{GlobalVariable, VariableType},
};
use crate::wasm::ast::WasmGlobal;

/// Processor for global variables
pub struct GlobalsProcessor;

impl GlobalsProcessor {
    /// Create a new globals processor
    pub fn new(_config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self)
    }

    /// Process global variables
    pub fn process_globals(&mut self, wasm_globals: &[WasmGlobal], _config: &TranspilationConfig) -> TranspilationResult<Vec<GlobalVariable>> {
        let mut globals = Vec::new();

        for (index, global) in wasm_globals.iter().enumerate() {
            globals.push(GlobalVariable::new(
                index as u32,
                VariableType::I32, // Simplified - would need proper type mapping
                global.is_mutable(),
            ));
        }

        Ok(globals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_globals_processor_creation() {
        let config = TranspilationConfig::default();
        let processor = GlobalsProcessor::new(&config);
        assert!(processor.is_ok());
    }
}
