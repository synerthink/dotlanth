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

//! Variable and memory layout type definitions for transpilation

/// Local variable information
#[derive(Debug, Clone)]
pub struct LocalVariable {
    /// Variable index
    pub index: u32,
    /// Variable type
    pub var_type: VariableType,
    /// Whether the variable is a parameter
    pub is_parameter: bool,
    /// Variable name (if available)
    pub name: Option<String>,
    /// Scope information
    pub scope: VariableScope,
}

impl LocalVariable {
    /// Create a new local variable
    pub fn new(index: u32, var_type: VariableType, is_parameter: bool) -> Self {
        Self {
            index,
            var_type,
            is_parameter,
            name: None,
            scope: VariableScope::Function,
        }
    }

    /// Create a parameter variable
    pub fn parameter(index: u32, var_type: VariableType) -> Self {
        Self::new(index, var_type, true)
    }

    /// Create a local variable
    pub fn local(index: u32, var_type: VariableType) -> Self {
        Self::new(index, var_type, false)
    }

    /// Set the variable name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the variable scope
    pub fn with_scope(mut self, scope: VariableScope) -> Self {
        self.scope = scope;
        self
    }

    /// Get the size of this variable in bytes
    pub fn size_bytes(&self) -> u32 {
        self.var_type.size_bytes()
    }
}

/// Variable types in DotVM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableType {
    I32,
    I64,
    F32,
    F64,
    V128,
    Pointer,
    /// Custom type with size in bytes
    Custom(u32),
}

impl VariableType {
    /// Get the size of this type in bytes
    pub fn size_bytes(&self) -> u32 {
        match self {
            Self::I32 => 4,
            Self::I64 => 8,
            Self::F32 => 4,
            Self::F64 => 8,
            Self::V128 => 16,
            Self::Pointer => 8, // Assuming 64-bit pointers
            Self::Custom(size) => *size,
        }
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::I32 | Self::I64)
    }

    /// Check if this is a floating-point type
    pub fn is_float(&self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    /// Check if this is a vector type
    pub fn is_vector(&self) -> bool {
        matches!(self, Self::V128)
    }

    /// Check if this is a pointer type
    pub fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer)
    }

    /// Get the alignment requirement for this type
    pub fn alignment(&self) -> u32 {
        match self {
            Self::I32 | Self::F32 => 4,
            Self::I64 | Self::F64 | Self::Pointer => 8,
            Self::V128 => 16,
            Self::Custom(size) => (*size).next_power_of_two().min(16),
        }
    }
}

/// Variable scope information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableScope {
    /// Function-level scope
    Function,
    /// Block-level scope
    Block(u32),
    /// Loop-level scope
    Loop(u32),
    /// Global scope
    Global,
}

/// Global variable information
#[derive(Debug, Clone)]
pub struct GlobalVariable {
    /// Global variable index
    pub index: u32,
    /// Variable type
    pub var_type: VariableType,
    /// Whether the variable is mutable
    pub is_mutable: bool,
    /// Initial value
    pub initial_value: Option<u64>,
    /// Variable name (if available)
    pub name: Option<String>,
    /// Whether this global is exported
    pub is_exported: bool,
}

impl GlobalVariable {
    /// Create a new global variable
    pub fn new(index: u32, var_type: VariableType, is_mutable: bool) -> Self {
        Self {
            index,
            var_type,
            is_mutable,
            initial_value: None,
            name: None,
            is_exported: false,
        }
    }

    /// Set the initial value
    pub fn with_initial_value(mut self, value: u64) -> Self {
        self.initial_value = Some(value);
        self
    }

    /// Set the variable name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Mark as exported
    pub fn mark_exported(mut self) -> Self {
        self.is_exported = true;
        self
    }

    /// Get the size of this global in bytes
    pub fn size_bytes(&self) -> u32 {
        self.var_type.size_bytes()
    }
}

/// Memory layout information
#[derive(Debug, Clone)]
pub struct MemoryLayout {
    /// Initial number of pages
    pub initial_pages: u32,
    /// Maximum number of pages (if limited)
    pub maximum_pages: Option<u32>,
    /// Page size in bytes
    pub page_size: u32,
    /// Memory segments
    pub segments: Vec<MemorySegment>,
    /// Memory protection flags
    pub protection: MemoryProtection,
}

impl Default for MemoryLayout {
    fn default() -> Self {
        Self {
            initial_pages: 1,
            maximum_pages: None,
            page_size: 65536, // 64KB pages (WASM standard)
            segments: Vec::new(),
            protection: MemoryProtection::default(),
        }
    }
}

impl MemoryLayout {
    /// Create a new memory layout
    pub fn new(initial_pages: u32, page_size: u32) -> Self {
        Self {
            initial_pages,
            maximum_pages: None,
            page_size,
            segments: Vec::new(),
            protection: MemoryProtection::default(),
        }
    }

    /// Set the maximum number of pages
    pub fn with_max_pages(mut self, max_pages: u32) -> Self {
        self.maximum_pages = Some(max_pages);
        self
    }

    /// Add a memory segment
    pub fn add_segment(&mut self, segment: MemorySegment) {
        self.segments.push(segment);
    }

    /// Get the total initial memory size in bytes
    pub fn initial_size_bytes(&self) -> u64 {
        self.initial_pages as u64 * self.page_size as u64
    }

    /// Get the maximum memory size in bytes
    pub fn max_size_bytes(&self) -> Option<u64> {
        self.maximum_pages.map(|pages| pages as u64 * self.page_size as u64)
    }

    /// Check if the layout can grow
    pub fn can_grow(&self) -> bool {
        self.maximum_pages.map_or(true, |max| max > self.initial_pages)
    }
}

/// Memory segment information
#[derive(Debug, Clone)]
pub struct MemorySegment {
    /// Segment offset in memory
    pub offset: u64,
    /// Segment size in bytes
    pub size: u64,
    /// Segment data (if available)
    pub data: Option<Vec<u8>>,
    /// Segment type
    pub segment_type: SegmentType,
    /// Whether this segment is initialized
    pub is_initialized: bool,
}

impl MemorySegment {
    /// Create a new memory segment
    pub fn new(offset: u64, size: u64, segment_type: SegmentType) -> Self {
        Self {
            offset,
            size,
            data: None,
            segment_type,
            is_initialized: false,
        }
    }

    /// Set the segment data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self.is_initialized = true;
        self
    }

    /// Mark as initialized
    pub fn mark_initialized(mut self) -> Self {
        self.is_initialized = true;
        self
    }
}

/// Memory segment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentType {
    /// Data segment with static data
    Data,
    /// Code segment (read-only)
    Code,
    /// Stack segment
    Stack,
    /// Heap segment
    Heap,
    /// Custom segment
    Custom,
}

/// Memory protection flags
#[derive(Debug, Clone, Default)]
pub struct MemoryProtection {
    /// Whether memory is readable
    pub readable: bool,
    /// Whether memory is writable
    pub writable: bool,
    /// Whether memory is executable
    pub executable: bool,
    /// Whether memory can grow
    pub growable: bool,
}

impl MemoryProtection {
    /// Create read-write memory protection
    pub fn read_write() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            growable: true,
        }
    }

    /// Create read-only memory protection
    pub fn read_only() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: false,
            growable: false,
        }
    }

    /// Create executable memory protection
    pub fn executable() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: true,
            growable: false,
        }
    }
}
