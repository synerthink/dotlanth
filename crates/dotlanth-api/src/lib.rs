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

//! Dotlanth REST API Gateway
//!
//! This crate provides a REST API gateway that integrates with DotVM and DotDB
//! through gRPC services, offering HTTP/REST endpoints for web clients.

pub mod auth;
pub mod compatibility_testing;
pub mod config;
pub mod db;
pub mod error;
pub mod gateway;
pub mod graphql;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod rate_limiting;
pub mod router;
pub mod security;
pub mod server;
pub mod versioning;
pub mod vm;
pub mod websocket;
