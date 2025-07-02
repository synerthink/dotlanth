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

//! Memory layout processing for transpilation

use super::super::{
    config::TranspilationConfig,
    error::{TranspilationError, TranspilationResult},
    types::MemoryLayout,
};
use crate::wasm::ast::WasmMemory;

/// Processor for memory layout
pub struct MemoryProcessor;

impl MemoryProcessor {
    /// Create a new memory processor
    pub fn new(_config: &TranspilationConfig) -> TranspilationResult<Self> {
        Ok(Self)
    }

    /// Process memory layout
    pub fn process_memory(&mut self, wasm_memories: &[WasmMemory], _config: &TranspilationConfig) -> TranspilationResult<MemoryLayout> {
        if let Some(memory) = wasm_memories.first() {
            Ok(MemoryLayout::new(memory.initial_pages(), 65536).with_max_pages(memory.max_pages().unwrap_or(u32::MAX)))
        } else {
            Ok(MemoryLayout::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transpiler::config::TranspilationConfig;

    #[test]
    fn test_memory_processor_creation() {
        let config = TranspilationConfig::default();
        let processor = MemoryProcessor::new(&config);
        assert!(processor.is_ok());
    }
}
