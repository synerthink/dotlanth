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

//! Configuration for bytecode generation

use dotvm_core::bytecode::VmArchitecture;

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

    /// Optimization level (0-3)
    pub optimization_level: u8,

    /// Whether to enable dead code elimination
    pub enable_dead_code_elimination: bool,

    /// Whether to enable constant folding
    pub enable_constant_folding: bool,
}

impl Default for BytecodeGenerationConfig {
    fn default() -> Self {
        Self {
            enable_optimizations: true,
            include_debug_info: false,
            enable_compression: false,
            target_architecture: VmArchitecture::Arch64,
            max_bytecode_size: None,
            optimization_level: 2,
            enable_dead_code_elimination: true,
            enable_constant_folding: true,
        }
    }
}

impl BytecodeGenerationConfig {
    /// Create a new configuration for the given architecture
    pub fn for_architecture(arch: VmArchitecture) -> Self {
        Self {
            target_architecture: arch,
            ..Default::default()
        }
    }

    /// Create a debug configuration with debug info enabled
    pub fn debug() -> Self {
        Self {
            include_debug_info: true,
            enable_optimizations: false,
            optimization_level: 0,
            enable_dead_code_elimination: false,
            enable_constant_folding: false,
            ..Default::default()
        }
    }

    /// Create a release configuration with maximum optimizations
    pub fn release() -> Self {
        Self {
            enable_optimizations: true,
            enable_compression: true,
            optimization_level: 3,
            include_debug_info: false,
            ..Default::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.optimization_level > 3 {
            return Err("Optimization level must be between 0 and 3".to_string());
        }

        if let Some(max_size) = self.max_bytecode_size {
            if max_size == 0 {
                return Err("Maximum bytecode size must be greater than 0".to_string());
            }
        }

        Ok(())
    }
}
