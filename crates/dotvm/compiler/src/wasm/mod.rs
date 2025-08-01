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

//! Refactored WASM module with improved separation of concerns
//!
//! This module provides a clean, modular architecture for handling WebAssembly
//! parsing, AST representation, and opcode mapping with clear separation of
//! responsibilities and extensible design for WASM proposals.

// Core modules
pub mod ast;
pub mod error;

// Parsing modules
pub mod parser;
pub mod sections;
pub mod validation;

// Mapping modules
pub mod features;
pub mod mapping;

// Re-export commonly used types
pub use ast::*;
pub use error::*;
pub use mapping::OpcodeMapper;
pub use parser::WasmParser;

// Tests
#[cfg(test)]
pub mod test_integration;
