// Dotlanth
//! Implements BytecodeWriter trait for safe binary output

use super::traits::{BytecodeWriter as WriterTrait, PatchPoint, WriteError};
use crate::codegen::error::{BytecodeGenerationError, BytecodeResult};

/// Concrete bytecode writer with buffering and patching
pub struct BytecodeWriter {
    buffer: Vec<u8>,
    max_size: Option<usize>,
}

impl BytecodeWriter {
    /// Create a new writer without size limit
    pub fn new() -> Self {
        BytecodeWriter { buffer: Vec::new(), max_size: None }
    }

    /// Create a new writer with a maximum buffer size
    pub fn with_max_size(limit: usize) -> Self {
        BytecodeWriter {
            buffer: Vec::new(),
            max_size: Some(limit),
        }
    }

    /// Get read-only access to the internal buffer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl WriterTrait for BytecodeWriter {
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), WriteError> {
        if let Some(max) = self.max_size {
            if self.buffer.len() + data.len() > max {
                return Err(WriteError::BufferOverflow);
            }
        }
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn write_section(&mut self, section: &[u8]) -> Result<(), WriteError> {
        WriterTrait::write_bytes(self, section)
    }

    fn create_patch_point(&mut self) -> PatchPoint {
        PatchPoint { offset: self.buffer.len() }
    }

    fn apply_patch(&mut self, patch: PatchPoint, data: &[u8]) -> Result<(), WriteError> {
        let end = patch.offset + data.len();
        if end > self.buffer.len() {
            return Err(WriteError::InvalidPatchPoint);
        }
        self.buffer[patch.offset..end].copy_from_slice(data);
        Ok(())
    }
}

impl BytecodeWriter {
    /// Write a single byte
    pub fn write_u8(&mut self, value: u8) -> BytecodeResult<()> {
        self.write_bytes(&[value]).map_err(|e| BytecodeGenerationError::WriteError(format!("{:?}", e)))
    }

    /// Write a 16-bit unsigned integer (little-endian)
    pub fn write_u16(&mut self, value: u16) -> BytecodeResult<()> {
        self.write_bytes(&value.to_le_bytes()).map_err(|e| BytecodeGenerationError::WriteError(format!("{:?}", e)))
    }

    /// Write a 32-bit unsigned integer (little-endian)
    pub fn write_u32(&mut self, value: u32) -> BytecodeResult<()> {
        self.write_bytes(&value.to_le_bytes()).map_err(|e| BytecodeGenerationError::WriteError(format!("{:?}", e)))
    }

    /// Write a string with length prefix
    pub fn write_string(&mut self, s: &str) -> BytecodeResult<()> {
        let bytes = s.as_bytes();
        self.write_u32(bytes.len() as u32)?;
        self.write_bytes(bytes).map_err(|e| BytecodeGenerationError::WriteError(format!("{:?}", e)))
    }

    /// Get current position in buffer
    pub fn position(&self) -> usize {
        self.buffer.len()
    }

    /// Get current buffer size
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get the final bytecode
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    /// Write bytes directly (public version of trait method)
    pub fn write_bytes(&mut self, data: &[u8]) -> BytecodeResult<()> {
        WriterTrait::write_bytes(self, data).map_err(|e| BytecodeGenerationError::WriteError(format!("{:?}", e)))
    }

    /// Write at a specific offset
    pub fn write_at_offset(&mut self, offset: usize, data: &[u8]) -> BytecodeResult<()> {
        if offset + data.len() > self.buffer.len() {
            return Err(BytecodeGenerationError::WriteError("Write beyond buffer".to_string()));
        }
        self.buffer[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
}

impl Default for BytecodeWriter {
    fn default() -> Self {
        Self::new()
    }
}
