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

//! Core generation traits

use crate::codegen::error::BytecodeResult;
use crate::transpiler::types::TranspiledModule;
use dotvm_core::bytecode::VmArchitecture;

/// Trait for top-level bytecode generators
pub trait BytecodeGenerator {
    /// The output produced by the generator
    type Output;

    /// Error type for generation failures
    type Error;

    /// Generate bytecode (or other output) from a transpiled module
    fn generate(&mut self, input: &TranspiledModule) -> Result<Self::Output, Self::Error>;

    /// Check if this generator supports the given VM architecture
    fn supports_architecture(&self, arch: VmArchitecture) -> bool;

    /// Return the optimization level used by this generator
    fn optimization_level(&self) -> u8;
}

/// Trait for individual section generators
pub trait SectionGenerator {
    /// Section type produced by this generator
    type Section;

    /// Context needed to generate the section
    type Context;

    /// Generate the corresponding section from the context
    fn generate_section(&self, context: &Self::Context) -> BytecodeResult<Self::Section>;

    /// The kind of section this generator produces
    fn section_type(&self) -> &'static str;

    /// Other section dependencies that must be generated first
    fn dependencies(&self) -> Vec<&'static str>;
}
