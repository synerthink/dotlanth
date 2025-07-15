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

use proto::runtime_server::{Runtime, RuntimeServer};
use std::sync::Arc;
use tonic::transport::Server;

// Import our services
mod services;
use services::VmServiceImpl;

mod proto {
    tonic::include_proto!("runtime");

    pub mod vm_service {
        tonic::include_proto!("vm_service");
    }

    // TODO: Add database_service when ready
    // pub mod database_service {
    //     tonic::include_proto!("database_service");
    // }

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("runtime_descriptor");
}

#[derive(Debug, Default)]
struct RuntimeService {}

#[tonic::async_trait]
impl Runtime for RuntimeService {
    async fn ping(&self, request: tonic::Request<proto::PingRequest>) -> Result<tonic::Response<proto::PingResponse>, tonic::Status> {
        let response = proto::PingResponse {
            message: format!("Ping: {}", request.into_inner().message),
        };

        Ok(tonic::Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let addr = "[::1]:50051".parse()?;

    // Create the original runtime service
    let runtime = RuntimeService::default();

    // Create the new VM service
    let vm_service = VmServiceImpl::new();

    // Set up reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build()?;

    println!("Starting Dotlanth Runtime Server on {}", addr);

    // Start the server with both services
    Server::builder()
        .add_service(reflection_service)
        .add_service(RuntimeServer::new(runtime))
        .add_service(proto::vm_service::vm_service_server::VmServiceServer::new(vm_service))
        .serve(addr)
        .await?;

    Ok(())
}

// TODO: Add mock implementations when needed
