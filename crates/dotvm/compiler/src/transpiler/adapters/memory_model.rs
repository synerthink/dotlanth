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

//! Memory model adaptation for different architectures

use super::super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::MemoryLayout,
};
use dotvm_core::bytecode::VmArchitecture;

/// Memory model adapter
pub struct MemoryModelAdapter {
    /// Target architecture
    target_architecture: VmArchitecture,
}

impl MemoryModelAdapter {
    /// Create a new memory model adapter
    pub fn new(config: &TranspilationConfig) -> Self {
        Self {
            target_architecture: config.target_architecture,
        }
    }

    /// Adapt memory layout for target architecture
    pub fn adapt_memory_layout(&self, layout: &mut MemoryLayout) -> TranspilationResult<()> {
        match self.target_architecture {
            VmArchitecture::Arch64 => {
                // 64-bit architecture adaptations
                self.adapt_for_64bit(layout)?;
            }
            VmArchitecture::Arch128 => {
                // 128-bit architecture adaptations
                self.adapt_for_128bit(layout)?;
            }
            VmArchitecture::Arch256 => {
                // 256-bit architecture adaptations
                self.adapt_for_256bit(layout)?;
            }
            _ => {
                return Err(TranspilationError::UnsupportedFeature(format!("Memory model adaptation for {:?}", self.target_architecture)));
            }
        }

        Ok(())
    }

    /// Adapt for 64-bit architecture
    fn adapt_for_64bit(&self, _layout: &mut MemoryLayout) -> TranspilationResult<()> {
        // 64-bit specific adaptations
        Ok(())
    }

    /// Adapt for 128-bit architecture
    fn adapt_for_128bit(&self, _layout: &mut MemoryLayout) -> TranspilationResult<()> {
        // 128-bit specific adaptations
        Ok(())
    }

    /// Adapt for 256-bit architecture
    fn adapt_for_256bit(&self, _layout: &mut MemoryLayout) -> TranspilationResult<()> {
        // 256-bit specific adaptations
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_memory_model_adapter() {
        let config = TranspilationConfig::default();
        let adapter = MemoryModelAdapter::new(&config);
        let mut layout = MemoryLayout::default();

        let result = adapter.adapt_memory_layout(&mut layout);
        assert!(result.is_ok());
    }
}
