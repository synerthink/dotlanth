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

//! Core transcoding functionality between gRPC and HTTP protocols

use super::GatewayConfig;
use super::error_mapping::ErrorMapper;
use super::protocol_negotiation::ProtocolNegotiator;
use super::request_transformer::RequestTransformer;
use super::response_transformer::ResponseTransformer;
use crate::error::{ApiError, ApiResult};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use tonic::transport::Channel;
use tracing::{debug, error, info, warn};

/// gRPC service method descriptor
#[derive(Debug, Clone)]
pub struct ServiceMethod {
    pub service_name: String,
    pub method_name: String,
    pub input_type: String,
    pub output_type: String,
    pub is_streaming: bool,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
}

/// HTTP to gRPC transcoding context
#[derive(Debug)]
pub struct TranscodingContext {
    pub service_method: ServiceMethod,
    pub http_method: Method,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub headers: HeaderMap,
    pub content_type: String,
    pub accept_type: String,
}

/// Main gRPC-HTTP transcoder
pub struct GrpcHttpTranscoder {
    config: GatewayConfig,
    error_mapper: ErrorMapper,
    protocol_negotiator: ProtocolNegotiator,
    request_transformer: RequestTransformer,
    response_transformer: ResponseTransformer,
    service_registry: HashMap<String, ServiceMethod>,
}

impl GrpcHttpTranscoder {
    /// Create a new transcoder
    pub fn new(config: GatewayConfig) -> ApiResult<Self> {
        let error_mapper = ErrorMapper::new();
        let protocol_negotiator = ProtocolNegotiator::new();
        let request_transformer = RequestTransformer::new();
        let response_transformer = ResponseTransformer::new();
        let service_registry = Self::build_service_registry();

        Ok(Self {
            config,
            error_mapper,
            protocol_negotiator,
            request_transformer,
            response_transformer,
            service_registry,
        })
    }

    /// Build the service registry with all available gRPC methods
    fn build_service_registry() -> HashMap<String, ServiceMethod> {
        let mut registry = HashMap::new();

        // VM Service methods
        registry.insert(
            "/api/v1/vm/execute".to_string(),
            ServiceMethod {
                service_name: "vm_service.VmService".to_string(),
                method_name: "ExecuteDot".to_string(),
                input_type: "ExecuteDotRequest".to_string(),
                output_type: "ExecuteDotResponse".to_string(),
                is_streaming: false,
                is_client_streaming: false,
                is_server_streaming: false,
            },
        );

        registry.insert(
            "/api/v1/vm/deploy".to_string(),
            ServiceMethod {
                service_name: "vm_service.VmService".to_string(),
                method_name: "DeployDot".to_string(),
                input_type: "DeployDotRequest".to_string(),
                output_type: "DeployDotResponse".to_string(),
                is_streaming: false,
                is_client_streaming: false,
                is_server_streaming: false,
            },
        );

        registry.insert(
            "/api/v1/vm/dots".to_string(),
            ServiceMethod {
                service_name: "vm_service.VmService".to_string(),
                method_name: "ListDots".to_string(),
                input_type: "ListDotsRequest".to_string(),
                output_type: "ListDotsResponse".to_string(),
                is_streaming: false,
                is_client_streaming: false,
                is_server_streaming: false,
            },
        );

        registry.insert(
            "/api/v1/vm/stream".to_string(),
            ServiceMethod {
                service_name: "vm_service.VmService".to_string(),
                method_name: "StreamExecution".to_string(),
                input_type: "StreamExecutionRequest".to_string(),
                output_type: "StreamExecutionResponse".to_string(),
                is_streaming: true,
                is_client_streaming: false,
                is_server_streaming: true,
            },
        );

        // Database Service methods
        registry.insert(
            "/api/v1/db/documents".to_string(),
            ServiceMethod {
                service_name: "database_service.DatabaseService".to_string(),
                method_name: "CreateDocument".to_string(),
                input_type: "CreateDocumentRequest".to_string(),
                output_type: "CreateDocumentResponse".to_string(),
                is_streaming: false,
                is_client_streaming: false,
                is_server_streaming: false,
            },
        );

        registry.insert(
            "/api/v1/db/query".to_string(),
            ServiceMethod {
                service_name: "database_service.DatabaseService".to_string(),
                method_name: "QueryDocuments".to_string(),
                input_type: "QueryDocumentsRequest".to_string(),
                output_type: "QueryDocumentsResponse".to_string(),
                is_streaming: false,
                is_client_streaming: false,
                is_server_streaming: false,
            },
        );

        registry.insert(
            "/api/v1/db/stream".to_string(),
            ServiceMethod {
                service_name: "database_service.DatabaseService".to_string(),
                method_name: "StreamChanges".to_string(),
                input_type: "StreamChangesRequest".to_string(),
                output_type: "StreamChangesResponse".to_string(),
                is_streaming: true,
                is_client_streaming: false,
                is_server_streaming: true,
            },
        );

        registry
    }

    /// Transcode HTTP request to gRPC call
    pub async fn transcode_http_to_grpc(&self, req: Request<Incoming>, grpc_channel: Channel) -> ApiResult<Response<Full<Bytes>>> {
        let start_time = Instant::now();

        debug!("Starting HTTP to gRPC transcoding for: {}", req.uri());

        // Parse the request and determine the target gRPC method
        let context = self.parse_http_request(&req).await?;

        // Validate the request
        if self.config.enable_validation {
            self.validate_request(&context)?;
        }

        // Transform HTTP request to gRPC request
        let grpc_request = self.request_transformer.transform_to_grpc(&req, &context).await?;

        // Execute the gRPC call
        let grpc_response = self.execute_grpc_call(grpc_request, &context, grpc_channel).await?;

        // Transform gRPC response to HTTP response
        let http_response = self.response_transformer.transform_to_http(grpc_response, &context).await?;

        let latency = start_time.elapsed().as_millis() as f64;
        debug!("HTTP to gRPC transcoding completed in {}ms", latency);

        Ok(http_response)
    }

    /// Transcode gRPC request to HTTP call
    pub async fn transcode_grpc_to_http(
        &self,
        grpc_request: tonic::Request<Value>,
        http_client: &hyper_util::client::legacy::Client<hyper_util::client::legacy::connect::HttpConnector, http_body_util::Full<hyper::body::Bytes>>,
        target_url: &str,
    ) -> ApiResult<tonic::Response<Value>> {
        let start_time = Instant::now();

        debug!("Starting gRPC to HTTP transcoding for: {}", target_url);

        // Transform gRPC request to HTTP request
        let http_request = self.request_transformer.transform_to_http(grpc_request, target_url).await?;

        // Execute the HTTP call
        let http_response = http_client.request(http_request).await.map_err(|e| ApiError::ServiceUnavailable {
            message: format!("HTTP target error: {}", e),
        })?;

        // Transform HTTP response to gRPC response
        let grpc_response = self.response_transformer.transform_to_grpc(http_response).await?;

        let latency = start_time.elapsed().as_millis() as f64;
        debug!("gRPC to HTTP transcoding completed in {}ms", latency);

        Ok(grpc_response)
    }

    /// Parse HTTP request and extract transcoding context
    async fn parse_http_request(&self, req: &Request<Incoming>) -> ApiResult<TranscodingContext> {
        let path = req.uri().path().to_string();
        let method = req.method().clone();
        let headers = req.headers().clone();

        // Find matching service method
        let service_method = self
            .service_registry
            .get(&path)
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Service method for path: {}", path),
            })?
            .clone();

        // Parse query parameters
        let query_params = req.uri().query().map(|q| url::form_urlencoded::parse(q.as_bytes()).into_owned().collect()).unwrap_or_default();

        // Negotiate content types
        let (content_type, accept_type) = self.protocol_negotiator.negotiate_content_types(&headers)?;

        Ok(TranscodingContext {
            service_method,
            http_method: method,
            path,
            query_params,
            headers,
            content_type,
            accept_type,
        })
    }

    /// Validate the transcoding request
    fn validate_request(&self, context: &TranscodingContext) -> ApiResult<()> {
        // Validate HTTP method compatibility
        match context.http_method {
            Method::GET => {
                if context.service_method.method_name.contains("Create") || context.service_method.method_name.contains("Update") || context.service_method.method_name.contains("Delete") {
                    return Err(ApiError::BadRequest {
                        message: format!("GET method not allowed for {}", context.service_method.method_name),
                    });
                }
            }
            Method::POST => {
                // POST is generally allowed for all operations
            }
            Method::PUT => {
                if !context.service_method.method_name.contains("Update") && !context.service_method.method_name.contains("Create") {
                    return Err(ApiError::BadRequest {
                        message: format!("PUT method not allowed for {}", context.service_method.method_name),
                    });
                }
            }
            Method::DELETE => {
                if !context.service_method.method_name.contains("Delete") {
                    return Err(ApiError::BadRequest {
                        message: format!("DELETE method not allowed for {}", context.service_method.method_name),
                    });
                }
            }
            _ => {
                return Err(ApiError::BadRequest {
                    message: format!("Unsupported HTTP method: {}", context.http_method),
                });
            }
        }

        // Validate content type
        if !self.protocol_negotiator.is_supported_content_type(&context.content_type) {
            return Err(ApiError::BadRequest {
                message: format!("Unsupported content type: {}", context.content_type),
            });
        }

        Ok(())
    }

    /// Execute the actual gRPC call
    async fn execute_grpc_call(&self, grpc_request: tonic::Request<Value>, context: &TranscodingContext, mut channel: Channel) -> ApiResult<tonic::Response<Value>> {
        let service_name = &context.service_method.service_name;
        let method_name = &context.service_method.method_name;

        debug!("Executing gRPC call: {}/{}", service_name, method_name);

        // Create a generic gRPC client
        let mut client = tonic::client::Grpc::new(channel);

        // Set timeout
        let timeout = std::time::Duration::from_millis(self.config.max_timeout_ms);
        // Note: timeout method doesn't exist on Grpc client, we'll handle timeout at the call level

        // Execute the call based on the method type
        if context.service_method.is_streaming {
            return Err(ApiError::BadRequest {
                message: "Streaming methods should use WebSocket bridge".to_string(),
            });
        }

        // For unary calls, we need to use the appropriate service client
        // This is a simplified implementation - in practice, you'd use the generated clients
        match service_name.as_str() {
            "vm_service.VmService" => self.execute_vm_service_call(grpc_request, method_name, client).await,
            "database_service.DatabaseService" => self.execute_db_service_call(grpc_request, method_name, client).await,
            _ => Err(ApiError::BadRequest {
                message: format!("Unknown service: {}", service_name),
            }),
        }
    }

    /// Execute VM service gRPC call
    async fn execute_vm_service_call(&self, request: tonic::Request<Value>, method_name: &str, mut client: tonic::client::Grpc<Channel>) -> ApiResult<tonic::Response<Value>> {
        // This is a simplified implementation
        // In practice, you'd use the generated gRPC client stubs

        let response_data = match method_name {
            "ExecuteDot" => {
                serde_json::json!({
                    "execution_id": "exec_123",
                    "status": "completed",
                    "result": "execution successful"
                })
            }
            "DeployDot" => {
                serde_json::json!({
                    "dot_id": "dot_456",
                    "status": "deployed",
                    "address": "0x1234567890abcdef"
                })
            }
            "ListDots" => {
                serde_json::json!({
                    "dots": [
                        {
                            "id": "dot_1",
                            "name": "example_dot",
                            "status": "active"
                        }
                    ],
                    "total_count": 1
                })
            }
            _ => {
                return Err(ApiError::BadRequest {
                    message: format!("Unknown VM service method: {}", method_name),
                });
            }
        };

        let mut response = tonic::Response::new(response_data);
        response.metadata_mut().insert("content-type", "application/json".parse().unwrap());

        Ok(response)
    }

    /// Execute Database service gRPC call
    async fn execute_db_service_call(&self, request: tonic::Request<Value>, method_name: &str, mut client: tonic::client::Grpc<Channel>) -> ApiResult<tonic::Response<Value>> {
        // This is a simplified implementation
        // In practice, you'd use the generated gRPC client stubs

        let response_data = match method_name {
            "CreateDocument" => {
                serde_json::json!({
                    "document_id": "doc_789",
                    "status": "created",
                    "created_at": chrono::Utc::now().to_rfc3339()
                })
            }
            "QueryDocuments" => {
                serde_json::json!({
                    "documents": [
                        {
                            "id": "doc_1",
                            "data": {"key": "value"},
                            "created_at": chrono::Utc::now().to_rfc3339()
                        }
                    ],
                    "total_count": 1
                })
            }
            _ => {
                return Err(ApiError::BadRequest {
                    message: format!("Unknown Database service method: {}", method_name),
                });
            }
        };

        let mut response = tonic::Response::new(response_data);
        response.metadata_mut().insert("content-type", "application/json".parse().unwrap());

        Ok(response)
    }
}
