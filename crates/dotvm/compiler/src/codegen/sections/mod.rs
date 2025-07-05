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

//! Bytecode section generators
//!
//! This module contains specialized generators for different sections of the bytecode:
//! - Header section
//! - Function table section  
//! - Code section
//! - Data section
//! - Export/Import tables
//! - Debug information

pub mod code;
pub mod data;
pub mod debug;
pub mod export_import;
pub mod function_table;
pub mod header;
pub mod traits;

pub use code::CodeGenerator;
pub use data::DataGenerator;
pub use debug::{DebugInfo, DebugInfoGenerator};
pub use export_import::{ExportTable, ExportTableGenerator, ImportTable, ImportTableGenerator};
pub use function_table::{FunctionTable, FunctionTableGenerator};
pub use header::HeaderGenerator;
pub use traits::{SectionGenerator, SectionType, SectionValidator};
