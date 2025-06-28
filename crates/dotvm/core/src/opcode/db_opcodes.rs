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

//! Database Operation Opcodes
//!
//! This module defines opcodes for database operations like DB_GET and DB_PUT.

use std::fmt;

/// Database operation opcodes
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DatabaseOpcode {
    /// Get a document from the database
    /// Stack: [collection_name, document_id] -> [document_json]
    DbGet = 0x30,

    /// Put a document to the database
    /// Stack: [collection_name, document_json] -> [document_id]
    DbPut = 0x31,

    /// Update a document in the database
    /// Stack: [collection_name, document_id, document_json] -> []
    DbUpdate = 0x32,

    /// Delete a document from the database
    /// Stack: [collection_name, document_id] -> []
    DbDelete = 0x33,

    /// List documents in a collection
    /// Stack: [collection_name] -> [document_ids_array]
    DbList = 0x34,

    /// Create a collection
    /// Stack: [collection_name] -> []
    DbCreateCollection = 0x35,

    /// Delete a collection
    /// Stack: [collection_name] -> []
    DbDeleteCollection = 0x36,
}

impl DatabaseOpcode {
    /// Converts a mnemonic string to a DatabaseOpcode
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            "DB_GET" | "DBGET" => Some(Self::DbGet),
            "DB_PUT" | "DBPUT" => Some(Self::DbPut),
            "DB_UPDATE" | "DBUPDATE" => Some(Self::DbUpdate),
            "DB_DELETE" | "DBDELETE" => Some(Self::DbDelete),
            "DB_LIST" | "DBLIST" => Some(Self::DbList),
            "DB_CREATE_COLLECTION" | "DBCREATECOLLECTION" => Some(Self::DbCreateCollection),
            "DB_DELETE_COLLECTION" | "DBDELETECOLLECTION" => Some(Self::DbDeleteCollection),
            _ => None,
        }
    }

    /// Converts a DatabaseOpcode to its mnemonic string
    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            DatabaseOpcode::DbGet => "DB_GET",
            DatabaseOpcode::DbPut => "DB_PUT",
            DatabaseOpcode::DbUpdate => "DB_UPDATE",
            DatabaseOpcode::DbDelete => "DB_DELETE",
            DatabaseOpcode::DbList => "DB_LIST",
            DatabaseOpcode::DbCreateCollection => "DB_CREATE_COLLECTION",
            DatabaseOpcode::DbDeleteCollection => "DB_DELETE_COLLECTION",
        }
    }

    /// Returns the opcode's numerical value
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Converts a numerical value back to a DatabaseOpcode
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x30 => Some(Self::DbGet),
            0x31 => Some(Self::DbPut),
            0x32 => Some(Self::DbUpdate),
            0x33 => Some(Self::DbDelete),
            0x34 => Some(Self::DbList),
            0x35 => Some(Self::DbCreateCollection),
            0x36 => Some(Self::DbDeleteCollection),
            _ => None,
        }
    }

    /// Returns the number of operand bytes this opcode expects
    pub fn operand_size(&self) -> usize {
        // Database opcodes don't have immediate operands - they work with stack values
        0
    }
}

impl fmt::Display for DatabaseOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_mnemonic())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_mnemonic() {
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_GET"), Some(DatabaseOpcode::DbGet));
        assert_eq!(DatabaseOpcode::from_mnemonic("dbput"), Some(DatabaseOpcode::DbPut));
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_UPDATE"), Some(DatabaseOpcode::DbUpdate));
        assert_eq!(DatabaseOpcode::from_mnemonic("unknown"), None);
    }

    #[test]
    fn test_to_mnemonic() {
        assert_eq!(DatabaseOpcode::DbGet.to_mnemonic(), "DB_GET");
        assert_eq!(DatabaseOpcode::DbPut.to_mnemonic(), "DB_PUT");
        assert_eq!(DatabaseOpcode::DbUpdate.to_mnemonic(), "DB_UPDATE");
    }

    #[test]
    fn test_opcode_values() {
        assert_eq!(DatabaseOpcode::DbGet as u8, 0x30);
        assert_eq!(DatabaseOpcode::DbPut as u8, 0x31);
        assert_eq!(DatabaseOpcode::DbUpdate as u8, 0x32);
        assert_eq!(DatabaseOpcode::DbDelete as u8, 0x33);
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(DatabaseOpcode::from_u8(0x30), Some(DatabaseOpcode::DbGet));
        assert_eq!(DatabaseOpcode::from_u8(0x31), Some(DatabaseOpcode::DbPut));
        assert_eq!(DatabaseOpcode::from_u8(0xFF), None);
    }

    #[test]
    fn test_display() {
        assert_eq!(DatabaseOpcode::DbGet.to_string(), "DB_GET");
        assert_eq!(DatabaseOpcode::DbPut.to_string(), "DB_PUT");
    }
}
