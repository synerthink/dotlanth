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

//! Code generation module
//!
//! This module provides a well-structured bytecode generation system with clear
//! separation of concerns:
//!
//! - `config`: Configuration management
//! - `error`: Error types and handling
//! - `writer`: Safe bytecode writing utilities
//! - `sections`: Specialized generators for different bytecode sections
//! - `optimizer`: Post-generation optimization passes
//! - `generator`: Main orchestrator that coordinates all generation phases

pub mod config;
pub mod error;
pub mod generator;
pub mod optimizer;
pub mod sections;
pub mod writer;

// Re-export main types
pub use config::BytecodeGenerationConfig;
pub use error::{BytecodeGenerationError, BytecodeResult};
pub use generator::{BytecodeGenerator, DotVMGenerator, GeneratedBytecode, GenerationStats};
pub use optimizer::BytecodeOptimizer;
pub use writer::{BytecodeWriter, PatchPoint};

// Re-export section types
pub use sections::{
    CodeGenerator, DataGenerator, DebugInfo, DebugInfoGenerator, ExportTable, ExportTableGenerator, FunctionTable, FunctionTableGenerator, HeaderGenerator, ImportTable, ImportTableGenerator,
};
