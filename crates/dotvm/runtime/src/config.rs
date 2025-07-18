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

//! Runtime configuration for gRPC server

use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub bind_address: SocketAddr,
    pub enable_reflection: bool,
    pub enable_health_check: bool,
    pub max_connections: u32,
    pub connection_timeout_ms: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:50051".parse().unwrap(),
            enable_reflection: true,
            enable_health_check: true,
            max_connections: 1000,
            connection_timeout_ms: 30000,
        }
    }
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Allow override via environment variables
        if let Ok(addr_str) = std::env::var("GRPC_BIND_ADDR") {
            if let Ok(addr) = SocketAddr::from_str(&addr_str) {
                config.bind_address = addr;
            } else {
                eprintln!("Warning: Invalid GRPC_BIND_ADDR '{}', using default", addr_str);
            }
        }
        
        if let Ok(max_conn_str) = std::env::var("GRPC_MAX_CONNECTIONS") {
            if let Ok(max_conn) = max_conn_str.parse::<u32>() {
                config.max_connections = max_conn;
            }
        }
        
        if let Ok(timeout_str) = std::env::var("GRPC_CONNECTION_TIMEOUT_MS") {
            if let Ok(timeout) = timeout_str.parse::<u64>() {
                config.connection_timeout_ms = timeout;
            }
        }
        
        config
    }
    
    pub fn get_bind_address_for_platform(&self) -> SocketAddr {
        // Cross-platform binding strategy
        let host = if cfg!(target_os = "linux") {
            // On Linux, prefer 127.0.0.1 to avoid IPv6 issues
            "127.0.0.1"
        } else if cfg!(target_os = "macos") {
            // On macOS, 127.0.0.1 works reliably
            "127.0.0.1"
        } else if cfg!(target_os = "windows") {
            // On Windows, 127.0.0.1 is most compatible
            "127.0.0.1"
        } else {
            // Default fallback
            "127.0.0.1"
        };
        
        format!("{}:{}", host, self.bind_address.port())
            .parse()
            .unwrap_or(self.bind_address)
    }
}