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

//! Database handlers

use crate::db::DatabaseClient;
use crate::error::ApiError;
use crate::middleware::{check_permissions, extract_claims};
use crate::models::{Collection, CreateDocumentRequest, CreateDocumentResponse, Document, DocumentList, SearchResults, UpdateDocumentRequest};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Bytes};
use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use tracing::{error, info};

/// List all collections
/// GET /api/v1/collections
#[utoipa::path(
    get,
    path = "/api/v1/collections",
    responses(
        (status = 200, description = "List of collections", body = [Collection]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn list_collections(req: Request<hyper::body::Incoming>, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing list collections request");

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["read:documents"])?;

    // Get collections from database
    let collections = db_client.list_collections().await?;

    info!("Retrieved {} collections", collections.len());

    let response_json = serde_json::to_string(&collections)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Create a new collection
/// POST /api/v1/collections/{collection}
#[utoipa::path(
    post,
    path = "/api/v1/collections/{collection}",
    params(
        ("collection" = String, Path, description = "Collection name")
    ),
    responses(
        (status = 201, description = "Collection created", body = Collection),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Collection already exists")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn create_collection(req: Request<hyper::body::Incoming>, collection_name: String, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing create collection request: {}", collection_name);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["write:documents"])?;

    // Decode collection name
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    // Validate collection name
    if collection_name.is_empty() || collection_name.len() > 64 {
        return Err(ApiError::BadRequest {
            message: "Collection name must be 1-64 characters".to_string(),
        });
    }

    // Create collection
    let collection = db_client.create_collection(&collection_name).await?;

    info!("Created collection: {}", collection_name);

    let response_json = serde_json::to_string(&collection)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Delete a collection
/// DELETE /api/v1/collections/{collection}
#[utoipa::path(
    delete,
    path = "/api/v1/collections/{collection}",
    params(
        ("collection" = String, Path, description = "Collection name")
    ),
    responses(
        (status = 204, description = "Collection deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn delete_collection(req: Request<hyper::body::Incoming>, collection_name: String, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing delete collection request: {}", collection_name);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["delete:documents"])?;

    // Decode collection name
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    // Delete collection
    db_client.delete_collection(&collection_name).await?;

    info!("Deleted collection: {}", collection_name);

    Ok(Response::builder().status(StatusCode::NO_CONTENT).body(Full::new(Bytes::new()))?)
}

/// Get documents from a collection
/// GET /api/v1/collections/{collection}/documents
#[utoipa::path(
    get,
    path = "/api/v1/collections/{collection}/documents",
    params(
        ("collection" = String, Path, description = "Collection name"),
        ("page" = Option<u32>, Query, description = "Page number (1-based)"),
        ("page_size" = Option<u32>, Query, description = "Number of documents per page")
    ),
    responses(
        (status = 200, description = "List of documents", body = DocumentList),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn get_documents(req: Request<hyper::body::Incoming>, collection_name: String, query_params: HashMap<String, String>, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing get documents request: {}", collection_name);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["read:documents"])?;

    // Decode collection name
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    // Parse query parameters
    let page = query_params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let page_size = query_params.get("page_size").and_then(|p| p.parse().ok()).unwrap_or(20);

    // Validate pagination parameters
    if page < 1 || page_size < 1 || page_size > 100 {
        return Err(ApiError::BadRequest {
            message: "Page must be >= 1 and page_size must be 1-100".to_string(),
        });
    }

    // Get documents
    let document_list = db_client.get_documents(&collection_name, page, page_size).await?;

    info!("Retrieved {} documents from collection: {}", document_list.documents.len(), collection_name);

    let response_json = serde_json::to_string(&document_list)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Create a new document
/// POST /api/v1/collections/{collection}/documents
#[utoipa::path(
    post,
    path = "/api/v1/collections/{collection}/documents",
    params(
        ("collection" = String, Path, description = "Collection name")
    ),
    request_body = CreateDocumentRequest,
    responses(
        (status = 201, description = "Document created", body = CreateDocumentResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn create_document(req: Request<hyper::body::Incoming>, collection_name: String, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing create document request: {}", collection_name);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["write:documents"])?;

    // Decode collection name
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let create_request: CreateDocumentRequest = serde_json::from_slice(&body)?;

    // Create document
    let response = db_client.create_document(&collection_name, create_request.content).await?;

    info!("Created document {} in collection: {}", response.id, collection_name);

    let response_json = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Get a document by ID
/// GET /api/v1/collections/{collection}/documents/{id}
#[utoipa::path(
    get,
    path = "/api/v1/collections/{collection}/documents/{id}",
    params(
        ("collection" = String, Path, description = "Collection name"),
        ("id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document found", body = Document),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Document or collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn get_document(req: Request<hyper::body::Incoming>, collection_name: String, document_id: String, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing get document request: {}/{}", collection_name, document_id);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["read:documents"])?;

    // Decode parameters
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    let document_id = percent_decode_str(&document_id)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid document ID encoding".to_string(),
        })?
        .to_string();

    // Get document
    let document = db_client.get_document(&collection_name, &document_id).await?;

    info!("Retrieved document {} from collection: {}", document_id, collection_name);

    let response_json = serde_json::to_string(&document)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Update a document
/// PUT /api/v1/collections/{collection}/documents/{id}
#[utoipa::path(
    put,
    path = "/api/v1/collections/{collection}/documents/{id}",
    params(
        ("collection" = String, Path, description = "Collection name"),
        ("id" = String, Path, description = "Document ID")
    ),
    request_body = UpdateDocumentRequest,
    responses(
        (status = 200, description = "Document updated", body = Document),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Document or collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn update_document(req: Request<hyper::body::Incoming>, collection_name: String, document_id: String, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing update document request: {}/{}", collection_name, document_id);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["write:documents"])?;

    // Decode parameters
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    let document_id = percent_decode_str(&document_id)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid document ID encoding".to_string(),
        })?
        .to_string();

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let update_request: UpdateDocumentRequest = serde_json::from_slice(&body)?;

    // Update document
    let document = db_client.update_document(&collection_name, &document_id, update_request.content).await?;

    info!("Updated document {} in collection: {}", document_id, collection_name);

    let response_json = serde_json::to_string(&document)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Delete a document
/// DELETE /api/v1/collections/{collection}/documents/{id}
#[utoipa::path(
    delete,
    path = "/api/v1/collections/{collection}/documents/{id}",
    params(
        ("collection" = String, Path, description = "Collection name"),
        ("id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 204, description = "Document deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Document or collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn delete_document(req: Request<hyper::body::Incoming>, collection_name: String, document_id: String, db_client: DatabaseClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing delete document request: {}/{}", collection_name, document_id);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["delete:documents"])?;

    // Decode parameters
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    let document_id = percent_decode_str(&document_id)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid document ID encoding".to_string(),
        })?
        .to_string();

    // Delete document
    db_client.delete_document(&collection_name, &document_id).await?;

    info!("Deleted document {} from collection: {}", document_id, collection_name);

    Ok(Response::builder().status(StatusCode::NO_CONTENT).body(Full::new(Bytes::new()))?)
}

/// Search documents in a collection
/// GET /api/v1/collections/{collection}/search
#[utoipa::path(
    get,
    path = "/api/v1/collections/{collection}/search",
    params(
        ("collection" = String, Path, description = "Collection name"),
        ("q" = String, Query, description = "Search query"),
        ("limit" = Option<u32>, Query, description = "Maximum number of results"),
        ("offset" = Option<u32>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Search results", body = SearchResults),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Collection not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database"
)]
pub async fn search_documents(
    req: Request<hyper::body::Incoming>,
    collection_name: String,
    query_params: HashMap<String, String>,
    db_client: DatabaseClient,
) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing search documents request: {}", collection_name);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["read:documents"])?;

    // Decode collection name
    let collection_name = percent_decode_str(&collection_name)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid collection name encoding".to_string(),
        })?
        .to_string();

    // Extract search query
    let query = query_params.get("q").ok_or_else(|| ApiError::BadRequest {
        message: "Missing 'q' query parameter".to_string(),
    })?;

    // Parse optional parameters
    let limit = query_params.get("limit").and_then(|l| l.parse().ok());
    let offset = query_params.get("offset").and_then(|o| o.parse().ok());

    // Validate parameters
    if let Some(limit) = limit {
        if limit > 1000 {
            return Err(ApiError::BadRequest {
                message: "Limit cannot exceed 1000".to_string(),
            });
        }
    }

    // Search documents
    let search_results = db_client.search_documents(&collection_name, query, limit, offset).await?;

    info!("Search found {} matches in collection: {} (query: {})", search_results.total_matches, collection_name, query);

    let response_json = serde_json::to_string(&search_results)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}
