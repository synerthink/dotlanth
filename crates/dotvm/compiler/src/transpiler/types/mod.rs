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

//! Type definitions for the transpiler module
//!
//! This module contains all the data structures used throughout the transpilation
//! process, organized by their primary purpose and usage patterns.

pub mod exports_imports;
pub mod function;
pub mod instruction;
pub mod module;
pub mod variables;

// Re-export commonly used types for convenience
pub use exports_imports::*;
pub use function::*;
pub use instruction::*;
pub use module::*;
pub use variables::*;
