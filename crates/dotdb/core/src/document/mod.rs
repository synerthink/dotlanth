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

//! Document Storage Layer
//!
//! This module provides a document-oriented abstraction over the key-value
//! database interface. It supports JSON documents organized into collections
//! with UUID-based document identification.

pub mod collection;
pub mod storage;

pub use collection::*;
pub use storage::*;

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Document identifier using UUID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    /// Generate a new random document ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a document ID from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Create a document ID from a string representation
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// Get the UUID value
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for DocumentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<DocumentId> for Uuid {
    fn from(doc_id: DocumentId) -> Self {
        doc_id.0
    }
}

/// Collection name for organizing documents
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollectionName(pub String);

impl CollectionName {
    /// Create a new collection name
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CollectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CollectionName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<&str> for CollectionName {
    fn from(name: &str) -> Self {
        Self(name.to_string())
    }
}

/// JSON document value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Document ID
    pub id: DocumentId,
    /// JSON content
    pub content: serde_json::Value,
    /// Metadata
    pub metadata: DocumentMetadata,
}

impl Document {
    /// Create a new document with generated ID
    pub fn new(content: serde_json::Value) -> Self {
        Self {
            id: DocumentId::new(),
            content,
            metadata: DocumentMetadata::new(),
        }
    }

    /// Create a document with specific ID
    pub fn with_id(id: DocumentId, content: serde_json::Value) -> Self {
        Self {
            id,
            content,
            metadata: DocumentMetadata::new(),
        }
    }

    /// Get document as JSON string
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.content)
    }

    /// Create document from JSON string
    pub fn from_json_string(json: &str) -> Result<Self, serde_json::Error> {
        let content: serde_json::Value = serde_json::from_str(json)?;
        Ok(Self::new(content))
    }
}

/// Document metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Creation timestamp
    pub created_at: u64,
    /// Last modification timestamp
    pub updated_at: u64,
    /// Document version
    pub version: u64,
}

impl DocumentMetadata {
    /// Create new metadata with current timestamp
    pub fn new() -> Self {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        Self {
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    /// Update the modification timestamp and increment version
    pub fn update(&mut self) {
        self.updated_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        self.version += 1;
    }
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Document storage errors
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("Database error: {0}")]
    Database(#[from] crate::state::db_interface::DbError),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Document not found: {0}")]
    DocumentNotFound(DocumentId),

    #[error("Collection not found: {0}")]
    CollectionNotFound(CollectionName),

    #[error("Invalid document ID: {0}")]
    InvalidDocumentId(String),

    #[error("Invalid collection name: {0}")]
    InvalidCollectionName(String),

    #[error("Document already exists: {0}")]
    DocumentAlreadyExists(DocumentId),
}

/// Type alias for document operation results
pub type DocumentResult<T> = Result<T, DocumentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_id_generation() {
        let id1 = DocumentId::new();
        let id2 = DocumentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_document_id_from_string() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = DocumentId::from_string(uuid_str).unwrap();
        assert_eq!(id.to_string(), uuid_str);
    }

    #[test]
    fn test_collection_name() {
        let name = CollectionName::new("users");
        assert_eq!(name.as_str(), "users");
        assert_eq!(name.to_string(), "users");
    }

    #[test]
    fn test_document_creation() {
        let content = serde_json::json!({"name": "Alice", "age": 30});
        let doc = Document::new(content.clone());
        assert_eq!(doc.content, content);
        assert_eq!(doc.metadata.version, 1);
    }

    #[test]
    fn test_document_json_serialization() {
        let content = serde_json::json!({"name": "Bob", "count": 5});
        let doc = Document::new(content);

        let json_str = doc.to_json_string().unwrap();
        let parsed_doc = Document::from_json_string(&json_str).unwrap();
        assert_eq!(doc.content, parsed_doc.content);
    }

    #[test]
    fn test_document_metadata_update() {
        let mut metadata = DocumentMetadata::new();
        let original_version = metadata.version;
        let original_updated_at = metadata.updated_at;

        // Sleep a bit to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        metadata.update();

        assert_eq!(metadata.version, original_version + 1);
        assert!(metadata.updated_at >= original_updated_at);
    }
}
