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

//! Data models for the REST API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

// ====== Authentication Models ======

/// JWT token response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenResponse {
    /// JWT access token
    pub access_token: String,

    /// Token type (always "Bearer")
    pub token_type: String,

    /// Token expiration time in seconds
    pub expires_in: u64,
}

/// Login request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username or email
    pub username: String,

    /// Password
    pub password: String,
}

/// User profile information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    /// User ID
    pub id: String,

    /// Username
    pub username: String,

    /// Email address
    pub email: String,

    /// User roles
    pub roles: Vec<String>,

    /// User permissions
    pub permissions: Vec<String>,

    /// Account creation time
    pub created_at: DateTime<Utc>,

    /// Last login time
    pub last_login: Option<DateTime<Utc>>,
}

// ====== Database Models ======

/// Document in a collection
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Document {
    /// Document ID
    pub id: String,

    /// Document content as JSON
    pub content: serde_json::Value,

    /// Document creation time
    pub created_at: DateTime<Utc>,

    /// Document last update time
    pub updated_at: DateTime<Utc>,

    /// Document version
    pub version: u64,
}

/// Request to create a new document
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateDocumentRequest {
    /// Document content as JSON
    pub content: serde_json::Value,
}

/// Request to update a document
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateDocumentRequest {
    /// Updated document content as JSON
    pub content: serde_json::Value,
}

/// Response for document creation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateDocumentResponse {
    /// Created document ID
    pub id: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Collection information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Collection {
    /// Collection name
    pub name: String,

    /// Number of documents in the collection
    pub document_count: u64,

    /// Collection creation time
    pub created_at: DateTime<Utc>,

    /// Collection last update time
    pub updated_at: DateTime<Utc>,
}

/// Paginated list of documents
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentList {
    /// List of documents
    pub documents: Vec<Document>,

    /// Pagination information
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    /// Current page number (1-based)
    pub page: u32,

    /// Number of items per page
    pub page_size: u32,

    /// Total number of items
    pub total_items: u64,

    /// Total number of pages
    pub total_pages: u32,

    /// Whether there is a next page
    pub has_next: bool,

    /// Whether there is a previous page
    pub has_previous: bool,
}

/// Search query parameters
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchQuery {
    /// Search query string
    pub q: String,

    /// Fields to search in
    pub fields: Option<Vec<String>>,

    /// Maximum number of results
    pub limit: Option<u32>,

    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Search results
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchResults {
    /// Matching documents
    pub documents: Vec<Document>,

    /// Total number of matches
    pub total_matches: u64,

    /// Search query that was executed
    pub query: String,

    /// Search execution time in milliseconds
    pub execution_time_ms: u64,
}

// ====== VM Models ======

/// Dot deployment request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeployDotRequest {
    /// Dot name/identifier
    pub name: String,

    /// Bytecode as base64 encoded string
    pub bytecode: String,

    /// ABI specification
    pub abi: Option<serde_json::Value>,

    /// Deployment configuration
    pub config: Option<DotConfig>,
}

/// Dot configuration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DotConfig {
    /// VM architecture to use
    pub architecture: String,

    /// Memory limit in bytes
    pub memory_limit: Option<u64>,

    /// Execution timeout in seconds
    pub timeout_seconds: Option<u64>,

    /// Additional configuration parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Dot deployment response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeployDotResponse {
    /// Deployed dot ID
    pub dot_id: String,

    /// Deployment status
    pub status: DotStatus,

    /// Deployment timestamp
    pub deployed_at: DateTime<Utc>,

    /// Validation results
    pub validation: ValidationResult,
}

/// Dot execution request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecuteDotRequest {
    /// Function name to execute
    pub function: String,

    /// Function arguments
    pub arguments: Vec<serde_json::Value>,

    /// Execution context
    pub context: Option<ExecutionContext>,
}

/// Execution context
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecutionContext {
    /// Caller information
    pub caller: Option<String>,

    /// Transaction ID
    pub transaction_id: Option<String>,

    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,
}

/// Dot execution response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecuteDotResponse {
    /// Execution result
    pub result: serde_json::Value,

    /// Execution status
    pub status: ExecutionStatus,

    /// Gas used
    pub gas_used: u64,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Transaction ID if applicable
    pub transaction_id: Option<String>,
}

/// Dot state information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DotState {
    /// Dot ID
    pub dot_id: String,

    /// Current status
    pub status: DotStatus,

    /// State data
    pub state: serde_json::Value,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Version number
    pub version: u64,
}

/// Dot status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DotStatus {
    /// Dot is being deployed
    Deploying,

    /// Dot is active and ready for execution
    Active,

    /// Dot is paused
    Paused,

    /// Dot has failed
    Failed,

    /// Dot is being terminated
    Terminating,

    /// Dot has been terminated
    Terminated,

    /// Dot has an error
    Error,

    /// Unknown status
    Unknown,
}

/// Execution status enumeration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Success,

    /// Execution failed
    Failed,

    /// Execution timed out
    Timeout,

    /// Execution was cancelled
    Cancelled,

    /// Execution is still running
    Running,
}

/// Validation result
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// Validation errors if any
    pub errors: Vec<String>,

    /// Validation warnings if any
    pub warnings: Vec<String>,
}

// ====== General Models ======

/// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    pub status: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Service version
    pub version: String,

    /// Backend service statuses
    pub services: HashMap<String, ServiceStatus>,
}

/// Individual service status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServiceStatus {
    /// Service status
    pub status: String,

    /// Response time in milliseconds
    pub response_time_ms: u64,

    /// Last checked timestamp
    pub last_checked: DateTime<Utc>,
}

/// API version information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiVersion {
    /// API version
    pub version: String,

    /// Build information
    pub build: String,

    /// Supported features
    pub features: Vec<String>,
}
