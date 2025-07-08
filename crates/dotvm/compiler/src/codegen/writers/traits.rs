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
