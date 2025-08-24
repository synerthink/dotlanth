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

//! Collection Management
//!
//! This module provides high-level collection management operations
//! for organizing documents in the document store.

use super::{CollectionName, Document, DocumentId, DocumentResult, DocumentStorage};
use serde_json::Value;
use std::sync::Arc;

/// Collection manager for high-level document operations
pub struct CollectionManager {
    storage: Arc<dyn DocumentStorage>,
}

impl CollectionManager {
    /// Create a new collection manager
    pub fn new(storage: Arc<dyn DocumentStorage>) -> Self {
        Self { storage }
    }

    /// Insert a JSON document into a collection
    pub fn insert_json(&self, collection: &str, json: &str) -> DocumentResult<DocumentId> {
        let collection_name = CollectionName::new(collection);
        let document = Document::from_json_string(json)?;
        self.storage.create_document(&collection_name, document)
    }

    /// Insert a JSON value into a collection
    pub fn insert_value(&self, collection: &str, value: Value) -> DocumentResult<DocumentId> {
        let collection_name = CollectionName::new(collection);
        let document = Document::new(value);
        self.storage.create_document(&collection_name, document)
    }

    /// Get a document as JSON string
    pub fn get_json(&self, collection: &str, id: &DocumentId) -> DocumentResult<Option<String>> {
        let collection_name = CollectionName::new(collection);
        match self.storage.get_document(&collection_name, id)? {
            Some(document) => Ok(Some(document.to_json_string()?)),
            None => Ok(None),
        }
    }

    /// Get a document as JSON value
    pub fn get_value(&self, collection: &str, id: &DocumentId) -> DocumentResult<Option<Value>> {
        let collection_name = CollectionName::new(collection);
        match self.storage.get_document(&collection_name, id)? {
            Some(document) => Ok(Some(document.content)),
            None => Ok(None),
        }
    }

    /// Update a document with JSON string
    pub fn update_json(&self, collection: &str, id: &DocumentId, json: &str) -> DocumentResult<()> {
        let collection_name = CollectionName::new(collection);
        let content: Value = serde_json::from_str(json)?;
        let document = Document::with_id(id.clone(), content);
        self.storage.update_document(&collection_name, document)
    }

    /// Update a document with JSON value
    pub fn update_value(&self, collection: &str, id: &DocumentId, value: Value) -> DocumentResult<()> {
        let collection_name = CollectionName::new(collection);
        let document = Document::with_id(id.clone(), value);
        self.storage.update_document(&collection_name, document)
    }

    /// Delete a document
    pub fn delete(&self, collection: &str, id: &DocumentId) -> DocumentResult<bool> {
        let collection_name = CollectionName::new(collection);
        self.storage.delete_document(&collection_name, id)
    }

    /// Check if a document exists
    pub fn exists(&self, collection: &str, id: &DocumentId) -> DocumentResult<bool> {
        let collection_name = CollectionName::new(collection);
        self.storage.document_exists(&collection_name, id)
    }

    /// List all document IDs in a collection
    pub fn list_document_ids(&self, collection: &str) -> DocumentResult<Vec<DocumentId>> {
        let collection_name = CollectionName::new(collection);
        self.storage.list_documents(&collection_name)
    }

    /// Get all documents in a collection as JSON values
    pub fn get_all_values(&self, collection: &str) -> DocumentResult<Vec<(DocumentId, Value)>> {
        let collection_name = CollectionName::new(collection);
        let doc_ids = self.storage.list_documents(&collection_name)?;
        let mut documents = Vec::new();

        for id in doc_ids {
            if let Some(document) = self.storage.get_document(&collection_name, &id)? {
                documents.push((id, document.content));
            }
        }

        Ok(documents)
    }

    /// Count documents in a collection
    pub fn count(&self, collection: &str) -> DocumentResult<usize> {
        let collection_name = CollectionName::new(collection);
        self.storage.count_documents(&collection_name)
    }

    /// Create a collection
    pub fn create_collection(&self, collection: &str) -> DocumentResult<()> {
        let collection_name = CollectionName::new(collection);
        self.storage.create_collection(&collection_name)
    }

    /// Delete a collection and all its documents
    pub fn delete_collection(&self, collection: &str) -> DocumentResult<bool> {
        let collection_name = CollectionName::new(collection);
        self.storage.delete_collection(&collection_name)
    }

    /// List all collections
    pub fn list_collections(&self) -> DocumentResult<Vec<String>> {
        let collections = self.storage.list_collections()?;
        Ok(collections.into_iter().map(|c| c.0).collect())
    }

    /// Check if a collection exists
    pub fn collection_exists(&self, collection: &str) -> DocumentResult<bool> {
        let collection_name = CollectionName::new(collection);
        self.storage.collection_exists(&collection_name)
    }

    /// Find documents by a simple field match (basic query functionality)
    pub fn find_by_field(&self, collection: &str, field: &str, value: &Value) -> DocumentResult<Vec<(DocumentId, Value)>> {
        let collection_name = CollectionName::new(collection);
        let doc_ids = self.storage.list_documents(&collection_name)?;
        let mut matching_docs = Vec::new();

        for id in doc_ids {
            if let Some(document) = self.storage.get_document(&collection_name, &id)?
                && let Some(field_value) = document.content.get(field)
                && field_value == value
            {
                matching_docs.push((id, document.content));
            }
        }

        Ok(matching_docs)
    }

    /// Get the underlying storage interface
    pub fn storage(&self) -> &Arc<dyn DocumentStorage> {
        &self.storage
    }
}

/// Helper function to create a collection manager with in-memory storage
pub fn create_in_memory_collection_manager() -> DocumentResult<CollectionManager> {
    use super::storage::DocumentStore;
    use crate::state::db_interface::Database;

    let db = Arc::new(Database::new_in_memory()?);
    let storage = Arc::new(DocumentStore::new(db));
    Ok(CollectionManager::new(storage))
}

/// Helper function to create a collection manager with persistent storage
pub fn create_persistent_collection_manager<P: AsRef<std::path::Path>>(path: P, config: Option<crate::state::db_interface::DbConfig>) -> DocumentResult<CollectionManager> {
    use super::storage::DocumentStore;
    use crate::state::db_interface::Database;

    // Ensure the directory exists before creating the database
    std::fs::create_dir_all(&path).map_err(|e| crate::document::DocumentError::InvalidDocumentId(format!("Failed to create database directory: {}", e)))?;

    let config = config.unwrap_or_default();
    let db = Arc::new(Database::new(path, config)?);
    let storage = Arc::new(DocumentStore::new(db));
    Ok(CollectionManager::new(storage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_manager() -> CollectionManager {
        create_in_memory_collection_manager().unwrap()
    }

    #[test]
    fn test_insert_and_get_json() {
        let manager = create_test_manager();

        let json = r#"{"name": "Alice", "age": 30}"#;
        let id = manager.insert_json("users", json).unwrap();

        let retrieved = manager.get_json("users", &id).unwrap();
        assert!(retrieved.is_some());

        let retrieved_value: Value = serde_json::from_str(&retrieved.unwrap()).unwrap();
        let original_value: Value = serde_json::from_str(json).unwrap();
        assert_eq!(retrieved_value, original_value);
    }

    #[test]
    fn test_insert_and_get_value() {
        let manager = create_test_manager();

        let value = json!({"name": "Bob", "count": 5});
        let id = manager.insert_value("users", value.clone()).unwrap();

        let retrieved = manager.get_value("users", &id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), value);
    }

    #[test]
    fn test_update_operations() {
        let manager = create_test_manager();

        let original_value = json!({"name": "Charlie", "count": 1});
        let id = manager.insert_value("users", original_value).unwrap();

        // Update with JSON string
        let updated_json = r#"{"name": "Charlie", "count": 2}"#;
        manager.update_json("users", &id, updated_json).unwrap();

        let retrieved = manager.get_value("users", &id).unwrap().unwrap();
        assert_eq!(retrieved["count"], 2);

        // Update with value
        let updated_value = json!({"name": "Charlie", "count": 3});
        manager.update_value("users", &id, updated_value.clone()).unwrap();

        let retrieved = manager.get_value("users", &id).unwrap().unwrap();
        assert_eq!(retrieved, updated_value);
    }

    #[test]
    fn test_delete_operations() {
        let manager = create_test_manager();

        let value = json!({"name": "David"});
        let id = manager.insert_value("users", value).unwrap();

        assert!(manager.exists("users", &id).unwrap());

        let deleted = manager.delete("users", &id).unwrap();
        assert!(deleted);
        assert!(!manager.exists("users", &id).unwrap());

        // Try to delete again
        let deleted_again = manager.delete("users", &id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_collection_operations() {
        let manager = create_test_manager();

        // Initially no collections
        let collections = manager.list_collections().unwrap();
        assert!(collections.is_empty());

        // Create collection
        manager.create_collection("test").unwrap();
        assert!(manager.collection_exists("test").unwrap());

        let collections = manager.list_collections().unwrap();
        assert_eq!(collections.len(), 1);
        assert!(collections.contains(&"test".to_string()));

        // Delete collection
        let deleted = manager.delete_collection("test").unwrap();
        assert!(deleted);
        assert!(!manager.collection_exists("test").unwrap());
    }

    #[test]
    fn test_list_and_count_documents() {
        let manager = create_test_manager();

        // Add some documents
        let id1 = manager.insert_value("test", json!({"id": 1})).unwrap();
        let id2 = manager.insert_value("test", json!({"id": 2})).unwrap();
        let id3 = manager.insert_value("test", json!({"id": 3})).unwrap();

        // List document IDs
        let doc_ids = manager.list_document_ids("test").unwrap();
        assert_eq!(doc_ids.len(), 3);
        assert!(doc_ids.contains(&id1));
        assert!(doc_ids.contains(&id2));
        assert!(doc_ids.contains(&id3));

        // Count documents
        let count = manager.count("test").unwrap();
        assert_eq!(count, 3);

        // Get all documents
        let all_docs = manager.get_all_values("test").unwrap();
        assert_eq!(all_docs.len(), 3);
    }

    #[test]
    fn test_find_by_field() {
        let manager = create_test_manager();

        // Add some documents
        manager.insert_value("users", json!({"name": "Alice", "role": "admin"})).unwrap();
        manager.insert_value("users", json!({"name": "Bob", "role": "user"})).unwrap();
        manager.insert_value("users", json!({"name": "Charlie", "role": "admin"})).unwrap();

        // Find by role
        let admins = manager.find_by_field("users", "role", &json!("admin")).unwrap();
        assert_eq!(admins.len(), 2);

        let users = manager.find_by_field("users", "role", &json!("user")).unwrap();
        assert_eq!(users.len(), 1);

        // Find by name
        let alice = manager.find_by_field("users", "name", &json!("Alice")).unwrap();
        assert_eq!(alice.len(), 1);
        assert_eq!(alice[0].1["role"], "admin");
    }

    #[test]
    fn test_showcase_scenario() {
        let manager = create_test_manager();

        // Step 1: Insert a user document (equivalent to `dotdb put users '{"name": "Ada", "count": 5}'`)
        let user_data = json!({"name": "Ada", "count": 5});
        let user_id = manager.insert_value("users", user_data).unwrap();

        // Step 2: Read the user, increment count, and save back
        let mut user = manager.get_value("users", &user_id).unwrap().unwrap();
        let current_count = user["count"].as_i64().unwrap();
        user["count"] = json!(current_count + 1);
        manager.update_value("users", &user_id, user).unwrap();

        // Step 3: Verify the change (equivalent to `dotdb get users <id>`)
        let updated_user = manager.get_value("users", &user_id).unwrap().unwrap();
        assert_eq!(updated_user["count"], 6);
        assert_eq!(updated_user["name"], "Ada");
    }
}
