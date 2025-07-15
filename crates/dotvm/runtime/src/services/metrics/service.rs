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

//! Metrics service implementation

use std::sync::Arc;
use tonic::{Request, Response, Result as TonicResult, Status};
use tracing::{error, info, instrument};

use crate::proto::vm_service::{GetVmMetricsRequest, GetVmMetricsResponse, MetricDataPoint, VmMetric};

use super::collector::MetricsCollector;

/// Metrics service handles all metrics-related operations
pub struct MetricsService {
    collector: Arc<MetricsCollector>,
}

impl MetricsService {
    pub fn new() -> Self {
        Self {
            collector: Arc::new(MetricsCollector::new()),
        }
    }

    #[instrument(skip(self, request))]
    pub async fn get_vm_metrics(&self, request: Request<GetVmMetricsRequest>) -> TonicResult<Response<GetVmMetricsResponse>> {
        let req = request.into_inner();

        info!("Getting VM metrics");

        let result = self.collector.collect_metrics(req).await.map_err(|e| Status::internal(format!("Failed to collect metrics: {}", e)))?;

        Ok(Response::new(result))
    }
}
