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

//! Document Storage Implementation
//!
//! This module provides the main document storage interface that builds on top
//! of the key-value database interface to provide document-oriented operations.

use super::{CollectionName, Document, DocumentError, DocumentId, DocumentResult};
use crate::state::db_interface::DatabaseInterface;
use std::sync::Arc;

/// Document storage interface
pub trait DocumentStorage: Send + Sync {
    /// Create a new document in a collection
    fn create_document(&self, collection: &CollectionName, document: Document) -> DocumentResult<DocumentId>;

    /// Get a document by ID from a collection
    fn get_document(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<Option<Document>>;

    /// Update an existing document
    fn update_document(&self, collection: &CollectionName, document: Document) -> DocumentResult<()>;

    /// Delete a document by ID
    fn delete_document(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<bool>;

    /// Check if a document exists
    fn document_exists(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<bool>;

    /// List all document IDs in a collection
    fn list_documents(&self, collection: &CollectionName) -> DocumentResult<Vec<DocumentId>>;

    /// Get the number of documents in a collection
    fn count_documents(&self, collection: &CollectionName) -> DocumentResult<usize>;

    /// Create a collection (if it doesn't exist)
    fn create_collection(&self, collection: &CollectionName) -> DocumentResult<()>;

    /// Delete a collection and all its documents
    fn delete_collection(&self, collection: &CollectionName) -> DocumentResult<bool>;

    /// List all collections
    fn list_collections(&self) -> DocumentResult<Vec<CollectionName>>;

    /// Check if a collection exists
    fn collection_exists(&self, collection: &CollectionName) -> DocumentResult<bool>;
}

/// Document storage implementation using the database interface
pub struct DocumentStore {
    db: Arc<dyn DatabaseInterface>,
}

impl DocumentStore {
    /// Create a new document store
    pub fn new(db: Arc<dyn DatabaseInterface>) -> Self {
        Self { db }
    }

    /// Generate storage key for a document
    fn document_key(&self, collection: &CollectionName, id: &DocumentId) -> Vec<u8> {
        format!("doc:{}:{}", collection.as_str(), id).into_bytes()
    }

    /// Generate storage key for collection metadata
    fn collection_key(&self, collection: &CollectionName) -> Vec<u8> {
        format!("col:{}", collection.as_str()).into_bytes()
    }

    /// Generate storage key for collection document list
    fn collection_docs_key(&self, collection: &CollectionName) -> Vec<u8> {
        format!("col_docs:{}", collection.as_str()).into_bytes()
    }

    /// Generate storage key for global collections list
    fn collections_list_key(&self) -> Vec<u8> {
        b"collections".to_vec()
    }

    /// Serialize document to bytes
    fn serialize_document(&self, document: &Document) -> DocumentResult<Vec<u8>> {
        Ok(serde_json::to_vec(document)?)
    }

    /// Deserialize document from bytes
    fn deserialize_document(&self, data: &[u8]) -> DocumentResult<Document> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Serialize document ID list to bytes
    fn serialize_doc_list(&self, ids: &[DocumentId]) -> DocumentResult<Vec<u8>> {
        Ok(serde_json::to_vec(ids)?)
    }

    /// Deserialize document ID list from bytes
    fn deserialize_doc_list(&self, data: &[u8]) -> DocumentResult<Vec<DocumentId>> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Serialize collection list to bytes
    fn serialize_collection_list(&self, collections: &[CollectionName]) -> DocumentResult<Vec<u8>> {
        Ok(serde_json::to_vec(collections)?)
    }

    /// Deserialize collection list from bytes
    fn deserialize_collection_list(&self, data: &[u8]) -> DocumentResult<Vec<CollectionName>> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Add document ID to collection's document list
    fn add_to_collection_docs(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<()> {
        let key = self.collection_docs_key(collection);
        let mut doc_ids = if let Some(data) = self.db.get(&key)? { self.deserialize_doc_list(&data)? } else { Vec::new() };

        if !doc_ids.contains(id) {
            doc_ids.push(id.clone());
            let serialized = self.serialize_doc_list(&doc_ids)?;
            self.db.put(key, serialized)?;
        }

        Ok(())
    }

    /// Remove document ID from collection's document list
    fn remove_from_collection_docs(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<()> {
        let key = self.collection_docs_key(collection);
        if let Some(data) = self.db.get(&key)? {
            let mut doc_ids = self.deserialize_doc_list(&data)?;
            doc_ids.retain(|doc_id| doc_id != id);
            let serialized = self.serialize_doc_list(&doc_ids)?;
            self.db.put(key, serialized)?;
        }

        Ok(())
    }

    /// Add collection to global collections list
    fn add_to_collections_list(&self, collection: &CollectionName) -> DocumentResult<()> {
        let key = self.collections_list_key();
        let mut collections = if let Some(data) = self.db.get(&key)? {
            self.deserialize_collection_list(&data)?
        } else {
            Vec::new()
        };

        if !collections.contains(collection) {
            collections.push(collection.clone());
            let serialized = self.serialize_collection_list(&collections)?;
            self.db.put(key, serialized)?;
        }

        Ok(())
    }

    /// Remove collection from global collections list
    fn remove_from_collections_list(&self, collection: &CollectionName) -> DocumentResult<()> {
        let key = self.collections_list_key();
        if let Some(data) = self.db.get(&key)? {
            let mut collections = self.deserialize_collection_list(&data)?;
            collections.retain(|col| col != collection);
            let serialized = self.serialize_collection_list(&collections)?;
            self.db.put(key, serialized)?;
        }

        Ok(())
    }
}

impl DocumentStorage for DocumentStore {
    fn create_document(&self, collection: &CollectionName, mut document: Document) -> DocumentResult<DocumentId> {
        // Ensure collection exists
        self.create_collection(collection)?;

        // Check if document already exists
        let doc_key = self.document_key(collection, &document.id);
        if self.db.contains(&doc_key)? {
            return Err(DocumentError::DocumentAlreadyExists(document.id.clone()));
        }

        // Update metadata
        document.metadata.update();

        // Store document
        let serialized = self.serialize_document(&document)?;
        self.db.put(doc_key, serialized)?;

        // Add to collection's document list
        self.add_to_collection_docs(collection, &document.id)?;

        Ok(document.id)
    }

    fn get_document(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<Option<Document>> {
        let key = self.document_key(collection, id);
        match self.db.get(&key)? {
            Some(data) => {
                let document = self.deserialize_document(&data)?;
                Ok(Some(document))
            }
            None => Ok(None),
        }
    }

    fn update_document(&self, collection: &CollectionName, mut document: Document) -> DocumentResult<()> {
        // Check if document exists
        let doc_key = self.document_key(collection, &document.id);
        if !self.db.contains(&doc_key)? {
            return Err(DocumentError::DocumentNotFound(document.id.clone()));
        }

        // Update metadata
        document.metadata.update();

        // Store updated document
        let serialized = self.serialize_document(&document)?;
        self.db.put(doc_key, serialized)?;

        Ok(())
    }

    fn delete_document(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<bool> {
        let key = self.document_key(collection, id);
        let existed = self.db.delete(&key)?;

        if existed {
            // Remove from collection's document list
            self.remove_from_collection_docs(collection, id)?;
        }

        Ok(existed)
    }

    fn document_exists(&self, collection: &CollectionName, id: &DocumentId) -> DocumentResult<bool> {
        let key = self.document_key(collection, id);
        Ok(self.db.contains(&key)?)
    }

    fn list_documents(&self, collection: &CollectionName) -> DocumentResult<Vec<DocumentId>> {
        let key = self.collection_docs_key(collection);
        match self.db.get(&key)? {
            Some(data) => Ok(self.deserialize_doc_list(&data)?),
            None => Ok(Vec::new()),
        }
    }

    fn count_documents(&self, collection: &CollectionName) -> DocumentResult<usize> {
        let doc_ids = self.list_documents(collection)?;
        Ok(doc_ids.len())
    }

    fn create_collection(&self, collection: &CollectionName) -> DocumentResult<()> {
        let key = self.collection_key(collection);
        if !self.db.contains(&key)? {
            // Create collection metadata
            let metadata = serde_json::json!({
                "name": collection.as_str(),
                "created_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            });
            let serialized = serde_json::to_vec(&metadata)?;
            self.db.put(key, serialized)?;

            // Add to global collections list
            self.add_to_collections_list(collection)?;
        }

        Ok(())
    }

    fn delete_collection(&self, collection: &CollectionName) -> DocumentResult<bool> {
        // Check if collection exists
        let col_key = self.collection_key(collection);
        if !self.db.contains(&col_key)? {
            return Ok(false);
        }

        // Delete all documents in the collection
        let doc_ids = self.list_documents(collection)?;
        for id in doc_ids {
            let doc_key = self.document_key(collection, &id);
            self.db.delete(&doc_key)?;
        }

        // Delete collection document list
        let docs_key = self.collection_docs_key(collection);
        self.db.delete(&docs_key)?;

        // Delete collection metadata
        self.db.delete(&col_key)?;

        // Remove from global collections list
        self.remove_from_collections_list(collection)?;

        Ok(true)
    }

    fn list_collections(&self) -> DocumentResult<Vec<CollectionName>> {
        let key = self.collections_list_key();
        match self.db.get(&key)? {
            Some(data) => Ok(self.deserialize_collection_list(&data)?),
            None => Ok(Vec::new()),
        }
    }

    fn collection_exists(&self, collection: &CollectionName) -> DocumentResult<bool> {
        let key = self.collection_key(collection);
        Ok(self.db.contains(&key)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::db_interface::{Database, DbConfig};

    fn create_test_store() -> DocumentStore {
        let db = Arc::new(Database::new_in_memory().unwrap());
        DocumentStore::new(db)
    }

    #[test]
    fn test_create_and_get_document() {
        let store = create_test_store();
        let collection = CollectionName::new("users");

        let content = serde_json::json!({"name": "Alice", "age": 30});
        let document = Document::new(content.clone());
        let doc_id = document.id.clone();

        // Create document
        let created_id = store.create_document(&collection, document).unwrap();
        assert_eq!(created_id, doc_id);

        // Get document
        let retrieved = store.get_document(&collection, &doc_id).unwrap();
        assert!(retrieved.is_some());
        let retrieved_doc = retrieved.unwrap();
        assert_eq!(retrieved_doc.id, doc_id);
        assert_eq!(retrieved_doc.content, content);
    }

    #[test]
    fn test_update_document() {
        let store = create_test_store();
        let collection = CollectionName::new("users");

        let content = serde_json::json!({"name": "Bob", "count": 5});
        let document = Document::new(content);
        let doc_id = document.id.clone();

        // Create document
        store.create_document(&collection, document).unwrap();

        // Update document
        let updated_content = serde_json::json!({"name": "Bob", "count": 6});
        let mut updated_doc = Document::with_id(doc_id.clone(), updated_content.clone());
        store.update_document(&collection, updated_doc).unwrap();

        // Verify update
        let retrieved = store.get_document(&collection, &doc_id).unwrap().unwrap();
        assert_eq!(retrieved.content, updated_content);
    }

    #[test]
    fn test_delete_document() {
        let store = create_test_store();
        let collection = CollectionName::new("users");

        let content = serde_json::json!({"name": "Charlie"});
        let document = Document::new(content);
        let doc_id = document.id.clone();

        // Create document
        store.create_document(&collection, document).unwrap();
        assert!(store.document_exists(&collection, &doc_id).unwrap());

        // Delete document
        let deleted = store.delete_document(&collection, &doc_id).unwrap();
        assert!(deleted);
        assert!(!store.document_exists(&collection, &doc_id).unwrap());

        // Try to delete again
        let deleted_again = store.delete_document(&collection, &doc_id).unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_list_documents() {
        let store = create_test_store();
        let collection = CollectionName::new("test");

        // Initially empty
        let docs = store.list_documents(&collection).unwrap();
        assert!(docs.is_empty());

        // Add some documents
        let doc1 = Document::new(serde_json::json!({"id": 1}));
        let doc2 = Document::new(serde_json::json!({"id": 2}));
        let id1 = doc1.id.clone();
        let id2 = doc2.id.clone();

        store.create_document(&collection, doc1).unwrap();
        store.create_document(&collection, doc2).unwrap();

        // List documents
        let docs = store.list_documents(&collection).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs.contains(&id1));
        assert!(docs.contains(&id2));

        // Count documents
        let count = store.count_documents(&collection).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_collection_operations() {
        let store = create_test_store();
        let collection1 = CollectionName::new("col1");
        let collection2 = CollectionName::new("col2");

        // Initially no collections
        let collections = store.list_collections().unwrap();
        assert!(collections.is_empty());

        // Create collections
        store.create_collection(&collection1).unwrap();
        store.create_collection(&collection2).unwrap();

        // List collections
        let collections = store.list_collections().unwrap();
        assert_eq!(collections.len(), 2);
        assert!(collections.contains(&collection1));
        assert!(collections.contains(&collection2));

        // Check existence
        assert!(store.collection_exists(&collection1).unwrap());
        assert!(store.collection_exists(&collection2).unwrap());

        // Delete collection
        let deleted = store.delete_collection(&collection1).unwrap();
        assert!(deleted);
        assert!(!store.collection_exists(&collection1).unwrap());

        // List collections again
        let collections = store.list_collections().unwrap();
        assert_eq!(collections.len(), 1);
        assert!(collections.contains(&collection2));
    }

    #[test]
    fn test_duplicate_document_creation() {
        let store = create_test_store();
        let collection = CollectionName::new("test");

        let content = serde_json::json!({"name": "Test"});
        let document = Document::new(content);
        let doc_id = document.id.clone();

        // Create document
        store.create_document(&collection, document.clone()).unwrap();

        // Try to create same document again
        let result = store.create_document(&collection, document);
        assert!(matches!(result, Err(DocumentError::DocumentAlreadyExists(_))));
    }

    #[test]
    fn test_update_nonexistent_document() {
        let store = create_test_store();
        let collection = CollectionName::new("test");

        let content = serde_json::json!({"name": "Test"});
        let document = Document::new(content);

        // Try to update non-existent document
        let result = store.update_document(&collection, document);
        assert!(matches!(result, Err(DocumentError::DocumentNotFound(_))));
    }
}
