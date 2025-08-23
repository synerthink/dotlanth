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

//! HTTP server implementation using Hyper

use crate::auth::AuthService;
use crate::config::Config;
use crate::db::DatabaseClient;
use crate::error::{ApiError, ApiResult};
use crate::middleware::VersioningMiddleware;
use crate::router::Router;
use crate::security::{SecurityConfig, SecurityLayer};
use crate::versioning::{CompatibilityChecker, DeprecationManager, SchemaEvolutionManager, VersionRegistry};
use crate::vm::VmClient;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tracing::{error, info};

/// API server using Hyper
pub struct ApiServer {
    config: Config,
    bind_address: SocketAddr,
    router: Arc<Router>,
    auth_service: Arc<Mutex<AuthService>>,
    db_client: DatabaseClient,
    vm_client: VmClient,
    versioning_middleware: Arc<VersioningMiddleware>,
}

impl ApiServer {
    /// Create a new API server
    pub async fn new(config: Config) -> ApiResult<Self> {
        // Parse bind address
        let bind_address: SocketAddr = config.bind_address.parse().map_err(|e| ApiError::BadRequest {
            message: format!("Invalid bind address: {}", e),
        })?;

        // Create authentication service
        let auth_service = Arc::new(Mutex::new(AuthService::new(&config.jwt_secret)?));

        // Create database client
        let db_client = DatabaseClient::new(&config.db_service_address)?;

        // Create VM client
        let vm_client = VmClient::new(&config.vm_service_address).await?;

        // Initialize versioning components
        let version_registry = VersionRegistry::new();
        let compatibility_checker = CompatibilityChecker::new();
        let deprecation_manager = DeprecationManager::default();
        let schema_manager = SchemaEvolutionManager::new();

        // Create versioning middleware
        let versioning_middleware = Arc::new(VersioningMiddleware::new(version_registry, compatibility_checker, deprecation_manager, schema_manager));

        // Create router
        let router = Arc::new(Router::new(auth_service.clone(), db_client.clone(), vm_client.clone()).await?);

        info!("API server created successfully with versioning support");

        Ok(Self {
            config,
            bind_address,
            router,
            auth_service,
            db_client,
            vm_client,
            versioning_middleware,
        })
    }

    /// Get the bind address
    pub fn bind_address(&self) -> SocketAddr {
        self.bind_address
    }

    /// Start the server
    pub async fn run(self) -> ApiResult<()> {
        // Create TCP listener
        let listener = TcpListener::bind(self.bind_address).await.map_err(|e| ApiError::IoError(e))?;

        info!("Dotlanth REST API Gateway listening on http://{}", self.bind_address);
        info!("OpenAPI documentation available at http://{}/docs", self.bind_address);

        // Create security configuration
        let security_config = SecurityConfig {
            enable_sanitization: true,
            enable_security_headers: true,
            enable_ddos_protection: true,
            enable_api_keys: true,
            enable_request_size_limiting: true,
            max_body_size: self.config.max_body_size,
            rate_limit_config: crate::rate_limiting::RateLimitConfig {
                max_requests: 100,
                window: std::time::Duration::from_secs(60),
                algorithm: crate::rate_limiting::RateLimitAlgorithm::SlidingWindow,
                per_ip: true,
                per_user: false,
                per_api_key: false,
            },
            authenticated_rate_limit_config: crate::rate_limiting::RateLimitConfig {
                max_requests: 1000,
                window: std::time::Duration::from_secs(60),
                algorithm: crate::rate_limiting::RateLimitAlgorithm::SlidingWindow,
                per_ip: false,
                per_user: true,
                per_api_key: true,
            },
            ddos_threshold: 1000,
            ddos_window: std::time::Duration::from_secs(1),
        };

        // Create security layer
        let security_layer = SecurityLayer::new(security_config, self.auth_service.clone());

        // Accept connections
        loop {
            let (stream, remote_addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            let io = TokioIo::new(stream);
            let router = self.router.clone();
            //let security_layer = security_layer.clone();

            // Spawn a task to handle the connection
            tokio::task::spawn(async move {
                // Create service with middleware
                let service = ServiceBuilder::new()
                    //.layer(security_layer)
                    .service(service_fn(move |req: Request<Incoming>| {
                        let router = router.clone();
                        async move {
                            match router.route(req).await {
                                Ok(response) => Ok::<_, Infallible>(response),
                                Err(e) => {
                                    error!("Request failed: {}", e);
                                    Ok(Response::from(e))
                                }
                            }
                        }
                    }));

                // Serve the connection
                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    error!("Error serving connection from {}: {}", remote_addr, err);
                }
            });
        }
    }
}
