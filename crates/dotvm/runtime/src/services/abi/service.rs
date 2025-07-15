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

//! ABI service implementation

use std::sync::Arc;
use tonic::{Request, Response, Result as TonicResult, Status};
use tracing::{error, info, instrument};

use crate::proto::vm_service::{GenerateAbiRequest, GenerateAbiResponse, GetDotAbiRequest, GetDotAbiResponse, RegisterAbiRequest, RegisterAbiResponse, ValidateAbiRequest, ValidateAbiResponse};

use super::generator::AbiGenerator;
use super::registry::AbiRegistry;
use super::validator::AbiValidator;

/// ABI service handles all ABI-related operations
pub struct AbiService {
    generator: Arc<AbiGenerator>,
    validator: Arc<AbiValidator>,
    registry: Arc<AbiRegistry>,
}

impl AbiService {
    pub fn new() -> Self {
        Self {
            generator: Arc::new(AbiGenerator::new()),
            validator: Arc::new(AbiValidator::new()),
            registry: Arc::new(AbiRegistry::new()),
        }
    }

    #[instrument(skip(self, request))]
    pub async fn get_dot_abi(&self, request: Request<GetDotAbiRequest>) -> TonicResult<Response<GetDotAbiResponse>> {
        let req = request.into_inner();

        info!("Getting ABI for dot: {}", req.dot_id);

        let result = self
            .registry
            .get_abi(&req.dot_id, if req.version.is_empty() { None } else { Some(req.version.as_str()) })
            .await
            .map_err(|e| Status::not_found(format!("ABI not found: {}", e)))?;

        Ok(Response::new(result))
    }

    #[instrument(skip(self, request))]
    pub async fn validate_abi(&self, request: Request<ValidateAbiRequest>) -> TonicResult<Response<ValidateAbiResponse>> {
        let req = request.into_inner();

        info!("Validating ABI");

        let result = self.validator.validate_abi(req).await.map_err(|e| Status::internal(format!("Validation failed: {}", e)))?;

        Ok(Response::new(result))
    }

    #[instrument(skip(self, request))]
    pub async fn generate_abi(&self, request: Request<GenerateAbiRequest>) -> TonicResult<Response<GenerateAbiResponse>> {
        let req = request.into_inner();

        info!("Generating ABI from dot source ({} chars)", req.dot_source.len());

        let result = self.generator.generate_from_source(req).await.map_err(|e| Status::internal(format!("Generation failed: {}", e)))?;

        Ok(Response::new(result))
    }

    #[instrument(skip(self, request))]
    pub async fn register_abi(&self, request: Request<RegisterAbiRequest>) -> TonicResult<Response<RegisterAbiResponse>> {
        let req = request.into_inner();

        info!("Registering ABI for dot: {}", req.dot_id);

        let result = self.registry.register_abi(req).await.map_err(|e| Status::internal(format!("Registration failed: {}", e)))?;

        Ok(Response::new(result))
    }
}
