// Dotlanth
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
