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

//! gRPC-HTTP Gateway Bridge Module
//!
//! This module provides bidirectional transcoding between gRPC services and HTTP REST API,
//! enabling protocol flexibility and seamless integration between different client types.

pub mod bridge;
pub mod config;
pub mod error_mapping;
pub mod protocol_negotiation;
pub mod request_transformer;
pub mod response_transformer;
pub mod streaming_bridge;
pub mod transcoder;
pub mod websocket_grpc_bridge;

pub use bridge::Bridge;
pub use config::{GatewayBridge, GatewayConfig, GatewayMetrics};
pub use error_mapping::ErrorMapper;
pub use protocol_negotiation::{ContentType, Encoding, ProtocolNegotiator};
pub use request_transformer::RequestTransformer;
pub use response_transformer::ResponseTransformer;
pub use streaming_bridge::{StreamingBridge, StreamingConnection, StreamingMetrics};
pub use transcoder::{GrpcHttpTranscoder, ServiceMethod, TranscodingContext};
pub use websocket_grpc_bridge::{WebSocketConnection, WebSocketGrpcBridge, WebSocketGrpcMessage};
