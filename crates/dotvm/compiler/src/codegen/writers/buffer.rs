// Dotlanth
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
