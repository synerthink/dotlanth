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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CryptoOpcode {
    Hash = 0x40,
    Encrypt = 0x41,
    Decrypt = 0x42,
    Sign = 0x43,
    VerifySignature = 0x44,
}

impl CryptoOpcode {
    /// Converts a mnemonic to a `CryptoOpcode`.
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            "CRYPTO_HASH" => Some(CryptoOpcode::Hash),
            "CRYPTO_ENCRYPT" => Some(CryptoOpcode::Encrypt),
            "CRYPTO_DECRYPT" => Some(CryptoOpcode::Decrypt),
            "CRYPTO_SIGN" => Some(CryptoOpcode::Sign),
            "CRYPTO_VERIFY_SIGNATURE" => Some(CryptoOpcode::VerifySignature),
            _ => None,
        }
    }

    /// Converts a `CryptoOpcode` to its mnemonic.
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            CryptoOpcode::Hash => "CRYPTO_HASH",
            CryptoOpcode::Encrypt => "CRYPTO_ENCRYPT",
            CryptoOpcode::Decrypt => "CRYPTO_DECRYPT",
            CryptoOpcode::Sign => "CRYPTO_SIGN",
            CryptoOpcode::VerifySignature => "CRYPTO_VERIFY_SIGNATURE",
        }
    }

    /// Returns the opcode's numerical value.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Converts a numerical value back to a CryptoOpcode.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x40 => Some(Self::Hash),
            0x41 => Some(Self::Encrypt),
            0x42 => Some(Self::Decrypt),
            0x43 => Some(Self::Sign),
            0x44 => Some(Self::VerifySignature),
            _ => None,
        }
    }
}

impl std::fmt::Display for CryptoOpcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_mnemonic())
    }
}
