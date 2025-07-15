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

//! Dots service implementation

use std::sync::Arc;
use tonic::{Request, Response, Result as TonicResult, Status};
use tracing::{error, info, instrument};

use crate::proto::vm_service::{
    DeleteDotRequest,
    DeleteDotResponse,
    DeployDotRequest,
    DeployDotResponse,
    DeploymentMetrics,
    // Types
    DotInfo,
    DotMetadata,
    DotStats,
    DotStatus,
    ExecuteDotRequest,
    ExecuteDotResponse,
    ExecutionMetrics,
    GetDotStateRequest,
    GetDotStateResponse,
    ListDotsRequest,
    ListDotsResponse,
    LogEntry,
};

use super::executor::DotExecutor;
use super::registry::DotRegistry;

/// Dots service handles all dot-related operations
pub struct DotsService {
    registry: Arc<DotRegistry>,
    executor: Arc<DotExecutor>,
}

impl DotsService {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(DotRegistry::new()),
            executor: Arc::new(DotExecutor::new()),
        }
    }

    #[instrument(skip(self, request))]
    pub async fn execute_dot(&self, request: Request<ExecuteDotRequest>) -> TonicResult<Response<ExecuteDotResponse>> {
        let req = request.into_inner();

        info!("Executing dot: {}", req.dot_id);

        // Validate request
        if req.dot_id.is_empty() {
            return Err(Status::invalid_argument("dot_id cannot be empty"));
        }

        // Get dot from registry
        let dot_info = self.registry.get_dot(&req.dot_id).await.map_err(|e| Status::not_found(format!("Dot not found: {}", e)))?;

        // Execute dot
        let result = self.executor.execute(&dot_info, req).await.map_err(|e| Status::internal(format!("Execution failed: {}", e)))?;

        Ok(Response::new(result))
    }

    #[instrument(skip(self, request))]
    pub async fn deploy_dot(&self, request: Request<DeployDotRequest>) -> TonicResult<Response<DeployDotResponse>> {
        let req = request.into_inner();

        info!("Deploying dot: {}", req.dot_name);

        // Validate request
        if req.dot_name.is_empty() {
            return Err(Status::invalid_argument("dot_name cannot be empty"));
        }

        if req.dot_source.is_empty() {
            return Err(Status::invalid_argument("dot_source cannot be empty"));
        }

        // Deploy dot
        let result = self.registry.deploy_dot(req).await.map_err(|e| Status::internal(format!("Deployment failed: {}", e)))?;

        Ok(Response::new(result))
    }

    #[instrument(skip(self, request))]
    pub async fn list_dots(&self, request: Request<ListDotsRequest>) -> TonicResult<Response<ListDotsResponse>> {
        let req = request.into_inner();

        info!("Listing dots");

        let result = self.registry.list_dots(req).await.map_err(|e| Status::internal(format!("Failed to list dots: {}", e)))?;

        Ok(Response::new(result))
    }

    #[instrument(skip(self, request))]
    pub async fn delete_dot(&self, request: Request<DeleteDotRequest>) -> TonicResult<Response<DeleteDotResponse>> {
        let req = request.into_inner();

        info!("Deleting dot: {}", req.dot_id);

        let result = self.registry.delete_dot(req).await.map_err(|e| Status::internal(format!("Failed to delete dot: {}", e)))?;

        Ok(Response::new(result))
    }

    #[instrument(skip(self, request))]
    pub async fn get_dot_state(&self, request: Request<GetDotStateRequest>) -> TonicResult<Response<GetDotStateResponse>> {
        let req = request.into_inner();

        info!("Getting state for dot: {}", req.dot_id);

        let result = self.executor.get_state(req).await.map_err(|e| Status::internal(format!("Failed to get state: {}", e)))?;

        Ok(Response::new(result))
    }
}
