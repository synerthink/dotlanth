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

//! VM-Database Bridge
//!
//! This module provides the bridge between the VM execution engine and the database.
//! It handles database operations called from VM bytecode.

use dotdb_core::document::{CollectionManager, DocumentId};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Database bridge for VM operations
#[derive(Clone)]
pub struct DatabaseBridge {
    collection_manager: Arc<Mutex<CollectionManager>>,
}

impl DatabaseBridge {
    /// Create a new database bridge with an in-memory collection manager
    pub fn new() -> Self {
        let collection_manager = dotdb_core::document::create_in_memory_collection_manager().expect("Failed to create in-memory collection manager");
        Self {
            collection_manager: Arc::new(Mutex::new(collection_manager)),
        }
    }

    /// Create a database bridge with a provided collection manager
    pub fn with_collection_manager(collection_manager: CollectionManager) -> Self {
        Self {
            collection_manager: Arc::new(Mutex::new(collection_manager)),
        }
    }

    /// Get a document from the database
    pub fn get_document(&self, collection: &str, document_id: &str) -> Result<Option<String>, DatabaseBridgeError> {
        let manager = self.collection_manager.lock().map_err(|_| DatabaseBridgeError::LockError)?;
        
        // Parse document ID
        let doc_id = DocumentId::from_string(document_id).map_err(|e| DatabaseBridgeError::InvalidDocumentId(e.to_string()))?;
        
        // Get document
        match manager.get_json(collection, &doc_id) {
            Ok(Some(json_str)) => Ok(Some(json_str)),
            Ok(None) => Ok(None),
            Err(e) => Err(DatabaseBridgeError::DatabaseError(e.to_string())),
        }
    }

    /// Put a document to the database
    pub fn put_document(&self, collection: &str, document_json: &str) -> Result<String, DatabaseBridgeError> {
        let manager = self.collection_manager.lock().map_err(|_| DatabaseBridgeError::LockError)?;
        
        // Insert JSON document
        let doc_id = manager.insert_json(collection, document_json).map_err(|e| DatabaseBridgeError::DatabaseError(e.to_string()))?;
        
        Ok(doc_id.to_string())
    }

    /// Update a document in the database
    pub fn update_document(&self, collection: &str, document_id: &str, document_json: &str) -> Result<(), DatabaseBridgeError> {
        let manager = self.collection_manager.lock().map_err(|_| DatabaseBridgeError::LockError)?;
        
        // Parse document ID
        let doc_id = DocumentId::from_string(document_id).map_err(|e| DatabaseBridgeError::InvalidDocumentId(e.to_string()))?;
        
        // Update document
        manager.update_json(collection, &doc_id, document_json).map_err(|e| DatabaseBridgeError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }

    /// Delete a document from the database
    pub fn delete_document(&self, collection: &str, document_id: &str) -> Result<(), DatabaseBridgeError> {
        let manager = self.collection_manager.lock().map_err(|_| DatabaseBridgeError::LockError)?;
        
        // Parse document ID
        let doc_id = DocumentId::from_string(document_id).map_err(|e| DatabaseBridgeError::InvalidDocumentId(e.to_string()))?;
        
        // Delete document
        manager.delete(collection, &doc_id).map_err(|e| DatabaseBridgeError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }

    /// List documents in a collection
    pub fn list_documents(&self, collection: &str) -> Result<Vec<String>, DatabaseBridgeError> {
        let manager = self.collection_manager.lock().map_err(|_| DatabaseBridgeError::LockError)?;
        
        // List documents
        let doc_ids = manager.list_document_ids(collection).map_err(|e| DatabaseBridgeError::DatabaseError(e.to_string()))?;
        
        Ok(doc_ids.into_iter().map(|id| id.to_string()).collect())
    }

    /// Create a collection
    pub fn create_collection(&self, collection: &str) -> Result<(), DatabaseBridgeError> {
        let manager = self.collection_manager.lock().map_err(|_| DatabaseBridgeError::LockError)?;
        
        // Create collection
        manager.create_collection(collection).map_err(|e| DatabaseBridgeError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }

    /// Delete a collection
    pub fn delete_collection(&self, collection: &str) -> Result<(), DatabaseBridgeError> {
        let manager = self.collection_manager.lock().map_err(|_| DatabaseBridgeError::LockError)?;
        
        // Delete collection
        manager.delete_collection(collection).map_err(|e| DatabaseBridgeError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }

    /// Get the collection manager (for advanced operations)
    pub fn collection_manager(&self) -> Arc<Mutex<CollectionManager>> {
        self.collection_manager.clone()
    }
}

impl Default for DatabaseBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DatabaseBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseBridge")
            .field("collection_manager", &"Arc<Mutex<CollectionManager>>")
            .finish()
    }
}

/// Database bridge errors
#[derive(Debug, thiserror::Error)]
pub enum DatabaseBridgeError {
    #[error("Database lock error")]
    LockError,

    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    #[error("Invalid document ID: {0}")]
    InvalidDocumentId(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_bridge_creation() {
        let bridge = DatabaseBridge::new();
        assert!(bridge.collection_manager.lock().is_ok());
    }

    #[test]
    fn test_put_and_get_document() {
        let bridge = DatabaseBridge::new();
        
        // Create collection first
        bridge.create_collection("test_collection").unwrap();
        
        // Put document
        let doc_json = r#"{"name": "Alice", "age": 30}"#;
        let doc_id = bridge.put_document("test_collection", doc_json).unwrap();
        
        // Get document
        let retrieved = bridge.get_document("test_collection", &doc_id).unwrap();
        assert!(retrieved.is_some());
        
        let retrieved_json: Value = serde_json::from_str(&retrieved.unwrap()).unwrap();
        let original_json: Value = serde_json::from_str(doc_json).unwrap();
        
        // Compare the data (ignoring potential ordering differences)
        assert_eq!(retrieved_json["name"], original_json["name"]);
        assert_eq!(retrieved_json["age"], original_json["age"]);
    }

    #[test]
    fn test_update_document() {
        let bridge = DatabaseBridge::new();
        
        // Create collection
        bridge.create_collection("test_collection").unwrap();
        
        // Put document
        let doc_json = r#"{"name": "Bob", "age": 25}"#;
        let doc_id = bridge.put_document("test_collection", doc_json).unwrap();
        
        // Update document
        let updated_json = r#"{"name": "Bob", "age": 26}"#;
        bridge.update_document("test_collection", &doc_id, updated_json).unwrap();
        
        // Get updated document
        let retrieved = bridge.get_document("test_collection", &doc_id).unwrap().unwrap();
        let retrieved_json: Value = serde_json::from_str(&retrieved).unwrap();
        
        assert_eq!(retrieved_json["age"], 26);
    }

    #[test]
    fn test_delete_document() {
        let bridge = DatabaseBridge::new();
        
        // Create collection
        bridge.create_collection("test_collection").unwrap();
        
        // Put document
        let doc_json = r#"{"name": "Charlie", "age": 35}"#;
        let doc_id = bridge.put_document("test_collection", doc_json).unwrap();
        
        // Delete document
        bridge.delete_document("test_collection", &doc_id).unwrap();
        
        // Try to get deleted document
        let retrieved = bridge.get_document("test_collection", &doc_id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_documents() {
        let bridge = DatabaseBridge::new();
        
        // Create collection
        bridge.create_collection("test_collection").unwrap();
        
        // Put multiple documents
        let doc1_id = bridge.put_document("test_collection", r#"{"name": "Doc1"}"#).unwrap();
        let doc2_id = bridge.put_document("test_collection", r#"{"name": "Doc2"}"#).unwrap();
        
        // List documents
        let doc_ids = bridge.list_documents("test_collection").unwrap();
        
        assert_eq!(doc_ids.len(), 2);
        assert!(doc_ids.contains(&doc1_id));
        assert!(doc_ids.contains(&doc2_id));
    }

    #[test]
    fn test_invalid_json() {
        let bridge = DatabaseBridge::new();
        
        // Create collection
        bridge.create_collection("test_collection").unwrap();
        
        // Try to put invalid JSON
        let result = bridge.put_document("test_collection", "invalid json");
        assert!(matches!(result, Err(DatabaseBridgeError::InvalidJson(_))));
    }
}