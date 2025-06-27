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

//! DotVM Tools Library
//!
//! This crate provides command-line tools and utilities for working with DotVM,
//! including transpilation, optimization, and debugging tools.

pub mod cli;
pub mod utils;

// Re-export main CLI functions for easy access
pub use cli::transpile::{TranspilationPipeline, TranspileArgs, run_transpile_cli};
