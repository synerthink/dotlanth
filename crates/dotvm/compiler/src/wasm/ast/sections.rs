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

//! WebAssembly section definitions and utilities

use serde::{Deserialize, Serialize};

/// WebAssembly section types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WasmSectionType {
    Custom = 0,
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
    DataCount = 12,
}

impl WasmSectionType {
    /// Get the section name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Type => "type",
            Self::Import => "import",
            Self::Function => "function",
            Self::Table => "table",
            Self::Memory => "memory",
            Self::Global => "global",
            Self::Export => "export",
            Self::Start => "start",
            Self::Element => "element",
            Self::Code => "code",
            Self::Data => "data",
            Self::DataCount => "datacount",
        }
    }

    /// Check if this section is required
    pub fn is_required(&self) -> bool {
        match self {
            Self::Custom => false,
            Self::Type => false,
            Self::Import => false,
            Self::Function => false,
            Self::Table => false,
            Self::Memory => false,
            Self::Global => false,
            Self::Export => false,
            Self::Start => false,
            Self::Element => false,
            Self::Code => false,
            Self::Data => false,
            Self::DataCount => false,
        }
    }

    /// Check if this section can appear multiple times
    pub fn can_repeat(&self) -> bool {
        matches!(self, Self::Custom)
    }

    /// Get the expected order of this section
    pub fn order(&self) -> u8 {
        *self as u8
    }

    /// Convert from section ID
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Custom),
            1 => Some(Self::Type),
            2 => Some(Self::Import),
            3 => Some(Self::Function),
            4 => Some(Self::Table),
            5 => Some(Self::Memory),
            6 => Some(Self::Global),
            7 => Some(Self::Export),
            8 => Some(Self::Start),
            9 => Some(Self::Element),
            10 => Some(Self::Code),
            11 => Some(Self::Data),
            12 => Some(Self::DataCount),
            _ => None,
        }
    }

    /// Check if this section should come before another section
    pub fn comes_before(&self, other: &Self) -> bool {
        if *self == Self::Custom || *other == Self::Custom {
            // Custom sections can appear anywhere
            true
        } else {
            self.order() < other.order()
        }
    }
}

impl std::fmt::Display for WasmSectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Section ordering validator
#[derive(Debug, Default)]
pub struct SectionOrderValidator {
    /// Last seen non-custom section
    last_section: Option<WasmSectionType>,
    /// Sections that have been seen
    seen_sections: std::collections::HashSet<WasmSectionType>,
}

impl SectionOrderValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate that a section appears in the correct order
    pub fn validate_section(&mut self, section_type: WasmSectionType) -> Result<(), String> {
        // Custom sections can appear anywhere
        if section_type == WasmSectionType::Custom {
            return Ok(());
        }

        // Check if section can repeat
        if !section_type.can_repeat() && self.seen_sections.contains(&section_type) {
            return Err(format!("Section {} appears multiple times", section_type));
        }

        // Check ordering
        if let Some(last) = self.last_section {
            if !last.comes_before(&section_type) && last != section_type {
                return Err(format!("Section {} appears after {} but should come before", section_type, last));
            }
        }

        self.last_section = Some(section_type);
        self.seen_sections.insert(section_type);
        Ok(())
    }

    /// Check if all required sections are present
    pub fn validate_completeness(&self) -> Result<(), String> {
        // Currently no sections are strictly required for a valid module
        // but we could add validation here if needed
        Ok(())
    }

    /// Reset the validator
    pub fn reset(&mut self) {
        self.last_section = None;
        self.seen_sections.clear();
    }
}

/// Section size limits for validation
#[derive(Debug, Clone)]
pub struct SectionLimits {
    /// Maximum number of types
    pub max_types: Option<usize>,
    /// Maximum number of imports
    pub max_imports: Option<usize>,
    /// Maximum number of functions
    pub max_functions: Option<usize>,
    /// Maximum number of tables
    pub max_tables: Option<usize>,
    /// Maximum number of memories
    pub max_memories: Option<usize>,
    /// Maximum number of globals
    pub max_globals: Option<usize>,
    /// Maximum number of exports
    pub max_exports: Option<usize>,
    /// Maximum number of elements
    pub max_elements: Option<usize>,
    /// Maximum number of data segments
    pub max_data_segments: Option<usize>,
    /// Maximum size of a single section in bytes
    pub max_section_size: Option<usize>,
}

impl Default for SectionLimits {
    fn default() -> Self {
        Self {
            max_types: Some(10000),
            max_imports: Some(10000),
            max_functions: Some(10000),
            max_tables: Some(100),
            max_memories: Some(1), // WASM spec currently allows only 1
            max_globals: Some(10000),
            max_exports: Some(10000),
            max_elements: Some(10000),
            max_data_segments: Some(10000),
            max_section_size: Some(64 * 1024 * 1024), // 64MB
        }
    }
}

impl SectionLimits {
    /// Create limits with no restrictions
    pub fn unlimited() -> Self {
        Self {
            max_types: None,
            max_imports: None,
            max_functions: None,
            max_tables: None,
            max_memories: None,
            max_globals: None,
            max_exports: None,
            max_elements: None,
            max_data_segments: None,
            max_section_size: None,
        }
    }

    /// Create strict limits for security
    pub fn strict() -> Self {
        Self {
            max_types: Some(1000),
            max_imports: Some(100),
            max_functions: Some(1000),
            max_tables: Some(10),
            max_memories: Some(1),
            max_globals: Some(100),
            max_exports: Some(100),
            max_elements: Some(100),
            max_data_segments: Some(100),
            max_section_size: Some(1024 * 1024), // 1MB
        }
    }

    /// Validate section count
    pub fn validate_count(&self, section_type: WasmSectionType, count: usize) -> Result<(), super::super::error::WasmError> {
        let limit = match section_type {
            WasmSectionType::Type => self.max_types,
            WasmSectionType::Import => self.max_imports,
            WasmSectionType::Function => self.max_functions,
            WasmSectionType::Table => self.max_tables,
            WasmSectionType::Memory => self.max_memories,
            WasmSectionType::Global => self.max_globals,
            WasmSectionType::Export => self.max_exports,
            WasmSectionType::Element => self.max_elements,
            WasmSectionType::Data => self.max_data_segments,
            _ => return Ok(()), // No limits for other sections
        };

        if let Some(max) = limit {
            if count > max {
                return Err(super::super::error::WasmError::validation_failed(format!(
                    "Section {} has {} items but maximum is {}",
                    section_type, count, max
                )));
            }
        }

        Ok(())
    }

    /// Validate section size
    pub fn validate_size(&self, section_type: WasmSectionType, size: usize) -> Result<(), super::super::error::WasmError> {
        if let Some(max_size) = self.max_section_size {
            if size > max_size {
                return Err(super::super::error::WasmError::validation_failed(format!(
                    "Section {} is {} bytes but maximum is {} bytes",
                    section_type, size, max_size
                )));
            }
        }

        Ok(())
    }
}

/// Section metadata for tracking
#[derive(Debug, Clone)]
pub struct SectionMetadata {
    /// Section type
    pub section_type: WasmSectionType,
    /// Section size in bytes
    pub size: usize,
    /// Number of items in the section
    pub item_count: usize,
    /// Offset in the binary
    pub offset: usize,
}

impl SectionMetadata {
    /// Create new section metadata
    pub fn new(section_type: WasmSectionType, size: usize, item_count: usize, offset: usize) -> Self {
        Self {
            section_type,
            size,
            item_count,
            offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_types() {
        assert_eq!(WasmSectionType::Type.name(), "type");
        assert_eq!(WasmSectionType::Code.name(), "code");
        assert_eq!(WasmSectionType::Custom.order(), 0);
        assert_eq!(WasmSectionType::Code.order(), 10);

        assert!(WasmSectionType::Custom.can_repeat());
        assert!(!WasmSectionType::Type.can_repeat());
    }

    #[test]
    fn test_section_ordering() {
        assert!(WasmSectionType::Type.comes_before(&WasmSectionType::Function));
        assert!(!WasmSectionType::Code.comes_before(&WasmSectionType::Type));
        assert!(WasmSectionType::Custom.comes_before(&WasmSectionType::Type));
    }

    #[test]
    fn test_section_order_validator() {
        let mut validator = SectionOrderValidator::new();

        // Valid order
        assert!(validator.validate_section(WasmSectionType::Type).is_ok());
        assert!(validator.validate_section(WasmSectionType::Function).is_ok());
        assert!(validator.validate_section(WasmSectionType::Code).is_ok());

        // Invalid order
        validator.reset();
        assert!(validator.validate_section(WasmSectionType::Code).is_ok());
        assert!(validator.validate_section(WasmSectionType::Type).is_err());
    }

    #[test]
    fn test_section_limits() {
        let limits = SectionLimits::strict();

        // Valid count
        assert!(limits.validate_count(WasmSectionType::Function, 500).is_ok());

        // Invalid count
        assert!(limits.validate_count(WasmSectionType::Function, 2000).is_err());

        // Valid size
        assert!(limits.validate_size(WasmSectionType::Code, 500000).is_ok());

        // Invalid size
        assert!(limits.validate_size(WasmSectionType::Code, 2000000).is_err());
    }

    #[test]
    fn test_section_from_id() {
        assert_eq!(WasmSectionType::from_id(1), Some(WasmSectionType::Type));
        assert_eq!(WasmSectionType::from_id(10), Some(WasmSectionType::Code));
        assert_eq!(WasmSectionType::from_id(255), None);
    }
}
