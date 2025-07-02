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

//! Instruction-level processing for transpilation

use super::super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::{Operand, TranspiledInstruction},
};
use crate::wasm::{ast::WasmInstruction, opcode_mapper::OpcodeMapper};

/// Processor for converting WASM instructions to DotVM instructions
pub struct InstructionProcessor {
    /// Opcode mapper
    opcode_mapper: OpcodeMapper,
}

impl InstructionProcessor {
    /// Create a new instruction processor
    pub fn new(config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self {
            opcode_mapper: OpcodeMapper::new(config.target_architecture),
        })
    }

    /// Process multiple instructions
    pub fn process_instructions(&mut self, wasm_instructions: &[WasmInstruction], config: &TranspilationConfig) -> TranspilationResult<Vec<TranspiledInstruction>> {
        let mut transpiled_instructions = Vec::new();

        for wasm_instruction in wasm_instructions {
            let mapped_instructions = self.opcode_mapper.map_instruction(wasm_instruction)?;

            for mapped in mapped_instructions {
                let transpiled = TranspiledInstruction::new(
                    format!("{:?}", mapped.opcode),
                    mapped
                        .operands
                        .iter()
                        .map(|&op| if op <= u32::MAX as u64 { Operand::immediate(op as u32) } else { Operand::large_immediate(op) })
                        .collect(),
                );

                transpiled_instructions.push(transpiled);
            }
        }

        Ok(transpiled_instructions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_instruction_processor_creation() {
        let config = TranspilationConfig::default();
        let processor = InstructionProcessor::new(&config);
        assert!(processor.is_ok());
    }
}
