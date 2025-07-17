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

//! gRPC middleware implementations

pub mod auth;
pub mod compression;
pub mod connection_pool;
pub mod rate_limit;
pub mod tracing;
pub mod security;

pub use auth::{AuthInterceptor, JwtValidator};
pub use compression::CompressionLayer;
pub use connection_pool::{ConnectionPool, ConnectionPoolConfig};
pub use rate_limit::{RateLimitInterceptor, RateLimitConfig};
pub use tracing::TracingInterceptor;