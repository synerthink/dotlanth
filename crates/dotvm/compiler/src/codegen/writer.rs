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

//! Bytecode writer utilities for safe binary serialization

use super::error::{BytecodeGenerationError, BytecodeResult};

/// A writer for generating bytecode with proper error handling and bounds checking
#[derive(Clone)]
pub struct BytecodeWriter {
    buffer: Vec<u8>,
    position: usize,
    max_size: Option<usize>,
}

impl BytecodeWriter {
    /// Create a new bytecode writer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            position: 0,
            max_size: None,
        }
    }

    /// Create a new bytecode writer with a maximum size limit
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            position: 0,
            max_size: Some(max_size),
        }
    }

    /// Get the current position in the buffer
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the current size of the buffer
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// Get a reference to the buffer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Take ownership of the buffer
    pub fn into_buffer(self) -> Vec<u8> {
        self.buffer
    }

    /// Clear the buffer and reset position
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.position = 0;
    }

    /// Reserve space in the buffer
    pub fn reserve(&mut self, additional: usize) -> BytecodeResult<()> {
        let new_size = self.buffer.len() + additional;
        self.check_size_limit(new_size)?;
        self.buffer.reserve(additional);
        Ok(())
    }

    /// Write a single byte
    pub fn write_u8(&mut self, value: u8) -> BytecodeResult<()> {
        self.check_size_limit(self.buffer.len() + 1)?;
        self.buffer.push(value);
        self.position += 1;
        Ok(())
    }

    /// Write a 16-bit value in little-endian format
    pub fn write_u16(&mut self, value: u16) -> BytecodeResult<()> {
        let bytes = value.to_le_bytes();
        self.write_bytes(&bytes)
    }

    /// Write a 32-bit value in little-endian format
    pub fn write_u32(&mut self, value: u32) -> BytecodeResult<()> {
        let bytes = value.to_le_bytes();
        self.write_bytes(&bytes)
    }

    /// Write a 64-bit value in little-endian format
    pub fn write_u64(&mut self, value: u64) -> BytecodeResult<()> {
        let bytes = value.to_le_bytes();
        self.write_bytes(&bytes)
    }

    /// Write a slice of bytes
    pub fn write_bytes(&mut self, bytes: &[u8]) -> BytecodeResult<()> {
        let new_size = self.buffer.len() + bytes.len();
        self.check_size_limit(new_size)?;

        self.buffer.extend_from_slice(bytes);
        self.position += bytes.len();
        Ok(())
    }

    /// Write a string as length-prefixed UTF-8 bytes
    pub fn write_string(&mut self, s: &str) -> BytecodeResult<()> {
        let bytes = s.as_bytes();
        self.write_u32(bytes.len() as u32)?;
        self.write_bytes(bytes)
    }

    /// Write padding bytes to align to a specific boundary
    pub fn write_padding(&mut self, alignment: usize) -> BytecodeResult<()> {
        let current_pos = self.position;
        let padding_needed = (alignment - (current_pos % alignment)) % alignment;

        if padding_needed > 0 {
            let padding = vec![0u8; padding_needed];
            self.write_bytes(&padding)?;
        }

        Ok(())
    }

    /// Write data at a specific offset (for patching)
    pub fn write_at_offset(&mut self, offset: usize, bytes: &[u8]) -> BytecodeResult<()> {
        if offset + bytes.len() > self.buffer.len() {
            return Err(BytecodeGenerationError::SerializationError(format!("Write at offset {} would exceed buffer size", offset)));
        }

        self.buffer[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// Get the current offset for later patching
    pub fn create_patch_point(&self) -> PatchPoint {
        PatchPoint { offset: self.position }
    }

    /// Apply a patch at a previously created patch point
    pub fn apply_patch(&mut self, patch: PatchPoint, value: u32) -> BytecodeResult<()> {
        let bytes = value.to_le_bytes();
        self.write_at_offset(patch.offset, &bytes)
    }

    /// Check if the new size would exceed the limit
    fn check_size_limit(&self, new_size: usize) -> BytecodeResult<()> {
        if let Some(max_size) = self.max_size {
            if new_size > max_size {
                return Err(BytecodeGenerationError::BytecodeSizeLimitExceeded { actual: new_size, limit: max_size });
            }
        }
        Ok(())
    }
}

impl Default for BytecodeWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// A point in the bytecode that can be patched later
#[derive(Debug, Clone, Copy)]
pub struct PatchPoint {
    offset: usize,
}

impl PatchPoint {
    /// Get the offset of this patch point
    pub fn offset(&self) -> usize {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_writing() {
        let mut writer = BytecodeWriter::new();

        writer.write_u8(0x42).unwrap();
        writer.write_u16(0x1234).unwrap();
        writer.write_u32(0x12345678).unwrap();
        writer.write_u64(0x123456789ABCDEF0).unwrap();

        let buffer = writer.buffer();
        assert_eq!(buffer[0], 0x42);
        assert_eq!(&buffer[1..3], &[0x34, 0x12]); // Little-endian u16
        assert_eq!(&buffer[3..7], &[0x78, 0x56, 0x34, 0x12]); // Little-endian u32
        assert_eq!(writer.position(), 1 + 2 + 4 + 8);
    }

    #[test]
    fn test_string_writing() {
        let mut writer = BytecodeWriter::new();
        writer.write_string("hello").unwrap();

        let buffer = writer.buffer();
        assert_eq!(&buffer[0..4], &[5, 0, 0, 0]); // Length as u32
        assert_eq!(&buffer[4..9], b"hello");
    }

    #[test]
    fn test_size_limit() {
        let mut writer = BytecodeWriter::with_max_size(10);

        // Should succeed
        writer.write_u64(0x1234567890ABCDEF).unwrap();

        // Should fail - would exceed limit
        assert!(writer.write_u32(0x12345678).is_err());
    }

    #[test]
    fn test_patching() {
        let mut writer = BytecodeWriter::new();

        writer.write_u32(0).unwrap(); // Placeholder
        let patch_point = writer.create_patch_point();
        writer.write_u32(0).unwrap(); // Another placeholder

        // Patch the second value
        writer.apply_patch(patch_point, 0x12345678).unwrap();

        let buffer = writer.buffer();
        assert_eq!(&buffer[4..8], &[0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_padding() {
        let mut writer = BytecodeWriter::new();

        writer.write_u8(0x42).unwrap(); // Position 1
        writer.write_padding(4).unwrap(); // Align to 4-byte boundary

        assert_eq!(writer.position(), 4);
        let buffer = writer.buffer();
        assert_eq!(buffer[0], 0x42);
        assert_eq!(&buffer[1..4], &[0, 0, 0]); // Padding
    }
}
