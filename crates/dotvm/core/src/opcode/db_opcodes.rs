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
//! This module defines opcodes for direct DotDB integration providing
//! key-value operations, complex queries, transactions, indexing, and streaming.

use std::fmt;

/// Database operation opcodes for direct DotDB integration
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DatabaseOpcode {
    /// Read a key-value pair from the database
    /// Stack: [table_id, key] -> [value] or [null]
    DbRead = 0x30,

    /// Write a key-value pair to the database
    /// Stack: [table_id, key, value] -> []
    DbWrite = 0x31,

    /// Execute a complex query
    /// Stack: [query_spec_json] -> [query_result_json]
    DbQuery = 0x32,

    /// Execute an atomic transaction
    /// Stack: [transaction_ops_json] -> [transaction_result_json]
    DbTransaction = 0x33,

    /// Manage database indexes
    /// Stack: [index_operation_json] -> []
    DbIndex = 0x34,

    /// Create a stream for large result sets
    /// Stack: [stream_spec_json] -> [stream_result_json]
    DbStream = 0x35,

    // Legacy opcodes for backward compatibility
    /// Get a document from the database (legacy)
    /// Stack: [collection_name, document_id] -> [document_json]
    DbGet = 0x40,

    /// Put a document to the database (legacy)
    /// Stack: [collection_name, document_json] -> [document_id]
    DbPut = 0x41,

    /// Update a document in the database (legacy)
    /// Stack: [collection_name, document_id, document_json] -> []
    DbUpdate = 0x42,

    /// Delete a document from the database (legacy)
    /// Stack: [collection_name, document_id] -> []
    DbDelete = 0x43,

    /// List documents in a collection (legacy)
    /// Stack: [collection_name] -> [document_ids_array]
    DbList = 0x44,

    /// Create a collection (legacy)
    /// Stack: [collection_name] -> []
    DbCreateCollection = 0x45,

    /// Delete a collection (legacy)
    /// Stack: [collection_name] -> []
    DbDeleteCollection = 0x46,
}

impl DatabaseOpcode {
    /// Converts a mnemonic string to a DatabaseOpcode
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic.to_uppercase().as_str() {
            // New key-value opcodes
            "DB_READ" | "DBREAD" => Some(Self::DbRead),
            "DB_WRITE" | "DBWRITE" => Some(Self::DbWrite),
            "DB_QUERY" | "DBQUERY" => Some(Self::DbQuery),
            "DB_TRANSACTION" | "DBTRANSACTION" => Some(Self::DbTransaction),
            "DB_INDEX" | "DBINDEX" => Some(Self::DbIndex),
            "DB_STREAM" | "DBSTREAM" => Some(Self::DbStream),
            // Legacy document opcodes
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
            // New key-value opcodes
            DatabaseOpcode::DbRead => "DB_READ",
            DatabaseOpcode::DbWrite => "DB_WRITE",
            DatabaseOpcode::DbQuery => "DB_QUERY",
            DatabaseOpcode::DbTransaction => "DB_TRANSACTION",
            DatabaseOpcode::DbIndex => "DB_INDEX",
            DatabaseOpcode::DbStream => "DB_STREAM",
            // Legacy document opcodes
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
            // New key-value opcodes
            0x30 => Some(Self::DbRead),
            0x31 => Some(Self::DbWrite),
            0x32 => Some(Self::DbQuery),
            0x33 => Some(Self::DbTransaction),
            0x34 => Some(Self::DbIndex),
            0x35 => Some(Self::DbStream),
            // Legacy document opcodes
            0x40 => Some(Self::DbGet),
            0x41 => Some(Self::DbPut),
            0x42 => Some(Self::DbUpdate),
            0x43 => Some(Self::DbDelete),
            0x44 => Some(Self::DbList),
            0x45 => Some(Self::DbCreateCollection),
            0x46 => Some(Self::DbDeleteCollection),
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
    fn test_from_mnemonic_new_opcodes() {
        // Test new key-value opcodes
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_READ"), Some(DatabaseOpcode::DbRead));
        assert_eq!(DatabaseOpcode::from_mnemonic("dbwrite"), Some(DatabaseOpcode::DbWrite));
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_QUERY"), Some(DatabaseOpcode::DbQuery));
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_TRANSACTION"), Some(DatabaseOpcode::DbTransaction));
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_INDEX"), Some(DatabaseOpcode::DbIndex));
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_STREAM"), Some(DatabaseOpcode::DbStream));
    }

    #[test]
    fn test_from_mnemonic_legacy_opcodes() {
        // Test legacy document opcodes
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_GET"), Some(DatabaseOpcode::DbGet));
        assert_eq!(DatabaseOpcode::from_mnemonic("dbput"), Some(DatabaseOpcode::DbPut));
        assert_eq!(DatabaseOpcode::from_mnemonic("DB_UPDATE"), Some(DatabaseOpcode::DbUpdate));
        assert_eq!(DatabaseOpcode::from_mnemonic("unknown"), None);
    }

    #[test]
    fn test_to_mnemonic_new_opcodes() {
        assert_eq!(DatabaseOpcode::DbRead.to_mnemonic(), "DB_READ");
        assert_eq!(DatabaseOpcode::DbWrite.to_mnemonic(), "DB_WRITE");
        assert_eq!(DatabaseOpcode::DbQuery.to_mnemonic(), "DB_QUERY");
        assert_eq!(DatabaseOpcode::DbTransaction.to_mnemonic(), "DB_TRANSACTION");
        assert_eq!(DatabaseOpcode::DbIndex.to_mnemonic(), "DB_INDEX");
        assert_eq!(DatabaseOpcode::DbStream.to_mnemonic(), "DB_STREAM");
    }

    #[test]
    fn test_to_mnemonic_legacy_opcodes() {
        assert_eq!(DatabaseOpcode::DbGet.to_mnemonic(), "DB_GET");
        assert_eq!(DatabaseOpcode::DbPut.to_mnemonic(), "DB_PUT");
        assert_eq!(DatabaseOpcode::DbUpdate.to_mnemonic(), "DB_UPDATE");
    }

    #[test]
    fn test_opcode_values_new() {
        // Test new opcode values
        assert_eq!(DatabaseOpcode::DbRead as u8, 0x30);
        assert_eq!(DatabaseOpcode::DbWrite as u8, 0x31);
        assert_eq!(DatabaseOpcode::DbQuery as u8, 0x32);
        assert_eq!(DatabaseOpcode::DbTransaction as u8, 0x33);
        assert_eq!(DatabaseOpcode::DbIndex as u8, 0x34);
        assert_eq!(DatabaseOpcode::DbStream as u8, 0x35);
    }

    #[test]
    fn test_opcode_values_legacy() {
        // Test legacy opcode values
        assert_eq!(DatabaseOpcode::DbGet as u8, 0x40);
        assert_eq!(DatabaseOpcode::DbPut as u8, 0x41);
        assert_eq!(DatabaseOpcode::DbUpdate as u8, 0x42);
        assert_eq!(DatabaseOpcode::DbDelete as u8, 0x43);
    }

    #[test]
    fn test_from_u8_new_opcodes() {
        assert_eq!(DatabaseOpcode::from_u8(0x30), Some(DatabaseOpcode::DbRead));
        assert_eq!(DatabaseOpcode::from_u8(0x31), Some(DatabaseOpcode::DbWrite));
        assert_eq!(DatabaseOpcode::from_u8(0x32), Some(DatabaseOpcode::DbQuery));
        assert_eq!(DatabaseOpcode::from_u8(0x33), Some(DatabaseOpcode::DbTransaction));
        assert_eq!(DatabaseOpcode::from_u8(0x34), Some(DatabaseOpcode::DbIndex));
        assert_eq!(DatabaseOpcode::from_u8(0x35), Some(DatabaseOpcode::DbStream));
    }

    #[test]
    fn test_from_u8_legacy_opcodes() {
        assert_eq!(DatabaseOpcode::from_u8(0x40), Some(DatabaseOpcode::DbGet));
        assert_eq!(DatabaseOpcode::from_u8(0x41), Some(DatabaseOpcode::DbPut));
        assert_eq!(DatabaseOpcode::from_u8(0xFF), None);
    }

    #[test]
    fn test_display() {
        assert_eq!(DatabaseOpcode::DbRead.to_string(), "DB_READ");
        assert_eq!(DatabaseOpcode::DbWrite.to_string(), "DB_WRITE");
        assert_eq!(DatabaseOpcode::DbGet.to_string(), "DB_GET");
        assert_eq!(DatabaseOpcode::DbPut.to_string(), "DB_PUT");
    }
}
