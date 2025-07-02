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

//! Calling convention adaptation for different architectures

use super::super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::TranspiledFunction,
};
use dotvm_core::bytecode::VmArchitecture;

/// Calling convention adapter
pub struct CallingConventionAdapter {
    /// Target architecture
    target_architecture: VmArchitecture,
}

impl CallingConventionAdapter {
    /// Create a new calling convention adapter
    pub fn new(config: &TranspilationConfig) -> Self {
        Self {
            target_architecture: config.target_architecture,
        }
    }

    /// Adapt function calling convention
    pub fn adapt_function(&self, function: &mut TranspiledFunction) -> TranspilationResult<()> {
        match self.target_architecture {
            VmArchitecture::Arch64 => {
                self.adapt_for_64bit(function)?;
            }
            VmArchitecture::Arch128 => {
                self.adapt_for_128bit(function)?;
            }
            VmArchitecture::Arch256 => {
                self.adapt_for_256bit(function)?;
            }
            _ => {
                return Err(TranspilationError::UnsupportedFeature(format!("Calling convention adaptation for {:?}", self.target_architecture)));
            }
        }

        Ok(())
    }

    /// Adapt for 64-bit architecture calling convention
    fn adapt_for_64bit(&self, _function: &mut TranspiledFunction) -> TranspilationResult<()> {
        // 64-bit calling convention adaptations
        Ok(())
    }

    /// Adapt for 128-bit architecture calling convention
    fn adapt_for_128bit(&self, _function: &mut TranspiledFunction) -> TranspilationResult<()> {
        // 128-bit calling convention adaptations
        Ok(())
    }

    /// Adapt for 256-bit architecture calling convention
    fn adapt_for_256bit(&self, _function: &mut TranspiledFunction) -> TranspilationResult<()> {
        // 256-bit calling convention adaptations
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::{config::TranspilationConfig, types::TranspiledFunction};

    #[test]
    fn test_calling_convention_adapter() {
        let config = TranspilationConfig::default();
        let adapter = CallingConventionAdapter::new(&config);
        let mut function = TranspiledFunction::new("test".to_string(), 0, 0);

        let result = adapter.adapt_function(&mut function);
        assert!(result.is_ok());
    }
}
