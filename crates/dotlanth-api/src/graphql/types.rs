// Dotlanth
// Copyright (C) 2025 Synerthink

use crate::models;
use async_graphql::{InputObject, Json, SimpleObject};
use chrono::{DateTime, Utc};

#[derive(SimpleObject, Clone)]
pub struct GqlDocument {
    pub id: String,
    pub content: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
}

impl From<models::Document> for GqlDocument {
    fn from(d: models::Document) -> Self {
        Self {
            id: d.id,
            content: Json(d.content),
            created_at: d.created_at,
            updated_at: d.updated_at,
            version: d.version,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlCollection {
    pub name: String,
    pub document_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<models::Collection> for GqlCollection {
    fn from(c: models::Collection) -> Self {
        Self {
            name: c.name,
            document_count: c.document_count,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlApiVersion {
    pub version: String,
    pub build: String,
    pub features: Vec<String>,
}

#[derive(SimpleObject, Clone)]
pub struct GqlPaginationInfo {
    pub page: u32,
    pub page_size: u32,
    pub total_items: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_previous: bool,
}

impl From<models::PaginationInfo> for GqlPaginationInfo {
    fn from(p: models::PaginationInfo) -> Self {
        Self {
            page: p.page,
            page_size: p.page_size,
            total_items: p.total_items,
            total_pages: p.total_pages,
            has_next: p.has_next,
            has_previous: p.has_previous,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlDocumentList {
    pub documents: Vec<GqlDocument>,
    pub pagination: GqlPaginationInfo,
}

impl From<models::DocumentList> for GqlDocumentList {
    fn from(d: models::DocumentList) -> Self {
        Self {
            documents: d.documents.into_iter().map(GqlDocument::from).collect(),
            pagination: d.pagination.into(),
        }
    }
}

#[derive(InputObject, Clone)]
pub struct GqlDotConfig {
    pub architecture: String,
    pub memory_limit: Option<u64>,
    pub timeout_seconds: Option<u64>,
    pub parameters: std::collections::HashMap<String, Json<serde_json::Value>>,
}

impl From<GqlDotConfig> for models::DotConfig {
    fn from(c: GqlDotConfig) -> Self {
        Self {
            architecture: c.architecture,
            memory_limit: c.memory_limit,
            timeout_seconds: c.timeout_seconds,
            parameters: c.parameters.into_iter().map(|(k, Json(v))| (k, v)).collect(),
        }
    }
}

#[derive(InputObject, Clone)]
pub struct GqlDeployDotInput {
    pub name: String,
    pub bytecode: String,
    pub abi: Option<Json<serde_json::Value>>,
    pub config: Option<GqlDotConfig>,
}

impl From<GqlDeployDotInput> for models::DeployDotRequest {
    fn from(i: GqlDeployDotInput) -> Self {
        Self {
            name: i.name,
            bytecode: i.bytecode,
            abi: i.abi.map(|Json(v)| v),
            config: i.config.map(Into::into),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlDeployDotResponse {
    pub dot_id: String,
    pub status: String,
    pub deployed_at: DateTime<Utc>,
    pub validation_valid: bool,
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,
}

impl From<models::DeployDotResponse> for GqlDeployDotResponse {
    fn from(r: models::DeployDotResponse) -> Self {
        Self {
            dot_id: r.dot_id,
            status: format!("{:?}", r.status),
            deployed_at: r.deployed_at,
            validation_valid: r.validation.valid,
            validation_errors: r.validation.errors,
            validation_warnings: r.validation.warnings,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlCreateDocumentResponse {
    pub id: String,
    pub created_at: DateTime<Utc>,
}

impl From<models::CreateDocumentResponse> for GqlCreateDocumentResponse {
    fn from(r: models::CreateDocumentResponse) -> Self {
        Self { id: r.id, created_at: r.created_at }
    }
}

#[derive(InputObject, Clone)]
pub struct GqlExecutionContext {
    pub caller: Option<String>,
    pub transaction_id: Option<String>,
    pub data: std::collections::HashMap<String, Json<serde_json::Value>>,
}

impl From<GqlExecutionContext> for models::ExecutionContext {
    fn from(c: GqlExecutionContext) -> Self {
        Self {
            caller: c.caller,
            transaction_id: c.transaction_id,
            data: c.data.into_iter().map(|(k, Json(v))| (k, v)).collect(),
        }
    }
}

#[derive(InputObject, Clone)]
pub struct GqlExecuteDotInput {
    pub function: String,
    pub arguments: Vec<Json<serde_json::Value>>,
    pub context: Option<GqlExecutionContext>,
}

impl From<GqlExecuteDotInput> for models::ExecuteDotRequest {
    fn from(i: GqlExecuteDotInput) -> Self {
        Self {
            function: i.function,
            arguments: i.arguments.into_iter().map(|Json(v)| v).collect(),
            context: i.context.map(Into::into),
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlExecuteDotResponse {
    pub result: Json<serde_json::Value>,
    pub status: String,
    pub gas_used: u64,
    pub execution_time_ms: u64,
    pub transaction_id: Option<String>,
}

impl From<models::ExecuteDotResponse> for GqlExecuteDotResponse {
    fn from(r: models::ExecuteDotResponse) -> Self {
        Self {
            result: Json(r.result),
            status: format!("{:?}", r.status),
            gas_used: r.gas_used,
            execution_time_ms: r.execution_time_ms,
            transaction_id: r.transaction_id,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlWebSocketMessage {
    pub event_type: String,
    pub payload: Json<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl From<models::WebSocketMessage> for GqlWebSocketMessage {
    fn from(m: models::WebSocketMessage) -> Self {
        Self {
            event_type: m.event_type,
            payload: Json(m.payload),
            timestamp: m.timestamp,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct GqlSearchResults {
    pub documents: Vec<GqlDocument>,
    pub total_matches: u64,
    pub query: String,
    pub execution_time_ms: u64,
}

impl From<models::SearchResults> for GqlSearchResults {
    fn from(r: models::SearchResults) -> Self {
        Self {
            documents: r.documents.into_iter().map(GqlDocument::from).collect(),
            total_matches: r.total_matches,
            query: r.query,
            execution_time_ms: r.execution_time_ms,
        }
    }
}
