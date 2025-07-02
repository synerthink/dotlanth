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

//! Modular AST definitions for WebAssembly
//!
//! This module provides a clean separation of different AST components
//! with clear responsibilities and extensible design.

pub mod instructions;
pub mod module;
pub mod sections;
pub mod types;

// Re-export commonly used types
pub use instructions::*;
pub use module::*;
pub use sections::*;
pub use types::*;
