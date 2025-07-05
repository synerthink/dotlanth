// Dotlanth
//! Traits defining writer capabilities

/// Trait for low-level byte writing with patch support
pub trait BytecodeWriter {
    /// Write a sequence of bytes, potentially returning an error
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), WriteError>;

    /// Reserve a patch point and return its identifier
    fn create_patch_point(&mut self) -> PatchPoint;

    /// Apply data at a previously created patch point
    fn apply_patch(&mut self, patch: PatchPoint, data: &[u8]) -> Result<(), WriteError>;
    /// Write a complete section of raw bytes
    fn write_section(&mut self, section: &[u8]) -> Result<(), WriteError>;
}

/// Errors that may occur during writing
#[derive(Debug, Clone)]
pub enum WriteError {
    /// Attempt to write beyond the buffer limit
    BufferOverflow,
    /// Invalid patch point
    InvalidPatchPoint,
}

/// Location within the output buffer where data can be patched
#[derive(Debug, Clone, Copy)]
pub struct PatchPoint {
    pub offset: usize,
}
