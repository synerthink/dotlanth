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

//! Instruction set adaptation for different architectures

use super::super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::TranspiledInstruction,
};
use dotvm_core::bytecode::VmArchitecture;

/// Instruction set adapter
pub struct InstructionSetAdapter {
    /// Target architecture
    target_architecture: VmArchitecture,
}

impl InstructionSetAdapter {
    /// Create a new instruction set adapter
    pub fn new(config: &TranspilationConfig) -> Self {
        Self {
            target_architecture: config.target_architecture,
        }
    }

    /// Adapt instruction for target architecture
    pub fn adapt_instruction(&self, instruction: &mut TranspiledInstruction) -> TranspilationResult<()> {
        match self.target_architecture {
            VmArchitecture::Arch64 => {
                self.adapt_for_64bit(instruction)?;
            }
            VmArchitecture::Arch128 => {
                self.adapt_for_128bit(instruction)?;
            }
            VmArchitecture::Arch256 => {
                self.adapt_for_256bit(instruction)?;
            }
            _ => {
                return Err(TranspilationError::UnsupportedFeature(format!("Instruction adaptation for {:?}", self.target_architecture)));
            }
        }

        Ok(())
    }

    /// Adapt for 64-bit architecture
    fn adapt_for_64bit(&self, _instruction: &mut TranspiledInstruction) -> TranspilationResult<()> {
        // 64-bit specific instruction adaptations
        Ok(())
    }

    /// Adapt for 128-bit architecture
    fn adapt_for_128bit(&self, _instruction: &mut TranspiledInstruction) -> TranspilationResult<()> {
        // 128-bit specific instruction adaptations
        Ok(())
    }

    /// Adapt for 256-bit architecture
    fn adapt_for_256bit(&self, _instruction: &mut TranspiledInstruction) -> TranspilationResult<()> {
        // 256-bit specific instruction adaptations
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::{
        config::TranspilationConfig,
        types::{Operand, TranspiledInstruction},
    };

    #[test]
    fn test_instruction_set_adapter() {
        let config = TranspilationConfig::default();
        let adapter = InstructionSetAdapter::new(&config);
        let mut instruction = TranspiledInstruction::new("i32.add".to_string(), vec![Operand::immediate(1), Operand::immediate(2)]);

        let result = adapter.adapt_instruction(&mut instruction);
        assert!(result.is_ok());
    }
}
