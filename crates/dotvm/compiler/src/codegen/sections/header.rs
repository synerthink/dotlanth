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

//! Header section generator

use crate::codegen::{error::BytecodeResult, writer::BytecodeWriter};
use dotvm_core::bytecode::BytecodeHeader;

/// Generator for the bytecode header section
pub struct HeaderGenerator;

impl HeaderGenerator {
    /// Generate the header section
    pub fn generate(writer: &mut BytecodeWriter, header: &BytecodeHeader) -> BytecodeResult<()> {
        // Write magic number
        writer.write_bytes(&header.magic)?;

        // Write version
        writer.write_u8(header.version)?;

        // Write architecture
        writer.write_u8(header.architecture as u8)?;

        // Write reserved fields
        writer.write_bytes(&header.reserved)?;

        Ok(())
    }

    /// Calculate the size of the header section
    pub fn calculate_size() -> usize {
        BytecodeHeader::size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotvm_core::bytecode::VmArchitecture;

    #[test]
    fn test_header_generation() {
        let mut writer = BytecodeWriter::new();
        let header = BytecodeHeader::new(VmArchitecture::Arch64);

        HeaderGenerator::generate(&mut writer, &header).unwrap();

        assert_eq!(writer.size(), HeaderGenerator::calculate_size());
        assert_eq!(writer.size(), BytecodeHeader::size());
    }
}
