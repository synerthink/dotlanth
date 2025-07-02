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

//! Transpiler module for converting WASM to DotVM bytecode
//!
//! This module contains the core transpilation engine that converts
//! WebAssembly modules into DotVM bytecode using a modular pipeline architecture.

// Core modules
pub mod config;
pub mod engine; // Legacy engine (will be deprecated)
pub mod engine_new; // New pipeline-based engine
pub mod error;
pub mod types;

// Pipeline modules
pub mod adapters;
pub mod analysis;
pub mod pipeline;
pub mod processors;

// Tests
#[cfg(test)]
pub mod test_basic;

// Re-export commonly used types and functions
pub use config::*;
pub use error::*;
pub use types::*;

// Re-export both engines for migration period
pub use engine::*; // Legacy engine
pub use engine_new::NewTranspilationEngine; // New engine

// Convenience type alias for the new engine
pub type TranspilationEngine = engine_new::NewTranspilationEngine;
