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

//! Common section generator traits and types

use crate::codegen::core::context::GenerationContext;
use crate::codegen::error::BytecodeResult;

/// Identifier for bytecode sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionType {
    Header,
    FunctionTable,
    Code,
    Data,
    ExportTable,
    ImportTable,
    DebugInfo,
}

/// Trait for generating a specific bytecode section
pub trait SectionGenerator {
    /// Produce the raw bytes for this section given a generation context
    fn generate(&self, context: &GenerationContext) -> BytecodeResult<Vec<u8>>;

    /// Estimate the size (in bytes) of the section
    fn size_estimate(&self, context: &GenerationContext) -> usize;

    /// The type of section this generator produces
    fn section_type(&self) -> SectionType;

    /// Other section dependencies that must be generated first
    fn dependencies(&self) -> &'static [SectionType];
}

/// Trait for validating a completed section
pub trait SectionValidator {
    /// Validate the section bytes (e.g. header correctness, alignment)
    fn validate(&self, section: &[u8]) -> BytecodeResult<()>;
}
