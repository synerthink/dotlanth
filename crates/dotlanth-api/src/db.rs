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

//! Database client for interacting with DotDB core components

use crate::error::{ApiError, ApiResult};
use crate::models::{Collection, CreateDocumentResponse, Document, DocumentList, PaginationInfo, SearchResults};
use chrono::{DateTime, Utc};
use dotdb_core::document::collection::{CollectionManager, create_in_memory_collection_manager};
use dotdb_core::document::{DocumentError, DocumentId};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Database client for DotDB operations
#[derive(Clone)]
pub struct DatabaseClient {
    collection_manager: Arc<Mutex<CollectionManager>>,
}

impl DatabaseClient {
    /// Create a new database client
    pub fn new(_db_service_address: &str) -> ApiResult<Self> {
        info!("Creating database client with DotDB core integration");

        // Create an in-memory collection manager for now
        // In production, this could be persistent storage or a gRPC client
        let collection_manager = create_in_memory_collection_manager().map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to create collection manager: {}", e),
        })?;

        info!("Successfully created database client");

        Ok(Self {
            collection_manager: Arc::new(Mutex::new(collection_manager)),
        })
    }

    /// List all collections
    pub async fn list_collections(&self) -> ApiResult<Vec<Collection>> {
        let manager = self.collection_manager.lock().await;

        let collection_names = manager.list_collections().map_err(|e| self.convert_document_error(e))?;

        let mut result = Vec::new();
        for name in collection_names {
            // CollectionManager doesn't have count_documents method, so we get all IDs and count them
            let doc_ids = manager.list_document_ids(&name).map_err(|e| self.convert_document_error(e))?;
            let document_count = doc_ids.len() as u64;

            let collection = Collection {
                name,
                document_count,
                created_at: Utc::now(), // DotDB doesn't track collection creation time yet
                updated_at: Utc::now(),
            };
            result.push(collection);
        }

        Ok(result)
    }

    /// Create a new collection
    pub async fn create_collection(&self, name: &str) -> ApiResult<Collection> {
        let manager = self.collection_manager.lock().await;

        // Check if collection already exists
        if manager.collection_exists(name).map_err(|e| self.convert_document_error(e))? {
            return Err(ApiError::Conflict {
                message: format!("Collection '{}' already exists", name),
            });
        }

        manager.create_collection(name).map_err(|e| self.convert_document_error(e))?;

        info!("Created collection: {}", name);

        Ok(Collection {
            name: name.to_string(),
            document_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Delete a collection
    pub async fn delete_collection(&self, name: &str) -> ApiResult<()> {
        let manager = self.collection_manager.lock().await;

        let deleted = manager.delete_collection(name).map_err(|e| self.convert_document_error(e))?;

        if !deleted {
            return Err(ApiError::NotFound {
                message: format!("Collection '{}' not found", name),
            });
        }

        info!("Deleted collection: {}", name);
        Ok(())
    }

    /// Get documents from a collection with pagination
    pub async fn get_documents(&self, collection_name: &str, page: u32, page_size: u32) -> ApiResult<DocumentList> {
        let manager = self.collection_manager.lock().await;

        // Check if collection exists
        if !manager.collection_exists(collection_name).map_err(|e| self.convert_document_error(e))? {
            return Err(ApiError::NotFound {
                message: format!("Collection '{}' not found", collection_name),
            });
        }

        // Get all document IDs
        let doc_ids = manager.list_document_ids(collection_name).map_err(|e| self.convert_document_error(e))?;

        let total_items = doc_ids.len() as u64;
        let total_pages = ((total_items as f64) / (page_size as f64)).ceil() as u32;
        let offset = ((page - 1) * page_size) as usize;

        // Apply pagination to document IDs
        let paginated_ids: Vec<DocumentId> = doc_ids.into_iter().skip(offset).take(page_size as usize).collect();

        // Fetch the actual documents
        let mut documents = Vec::new();
        for doc_id in paginated_ids {
            if let Some(content) = manager.get_value(collection_name, &doc_id).map_err(|e| self.convert_document_error(e))? {
                documents.push(Document {
                    id: doc_id.to_string(), // DocumentId contains UUID, convert to string
                    content,
                    created_at: Utc::now(), // DotDB doesn't store timestamps yet
                    updated_at: Utc::now(),
                    version: 1,
                });
            }
        }

        let pagination = PaginationInfo {
            page,
            page_size,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_previous: page > 1,
        };

        Ok(DocumentList { documents, pagination })
    }

    /// Get a document by ID
    pub async fn get_document(&self, collection_name: &str, document_id: &str) -> ApiResult<Document> {
        let manager = self.collection_manager.lock().await;

        let doc_id = DocumentId::from_string(document_id).map_err(|_| ApiError::BadRequest {
            message: format!("Invalid document ID: {}", document_id),
        })?;
        let content = manager
            .get_value(collection_name, &doc_id)
            .map_err(|e| self.convert_document_error(e))?
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Document '{}' not found in collection '{}'", document_id, collection_name),
            })?;

        Ok(Document {
            id: document_id.to_string(),
            content,
            created_at: Utc::now(), // DotDB doesn't store timestamps yet
            updated_at: Utc::now(),
            version: 1,
        })
    }

    /// Create a new document
    pub async fn create_document(&self, collection_name: &str, content: Value) -> ApiResult<CreateDocumentResponse> {
        let manager = self.collection_manager.lock().await;

        let now = Utc::now();

        // Create the document using DotDB (which generates its own ID)
        let doc_id = manager.insert_value(collection_name, content).map_err(|e| self.convert_document_error(e))?;

        let document_id = doc_id.to_string();
        info!("Created document {} in collection: {}", document_id, collection_name);

        Ok(CreateDocumentResponse { id: document_id, created_at: now })
    }

    /// Update a document
    pub async fn update_document(&self, collection_name: &str, document_id: &str, content: Value) -> ApiResult<Document> {
        let manager = self.collection_manager.lock().await;

        let doc_id = DocumentId::from_string(document_id).map_err(|_| ApiError::BadRequest {
            message: format!("Invalid document ID: {}", document_id),
        })?;

        // Check if document exists first
        if manager.get_value(collection_name, &doc_id).map_err(|e| self.convert_document_error(e))?.is_none() {
            return Err(ApiError::NotFound {
                message: format!("Document '{}' not found in collection '{}'", document_id, collection_name),
            });
        }

        // Update the document
        manager.update_value(collection_name, &doc_id, content.clone()).map_err(|e| self.convert_document_error(e))?;

        info!("Updated document {} in collection: {}", document_id, collection_name);

        Ok(Document {
            id: document_id.to_string(),
            content,
            created_at: Utc::now(), // DotDB doesn't store timestamps yet
            updated_at: Utc::now(),
            version: 2, // Increment version
        })
    }

    /// Delete a document
    pub async fn delete_document(&self, collection_name: &str, document_id: &str) -> ApiResult<()> {
        let manager = self.collection_manager.lock().await;

        let doc_id = DocumentId::from_string(document_id).map_err(|_| ApiError::BadRequest {
            message: format!("Invalid document ID: {}", document_id),
        })?;
        let deleted = manager.delete(collection_name, &doc_id).map_err(|e| self.convert_document_error(e))?;

        if !deleted {
            return Err(ApiError::NotFound {
                message: format!("Document '{}' not found in collection '{}'", document_id, collection_name),
            });
        }

        info!("Deleted document {} from collection: {}", document_id, collection_name);
        Ok(())
    }

    /// Search documents in a collection
    pub async fn search_documents(&self, collection_name: &str, query: &str, limit: Option<u32>, offset: Option<u32>) -> ApiResult<SearchResults> {
        let manager = self.collection_manager.lock().await;
        let start_time = std::time::Instant::now();

        // Check if collection exists
        if !manager.collection_exists(collection_name).map_err(|e| self.convert_document_error(e))? {
            return Err(ApiError::NotFound {
                message: format!("Collection '{}' not found", collection_name),
            });
        }

        // Get all documents in the collection for searching
        let doc_ids = manager.list_document_ids(collection_name).map_err(|e| self.convert_document_error(e))?;

        let mut matching_docs = Vec::new();
        let query_lower = query.to_lowercase();

        for doc_id in doc_ids {
            if let Some(content) = manager.get_value(collection_name, &doc_id).map_err(|e| self.convert_document_error(e))? {
                // Simple text search in document content
                if let Ok(content_str) = serde_json::to_string(&content) {
                    if content_str.to_lowercase().contains(&query_lower) {
                        matching_docs.push(Document {
                            id: doc_id.to_string(),
                            content,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                            version: 1,
                        });
                    }
                }
            }
        }

        let total_matches = matching_docs.len() as u64;

        // Apply pagination
        if let Some(offset) = offset {
            matching_docs = matching_docs.into_iter().skip(offset as usize).collect();
        }

        if let Some(limit) = limit {
            matching_docs.truncate(limit as usize);
        }

        let execution_time = start_time.elapsed();

        info!("Search found {} matches in collection: {} (query: {})", total_matches, collection_name, query);

        Ok(SearchResults {
            documents: matching_docs,
            total_matches,
            query: query.to_string(),
            execution_time_ms: execution_time.as_millis() as u64,
        })
    }

    /// Health check for database connection
    pub async fn health_check(&self) -> ApiResult<bool> {
        // Try to access the collection manager
        let _manager = self.collection_manager.lock().await;
        Ok(true)
    }

    /// Convert DotDB DocumentError to ApiError
    fn convert_document_error(&self, error: DocumentError) -> ApiError {
        match error {
            DocumentError::DocumentNotFound(id) => ApiError::NotFound {
                message: format!("Document not found: {}", id.0),
            },
            DocumentError::DocumentAlreadyExists(id) => ApiError::Conflict {
                message: format!("Document already exists: {}", id.0),
            },
            DocumentError::CollectionNotFound(name) => ApiError::NotFound {
                message: format!("Collection not found: {}", name.0),
            },
            DocumentError::InvalidDocumentId(id) => ApiError::BadRequest {
                message: format!("Invalid document ID: {}", id),
            },
            DocumentError::InvalidCollectionName(name) => ApiError::BadRequest {
                message: format!("Invalid collection name: {}", name),
            },
            DocumentError::JsonSerialization(e) => ApiError::InternalServerError {
                message: format!("JSON serialization error: {}", e),
            },
            DocumentError::Database(e) => ApiError::InternalServerError {
                message: format!("Database error: {}", e),
            },
        }
    }
}
