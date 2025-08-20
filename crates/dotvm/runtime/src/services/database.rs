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

use futures::Stream;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tonic::{Request, Response, Result as TonicResult, Status};

// Import proto from the main crate to avoid duplicate compilation
use crate::proto::database_service as proto;

use proto::{database_service_server::DatabaseService, *};

#[derive(Debug)]
pub struct DatabaseServiceImpl {
    // In-memory storage for demonstration
    // In production, this would be connected to actual database backend
    collections: Arc<RwLock<HashMap<String, Collection>>>,
}

#[derive(Debug, Clone)]
struct Collection {
    name: String,
    config: Option<CollectionConfig>,
    data: HashMap<String, Vec<u8>>,
    indices: HashMap<String, Index>,
}

#[derive(Debug, Clone)]
struct Index {
    name: String,
    fields: Vec<String>,
    index_type: String,
}

impl DatabaseServiceImpl {
    pub fn new() -> Self {
        Self {
            collections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_collection(&self, name: &str) -> Option<Collection> {
        let collections = self.collections.read().await;
        collections.get(name).cloned()
    }

    async fn ensure_collection_exists(&self, name: &str) -> Result<(), String> {
        let mut collections = self.collections.write().await;
        if !collections.contains_key(name) {
            let collection = Collection {
                name: name.to_string(),
                config: None,
                data: HashMap::new(),
                indices: HashMap::new(),
            };
            collections.insert(name.to_string(), collection);
        }
        Ok(())
    }
}

impl Default for DatabaseServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl DatabaseService for DatabaseServiceImpl {
    async fn get(&self, request: Request<GetRequest>) -> TonicResult<Response<GetResponse>> {
        let req = request.into_inner();

        if let Some(collection) = self.get_collection(&req.collection).await {
            if let Some(value) = collection.data.get(&req.key) {
                let response = GetResponse {
                    success: true,
                    value: value.clone(),
                    error_message: String::new(),
                };
                Ok(Response::new(response))
            } else {
                let response = GetResponse {
                    success: false,
                    value: vec![],
                    error_message: format!("Key '{}' not found in collection '{}'", req.key, req.collection),
                };
                Ok(Response::new(response))
            }
        } else {
            let response = GetResponse {
                success: false,
                value: vec![],
                error_message: format!("Collection '{}' not found", req.collection),
            };
            Ok(Response::new(response))
        }
    }

    async fn put(&self, request: Request<PutRequest>) -> TonicResult<Response<PutResponse>> {
        let req = request.into_inner();

        self.ensure_collection_exists(&req.collection).await.map_err(|e| Status::internal(e))?;

        {
            let mut collections = self.collections.write().await;
            if let Some(collection) = collections.get_mut(&req.collection) {
                collection.data.insert(req.key.clone(), req.value);

                tracing::debug!(
                    collection = %req.collection,
                    key = %req.key,
                    "Data stored successfully"
                );
            }
        }

        let response = PutResponse {
            success: true,
            error_message: String::new(),
        };
        Ok(Response::new(response))
    }

    async fn delete(&self, request: Request<DeleteRequest>) -> TonicResult<Response<DeleteResponse>> {
        let req = request.into_inner();

        let mut collections = self.collections.write().await;
        if let Some(collection) = collections.get_mut(&req.collection) {
            if collection.data.remove(&req.key).is_some() {
                let response = DeleteResponse {
                    success: true,
                    error_message: String::new(),
                };
                Ok(Response::new(response))
            } else {
                let response = DeleteResponse {
                    success: false,
                    error_message: format!("Key '{}' not found in collection '{}'", req.key, req.collection),
                };
                Ok(Response::new(response))
            }
        } else {
            let response = DeleteResponse {
                success: false,
                error_message: format!("Collection '{}' not found", req.collection),
            };
            Ok(Response::new(response))
        }
    }

    async fn batch_operation(&self, request: Request<BatchOperationRequest>) -> TonicResult<Response<BatchOperationResponse>> {
        let req = request.into_inner();

        self.ensure_collection_exists(&req.collection).await.map_err(|e| Status::internal(e))?;

        let mut results = Vec::new();

        {
            let mut collections = self.collections.write().await;
            if let Some(collection) = collections.get_mut(&req.collection) {
                for operation in req.operations {
                    let result = match operation.r#type {
                        0 => {
                            // GET
                            if let Some(value) = collection.data.get(&operation.key) {
                                OperationResult {
                                    success: true,
                                    value: value.clone(),
                                    error_message: String::new(),
                                }
                            } else {
                                OperationResult {
                                    success: false,
                                    value: vec![],
                                    error_message: format!("Key '{}' not found", operation.key),
                                }
                            }
                        }
                        1 => {
                            // PUT
                            collection.data.insert(operation.key.clone(), operation.value);
                            OperationResult {
                                success: true,
                                value: vec![],
                                error_message: String::new(),
                            }
                        }
                        2 => {
                            // DELETE
                            if collection.data.remove(&operation.key).is_some() {
                                OperationResult {
                                    success: true,
                                    value: vec![],
                                    error_message: String::new(),
                                }
                            } else {
                                OperationResult {
                                    success: false,
                                    value: vec![],
                                    error_message: format!("Key '{}' not found", operation.key),
                                }
                            }
                        }
                        _ => OperationResult {
                            success: false,
                            value: vec![],
                            error_message: "Unknown operation type".to_string(),
                        },
                    };
                    results.push(result);
                }
            }
        }

        let response = BatchOperationResponse {
            success: true,
            results,
            error_message: String::new(),
        };
        Ok(Response::new(response))
    }

    async fn create_collection(&self, request: Request<CreateCollectionRequest>) -> TonicResult<Response<CreateCollectionResponse>> {
        let req = request.into_inner();

        let mut collections = self.collections.write().await;
        if collections.contains_key(&req.name) {
            let response = CreateCollectionResponse {
                success: false,
                error_message: format!("Collection '{}' already exists", req.name),
            };
            return Ok(Response::new(response));
        }

        let collection = Collection {
            name: req.name.clone(),
            config: req.config,
            data: HashMap::new(),
            indices: HashMap::new(),
        };
        collections.insert(req.name.clone(), collection);

        tracing::info!(collection_name = %req.name, "Collection created");

        let response = CreateCollectionResponse {
            success: true,
            error_message: String::new(),
        };
        Ok(Response::new(response))
    }

    async fn list_collections(&self, request: Request<ListCollectionsRequest>) -> TonicResult<Response<ListCollectionsResponse>> {
        let req = request.into_inner();

        let collections_guard = self.collections.read().await;
        let mut collections = Vec::new();

        for (name, collection) in collections_guard.iter() {
            if req.pattern.is_empty() || name.contains(&req.pattern) {
                let collection_info = CollectionInfo {
                    name: name.clone(),
                    config: collection.config.clone(),
                    stats: Some(CollectionStats {
                        document_count: collection.data.len() as u64,
                        size_bytes: collection.data.values().map(|v| v.len() as u64).sum(),
                        index_count: collection.indices.len() as u64,
                    }),
                };
                collections.push(collection_info);
            }
        }

        let response = ListCollectionsResponse { collections };
        Ok(Response::new(response))
    }

    async fn drop_collection(&self, request: Request<DropCollectionRequest>) -> TonicResult<Response<DropCollectionResponse>> {
        let req = request.into_inner();

        let mut collections = self.collections.write().await;
        if collections.remove(&req.name).is_some() {
            tracing::info!(collection_name = %req.name, "Collection dropped");
            let response = DropCollectionResponse {
                success: true,
                error_message: String::new(),
            };
            Ok(Response::new(response))
        } else {
            let response = DropCollectionResponse {
                success: false,
                error_message: format!("Collection '{}' not found", req.name),
            };
            Ok(Response::new(response))
        }
    }

    async fn query(&self, request: Request<QueryRequest>) -> TonicResult<Response<QueryResponse>> {
        let req = request.into_inner();

        if let Some(collection) = self.get_collection(&req.collection).await {
            // Simple query implementation - just return all data for now
            let mut results = Vec::new();

            for (key, value) in &collection.data {
                let query_result = QueryResult {
                    key: key.clone(),
                    value: value.clone(),
                    metadata: HashMap::new(),
                };
                results.push(query_result);
            }

            let response = QueryResponse {
                success: true,
                results,
                total_count: collection.data.len() as u32,
                has_more: false,
                error_message: String::new(),
            };
            Ok(Response::new(response))
        } else {
            let response = QueryResponse {
                success: false,
                results: vec![],
                total_count: 0,
                has_more: false,
                error_message: format!("Collection '{}' not found", req.collection),
            };
            Ok(Response::new(response))
        }
    }

    type StreamQueryStream = Pin<Box<dyn Stream<Item = Result<QueryResponse, Status>> + Send>>;

    async fn stream_query(&self, request: Request<QueryRequest>) -> TonicResult<Response<Self::StreamQueryStream>> {
        // For now, just return empty stream
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }

    async fn create_index(&self, request: Request<CreateIndexRequest>) -> TonicResult<Response<CreateIndexResponse>> {
        let req = request.into_inner();

        let response = CreateIndexResponse {
            success: false,
            error_message: "CreateIndex not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn list_indices(&self, request: Request<ListIndicesRequest>) -> TonicResult<Response<ListIndicesResponse>> {
        let response = ListIndicesResponse { indices: vec![] };
        Ok(Response::new(response))
    }

    async fn drop_index(&self, request: Request<DropIndexRequest>) -> TonicResult<Response<DropIndexResponse>> {
        let response = DropIndexResponse {
            success: false,
            error_message: "DropIndex not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn begin_transaction(&self, request: Request<BeginTransactionRequest>) -> TonicResult<Response<BeginTransactionResponse>> {
        let response = BeginTransactionResponse {
            success: false,
            transaction_id: String::new(),
            error_message: "Transactions not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn commit_transaction(&self, request: Request<CommitTransactionRequest>) -> TonicResult<Response<CommitTransactionResponse>> {
        let response = CommitTransactionResponse {
            success: false,
            error_message: "Transactions not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn rollback_transaction(&self, request: Request<RollbackTransactionRequest>) -> TonicResult<Response<RollbackTransactionResponse>> {
        let response = RollbackTransactionResponse {
            success: false,
            error_message: "Transactions not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn get_database_status(&self, request: Request<GetDatabaseStatusRequest>) -> TonicResult<Response<GetDatabaseStatusResponse>> {
        let collections_count = self.collections.read().await.len();

        let info = DatabaseInfo {
            version: "1.0.0".to_string(),
            uptime_seconds: 3600, // Mock value
            collection_count: collections_count as u32,
            total_size_bytes: 1024 * 1024, // Mock value
            metrics: Some(DatabaseMetrics {
                reads_per_second: 100,
                writes_per_second: 50,
                average_query_time_ms: 10.0,
                active_connections: 5,
                cache_hit_rate: 95,
            }),
        };

        let response = GetDatabaseStatusResponse {
            status: 2, // Running
            info: Some(info),
        };
        Ok(Response::new(response))
    }

    async fn get_database_metrics(&self, request: Request<GetDatabaseMetricsRequest>) -> TonicResult<Response<GetDatabaseMetricsResponse>> {
        let mock_metric = DatabaseMetric {
            name: "query_latency".to_string(),
            r#type: "histogram".to_string(),
            data_points: vec![MetricDataPoint {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                value: 10.5,
            }],
            labels: HashMap::new(),
        };

        let response = GetDatabaseMetricsResponse { metrics: vec![mock_metric] };
        Ok(Response::new(response))
    }

    async fn ping(&self, request: Request<PingRequest>) -> TonicResult<Response<PingResponse>> {
        let req = request.into_inner();

        let response = PingResponse {
            server_id: "database-server-001".to_string(),
            timestamp: req.timestamp,
            server_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        Ok(Response::new(response))
    }

    async fn health_check(&self, request: Request<HealthCheckRequest>) -> TonicResult<Response<HealthCheckResponse>> {
        let req = request.into_inner();

        let service_health = vec![ServiceHealth {
            service_name: "database_service".to_string(),
            status: 1, // HEALTH_SERVING
            message: "Database service is healthy".to_string(),
            details: HashMap::new(),
        }];

        let mut system_info = HashMap::new();
        if req.include_details {
            system_info.insert("collections_count".to_string(), self.collections.read().await.len().to_string());
        }

        let response = HealthCheckResponse {
            overall_status: 1, // HEALTH_SERVING
            service_health,
            system_info,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        Ok(Response::new(response))
    }
}
