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

//! Modular WebAssembly parser
//!
//! This module provides a clean, extensible parser architecture with
//! separate components for different aspects of WASM parsing.

pub mod config;
pub mod context;
pub mod core;

// Re-export main parser
pub use config::*;
pub use context::*;
pub use core::WasmParser;
