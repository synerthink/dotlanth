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

//! Processing components for different aspects of transpilation

pub mod exports_processor;
pub mod function_processor;
pub mod globals_processor;
pub mod instruction_processor;
pub mod memory_processor;
pub mod module_processor;

// Re-export processors
pub use exports_processor::ExportsProcessor;
pub use function_processor::FunctionProcessor;
pub use globals_processor::GlobalsProcessor;
pub use instruction_processor::InstructionProcessor;
pub use memory_processor::MemoryProcessor;
pub use module_processor::ModuleProcessor;
