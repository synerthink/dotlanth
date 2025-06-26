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

/// Enum representing the supported VM architectures.
/// These values will be part of the bytecode header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmArchitecture {
    Arch32 = 0,
    Arch64 = 1,
    Arch128 = 2,
    Arch256 = 3,
    Arch512 = 4,
}

impl VmArchitecture {
    /// Create a VmArchitecture from a u8 value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(VmArchitecture::Arch32),
            1 => Some(VmArchitecture::Arch64),
            2 => Some(VmArchitecture::Arch128),
            3 => Some(VmArchitecture::Arch256),
            4 => Some(VmArchitecture::Arch512),
            _ => None,
        }
    }

    /// Get the word size in bytes for this architecture.
    pub fn word_size(&self) -> usize {
        match self {
            VmArchitecture::Arch32 => 4,
            VmArchitecture::Arch64 => 8,
            VmArchitecture::Arch128 => 16,
            VmArchitecture::Arch256 => 32,
            VmArchitecture::Arch512 => 64,
        }
    }
}

/// Represents the header of the DotVM bytecode.
/// It includes a magic number for identification and the target architecture.
#[derive(Debug, Clone, Copy, PartialEq)] // Added PartialEq
pub struct BytecodeHeader {
    /// Magic number to identify DotVM bytecode. Expected to be "DOTVM".
    pub magic: [u8; 5],
    /// Version of the bytecode format.
    pub version: u8,
    /// Target VM architecture for this bytecode.
    pub architecture: VmArchitecture,
    /// Reserved bytes for future use.
    pub reserved: [u8; 2], // Added 2 reserved bytes to make the header 9 bytes total for now
}

impl BytecodeHeader {
    pub const MAGIC_NUMBER: [u8; 5] = [b'D', b'O', b'T', b'V', b'M'];
    pub const CURRENT_VERSION: u8 = 1;

    /// Create a new BytecodeHeader.
    pub fn new(architecture: VmArchitecture) -> Self {
        BytecodeHeader {
            magic: Self::MAGIC_NUMBER,
            version: Self::CURRENT_VERSION,
            architecture,
            reserved: [0; 2],
        }
    }

    /// Serialize the header into a byte array.
    pub fn to_bytes(&self) -> [u8; 9] {
        let mut bytes = [0u8; 9];
        bytes[0..5].copy_from_slice(&self.magic);
        bytes[5] = self.version;
        bytes[6] = self.architecture as u8;
        bytes[7..9].copy_from_slice(&self.reserved);
        bytes
    }

    /// Deserialize a header from a byte array.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 9 {
            return Err("Insufficient bytes to form a header");
        }
        if bytes[0..5] != Self::MAGIC_NUMBER {
            return Err("Invalid magic number");
        }
        // We can add version checks here if needed in the future
        // if bytes[5] > Self::CURRENT_VERSION {
        //     return Err("Unsupported bytecode version");
        // }
        let architecture = VmArchitecture::from_u8(bytes[6]).ok_or("Invalid architecture byte")?;

        let mut reserved_bytes = [0u8; 2];
        reserved_bytes.copy_from_slice(&bytes[7..9]);

        Ok(BytecodeHeader {
            magic: Self::MAGIC_NUMBER, // Already checked
            version: bytes[5],
            architecture,
            reserved: reserved_bytes,
        })
    }

    /// Returns the size of the serialized header in bytes.
    pub const fn size() -> usize {
        9 // 5 (magic) + 1 (version) + 1 (architecture) + 2 (reserved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_architecture_from_u8() {
        assert_eq!(VmArchitecture::from_u8(0), Some(VmArchitecture::Arch32));
        assert_eq!(VmArchitecture::from_u8(1), Some(VmArchitecture::Arch64));
        assert_eq!(VmArchitecture::from_u8(2), Some(VmArchitecture::Arch128));
        assert_eq!(VmArchitecture::from_u8(3), Some(VmArchitecture::Arch256));
        assert_eq!(VmArchitecture::from_u8(4), Some(VmArchitecture::Arch512));
        assert_eq!(VmArchitecture::from_u8(5), None);
    }

    #[test]
    fn test_vm_architecture_word_size() {
        assert_eq!(VmArchitecture::Arch32.word_size(), 4);
        assert_eq!(VmArchitecture::Arch64.word_size(), 8);
        assert_eq!(VmArchitecture::Arch128.word_size(), 16);
        assert_eq!(VmArchitecture::Arch256.word_size(), 32);
        assert_eq!(VmArchitecture::Arch512.word_size(), 64);
    }

    #[test]
    fn test_bytecode_header_new() {
        let header = BytecodeHeader::new(VmArchitecture::Arch64);
        assert_eq!(header.magic, BytecodeHeader::MAGIC_NUMBER);
        assert_eq!(header.version, BytecodeHeader::CURRENT_VERSION);
        assert_eq!(header.architecture, VmArchitecture::Arch64);
        assert_eq!(header.reserved, [0; 2]);
    }

    #[test]
    fn test_bytecode_header_to_bytes() {
        let header = BytecodeHeader::new(VmArchitecture::Arch128);
        let bytes = header.to_bytes();
        assert_eq!(bytes[0..5], BytecodeHeader::MAGIC_NUMBER);
        assert_eq!(bytes[5], BytecodeHeader::CURRENT_VERSION);
        assert_eq!(bytes[6], VmArchitecture::Arch128 as u8);
        assert_eq!(bytes[7..9], [0; 2]);
        assert_eq!(bytes.len(), BytecodeHeader::size());
    }

    #[test]
    fn test_bytecode_header_from_bytes_valid() {
        let header_orig = BytecodeHeader::new(VmArchitecture::Arch256);
        let bytes = header_orig.to_bytes();
        let header_deserialized = BytecodeHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header_deserialized.magic, header_orig.magic);
        assert_eq!(header_deserialized.version, header_orig.version);
        assert_eq!(header_deserialized.architecture, header_orig.architecture);
        assert_eq!(header_deserialized.reserved, header_orig.reserved);
    }

    #[test]
    fn test_bytecode_header_from_bytes_invalid_magic() {
        let mut bytes = BytecodeHeader::new(VmArchitecture::Arch64).to_bytes();
        bytes[0] = b'X'; // Corrupt magic number
        let result = BytecodeHeader::from_bytes(&bytes);
        assert_eq!(result, Err("Invalid magic number"));
    }

    #[test]
    fn test_bytecode_header_from_bytes_invalid_arch() {
        let mut bytes = BytecodeHeader::new(VmArchitecture::Arch64).to_bytes();
        bytes[6] = 10; // Invalid architecture byte
        let result = BytecodeHeader::from_bytes(&bytes);
        assert_eq!(result, Err("Invalid architecture byte"));
    }

    #[test]
    fn test_bytecode_header_from_bytes_insufficient_data() {
        let bytes = &BytecodeHeader::new(VmArchitecture::Arch64).to_bytes()[0..7];
        let result = BytecodeHeader::from_bytes(bytes);
        assert_eq!(result, Err("Insufficient bytes to form a header"));
    }

    #[test]
    fn test_bytecode_header_size_constant() {
        // Ensure the constant matches the actual serialized size
        let header = BytecodeHeader::new(VmArchitecture::Arch64);
        assert_eq!(BytecodeHeader::size(), header.to_bytes().len());
    }
}
