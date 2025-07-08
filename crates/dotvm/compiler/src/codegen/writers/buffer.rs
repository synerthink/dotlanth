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

//! Buffer management utilities for writers

/// Simple growable buffer wrapper
pub struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    /// Create an empty buffer
    pub fn new() -> Self {
        Buffer { data: Vec::new() }
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Access the raw data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Mutable access for patching
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}
