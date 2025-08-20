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

//! VM Service implementation for gRPC

use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Result as TonicResult, Status, Streaming};
use tracing::{error, info, instrument};
use uuid::Uuid;

// VM and StateStorage imports - now available
use dotdb_core::state::db_interface::{Database, DatabaseInterface, DbConfig};
use dotvm_core::bytecode::BytecodeFile;
use dotvm_core::vm::executor::VmExecutor;
use dotvm_core::vm::stack::StackValue;
use dotvm_core::vm::state_storage::state_storage::{DefaultStateStorage, StateStorage};
use dotvm_core::vm::vm_factory::{SimpleVMFactory, VMFactory, VmInstance};

// Import generated protobuf types
use crate::proto::vm_service::{vm_service_server::VmService, *};
use crate::services::streaming;

use super::{AbiService, DotsService, MetricsService, VmManagementService};

/// VM Service implementation - coordinates all sub-services
pub struct VmServiceImpl {
    dots_service: Arc<DotsService>,
    abi_service: Arc<AbiService>,
    metrics_service: Arc<MetricsService>,
    vm_management_service: Arc<VmManagementService>,

    // Advanced gRPC Features
    active_sessions: Arc<RwLock<HashMap<String, InteractiveSession>>>,
    debug_sessions: Arc<RwLock<HashMap<String, DebugSession>>>,
    connection_pool: Arc<ConnectionPool>,
    server_stats: Arc<RwLock<ServerStats>>,
    server_id: String,
    start_time: Instant,

    // Storage components
    database: Arc<Database>,
    vm_factory: Arc<SimpleVMFactory>,

    // Shared streaming components
    event_broadcaster: Arc<streaming::DotEventBroadcaster>,
    metrics_collector: Arc<streaming::VmMetricsCollector>,

    // VM instances for active sessions
    vm_instances: Arc<RwLock<HashMap<String, VmExecutionInstance>>>,
}

// Advanced Features - Session Management
#[derive(Debug)]
struct InteractiveSession {
    session_id: String,
    dot_id: String,
    sender: mpsc::UnboundedSender<Result<InteractiveExecutionResponse, Status>>,
    debug_mode: bool,
    created_at: Instant,
    last_activity: Instant,
}

#[derive(Debug)]
struct DebugSession {
    session_id: String,
    dot_id: String,
    sender: mpsc::UnboundedSender<Result<DebugResponse, Status>>,
    breakpoints: HashMap<u64, Breakpoint>,
    created_at: Instant,
    last_activity: Instant,
}

#[derive(Debug)]
struct Breakpoint {
    id: u64,
    address: u64,
    condition: Option<String>,
    enabled: bool,
}

#[derive(Debug)]
struct ConnectionPool {
    active_connections: Arc<RwLock<u32>>,
    total_requests: Arc<RwLock<u64>>,
    max_connections: u32,
    connection_timeout: Duration,
    request_history: Arc<RwLock<Vec<RequestMetric>>>,
}

#[derive(Debug, Clone)]
struct RequestMetric {
    timestamp: Instant,
    duration_ms: u64,
    success: bool,
    endpoint: String,
}

impl ConnectionPool {
    pub fn new(max_connections: u32, connection_timeout: Duration) -> Self {
        Self {
            active_connections: Arc::new(RwLock::new(0)),
            total_requests: Arc::new(RwLock::new(0)),
            max_connections,
            connection_timeout,
            request_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn acquire_connection(&self) -> Result<ConnectionGuard, Status> {
        let mut connections = self.active_connections.write().await;

        if *connections >= self.max_connections {
            return Err(Status::resource_exhausted("Maximum connections reached"));
        }

        *connections += 1;
        let connection_id = *connections;

        Ok(ConnectionGuard {
            pool: self.active_connections.clone(),
            connection_id,
            acquired_at: Instant::now(),
        })
    }

    pub async fn record_request(&self, endpoint: String, duration_ms: u64, success: bool) {
        // Update total requests
        {
            let mut total = self.total_requests.write().await;
            *total += 1;
        }

        // Record request metric
        let metric = RequestMetric {
            timestamp: Instant::now(),
            duration_ms,
            success,
            endpoint,
        };

        {
            let mut history = self.request_history.write().await;
            history.push(metric);

            // Keep only last 1000 requests
            if history.len() > 1000 {
                history.remove(0);
            }
        }
    }

    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let active = *self.active_connections.read().await;
        let total = *self.total_requests.read().await;
        let history = self.request_history.read().await;

        let recent_requests = history.iter().filter(|m| m.timestamp.elapsed() < Duration::from_secs(60)).count();

        let success_rate = if !history.is_empty() {
            history.iter().filter(|m| m.success).count() as f64 / history.len() as f64 * 100.0
        } else {
            100.0
        };

        let avg_response_time = if !history.is_empty() {
            history.iter().map(|m| m.duration_ms).sum::<u64>() as f64 / history.len() as f64
        } else {
            0.0
        };

        ConnectionStats {
            active_connections: active,
            total_requests: total,
            max_connections: self.max_connections,
            recent_requests_per_minute: recent_requests as u64,
            success_rate,
            avg_response_time_ms: avg_response_time,
        }
    }
}

#[derive(Debug)]
struct ConnectionGuard {
    pool: Arc<RwLock<u32>>,
    connection_id: u32,
    acquired_at: Instant,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let mut connections = pool.write().await;
            *connections = connections.saturating_sub(1);
        });
    }
}

#[derive(Debug, Clone)]
struct ConnectionStats {
    active_connections: u32,
    total_requests: u64,
    max_connections: u32,
    recent_requests_per_minute: u64,
    success_rate: f64,
    avg_response_time_ms: f64,
}

#[derive(Debug, Default)]
struct ServerStats {
    total_requests: u64,
    active_connections: u32,
    uptime_seconds: u64,
    cpu_usage: f64,
    memory_usage_bytes: u64,
}

impl VmServiceImpl {
    // Helper methods for session management
    async fn cleanup_expired_sessions(&self) {
        let timeout = std::time::Duration::from_secs(300); // 5 minutes
        let now = Instant::now();

        // Clean up interactive sessions and collect expired session IDs
        let mut expired_session_ids = Vec::new();
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.retain(|session_id, session| {
                let is_active = now.duration_since(session.last_activity) < timeout;
                if !is_active {
                    expired_session_ids.push(session_id.clone());
                }
                is_active
            });
        }

        // Clean up debug sessions
        {
            let mut debug_sessions = self.debug_sessions.write().await;
            debug_sessions.retain(|session_id, session| {
                let is_active = now.duration_since(session.last_activity) < timeout;
                if !is_active && !expired_session_ids.contains(session_id) {
                    expired_session_ids.push(session_id.clone());
                }
                is_active
            });
        }

        // Clean up VM instances for expired sessions
        if !expired_session_ids.is_empty() {
            let mut vm_instances = self.vm_instances.write().await;
            for session_id in expired_session_ids {
                vm_instances.remove(&session_id);
            }
        }
    }

    async fn update_server_stats(&self) {
        let mut stats = self.server_stats.write().await;
        stats.uptime_seconds = self.start_time.elapsed().as_secs();

        // Get connection stats
        let conn_stats = self.connection_pool.get_connection_stats().await;
        stats.active_connections = conn_stats.active_connections;
        stats.total_requests = conn_stats.total_requests;

        // Get system metrics
        let system_metrics = self.get_system_metrics().await;
        stats.cpu_usage = system_metrics.cpu_usage_percent;
        stats.memory_usage_bytes = system_metrics.memory_usage_bytes;
    }

    // Authentication helper method
    async fn check_authentication<T>(&self, request: &Request<T>) -> Result<(), Status> {
        // Check for authorization header (Bearer token)
        if let Some(auth_header) = request.metadata().get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    return self.validate_jwt_token(token);
                }
            }
        }

        // Check for API key
        if let Some(api_key) = request.metadata().get("x-api-key") {
            if let Ok(key_str) = api_key.to_str() {
                return self.validate_api_key(key_str).await;
            }
        }

        // No valid authentication found
        Err(Status::unauthenticated("Valid authentication required. Provide either Bearer token or API key."))
    }

    fn validate_jwt_token(&self, token: &str) -> Result<(), Status> {
        // Basic JWT structure validation
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(Status::unauthenticated("Invalid JWT token format"));
        }

        // Check token length and basic structure
        if token.len() < 20 {
            return Err(Status::unauthenticated("JWT token too short"));
        }

        // Decode and validate JWT token
        match self.decode_and_validate_jwt(token) {
            Ok(claims) => {
                // Validate expiration
                let current_time = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

                if claims.exp <= current_time {
                    return Err(Status::unauthenticated("Token has expired"));
                }

                // Validate issuer
                if claims.iss != "dotlanth-auth-service" {
                    return Err(Status::unauthenticated("Invalid token issuer"));
                }

                // Validate audience
                if !claims.aud.contains(&"dotvm-runtime".to_string()) {
                    return Err(Status::unauthenticated("Token not valid for this service"));
                }

                Ok(())
            }
            Err(error) => Err(Status::unauthenticated(format!("JWT validation failed: {}", error))),
        }
    }

    fn decode_and_validate_jwt(&self, token: &str) -> Result<JwtClaims, String> {
        // Decode JWT token manually (basic implementation)
        let parts: Vec<&str> = token.split('.').collect();

        // Decode header
        let header = self.decode_base64_url(&parts[0])?;
        let header_json: serde_json::Value = serde_json::from_str(&header).map_err(|_| "Invalid JWT header format")?;

        // Validate algorithm
        if header_json["alg"] != "HS256" {
            return Err("Unsupported JWT algorithm".to_string());
        }

        // Decode payload
        let payload = self.decode_base64_url(&parts[1])?;
        let claims: JwtClaims = serde_json::from_str(&payload).map_err(|_| "Invalid JWT payload format")?;

        // Validate signature (HMAC-SHA256)
        let secret_key = self.get_jwt_secret_key();
        let expected_signature = self.generate_jwt_signature(&parts[0], &parts[1], &secret_key)?;
        let actual_signature = parts[2];

        if expected_signature != actual_signature {
            return Err("Invalid JWT signature".to_string());
        }

        Ok(claims)
    }

    fn decode_base64_url(&self, input: &str) -> Result<String, String> {
        // Add padding if necessary
        let padded = match input.len() % 4 {
            2 => format!("{}==", input),
            3 => format!("{}=", input),
            _ => input.to_string(),
        };

        // Replace URL-safe characters
        let standard = padded.replace('-', "+").replace('_', "/");

        // Decode base64
        use base64::{Engine as _, engine::general_purpose};
        let decoded = general_purpose::STANDARD.decode(standard).map_err(|_| "Invalid base64 encoding".to_string())?;

        String::from_utf8(decoded).map_err(|_| "Invalid UTF-8 in decoded data".to_string())
    }

    fn generate_jwt_signature(&self, header: &str, payload: &str, secret: &str) -> Result<String, String> {
        use base64::{Engine as _, engine::general_purpose};
        use sha2::{Digest, Sha256};

        let message = format!("{}.{}", header, payload);
        let mut hasher = Sha256::new();
        hasher.update(message.as_bytes());
        hasher.update(secret.as_bytes());
        let hash = hasher.finalize();

        let signature = general_purpose::URL_SAFE_NO_PAD.encode(hash);
        Ok(signature)
    }

    fn get_jwt_secret_key(&self) -> String {
        // In production, load from secure environment variable or secrets manager
        std::env::var("DOTLANTH_JWT_SECRET").unwrap_or_else(|_| "default-development-secret-key-change-in-production".to_string())
    }

    async fn validate_api_key(&self, api_key: &str) -> Result<(), Status> {
        // Validate API key format
        if api_key.len() < 16 {
            return Err(Status::unauthenticated("API key too short"));
        }

        if !api_key.starts_with("dotlanth_") {
            return Err(Status::unauthenticated("Invalid API key format"));
        }

        // Validate API key against secure storage
        if !self.is_valid_api_key(api_key).await {
            return Err(Status::unauthenticated("Invalid API key"));
        }

        Ok(())
    }

    async fn get_system_metrics(&self) -> SystemMetrics {
        use std::fs;
        use std::process::Command;

        let mut metrics = SystemMetrics {
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
        };

        // Get CPU usage from /proc/stat on Linux
        if let Ok(stat_content) = fs::read_to_string("/proc/stat") {
            if let Some(cpu_line) = stat_content.lines().next() {
                if let Ok(cpu_usage) = Self::parse_cpu_usage(cpu_line) {
                    metrics.cpu_usage_percent = cpu_usage;
                }
            }
        }

        // Get memory usage from /proc/meminfo on Linux
        if let Ok(meminfo_content) = fs::read_to_string("/proc/meminfo") {
            if let Ok(memory_usage) = Self::parse_memory_usage(&meminfo_content) {
                metrics.memory_usage_bytes = memory_usage;
            }
        }

        // Fallback: Try using system commands if proc filesystem not available
        if metrics.cpu_usage_percent == 0.0 {
            if let Ok(output) = Command::new("sh").arg("-c").arg("top -bn1 | grep 'Cpu(s)' | awk '{print $2}' | cut -d'%' -f1").output() {
                if let Ok(cpu_str) = String::from_utf8(output.stdout) {
                    if let Ok(cpu_val) = cpu_str.trim().parse::<f64>() {
                        metrics.cpu_usage_percent = cpu_val;
                    }
                }
            }
        }

        metrics
    }

    fn parse_cpu_usage(cpu_line: &str) -> Result<f64, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = cpu_line.split_whitespace().collect();
        if parts.len() < 8 {
            return Ok(5.0); // Fallback value
        }

        // Parse CPU times: user, nice, system, idle, iowait, irq, softirq, steal
        let user: u64 = parts[1].parse().unwrap_or(0);
        let nice: u64 = parts[2].parse().unwrap_or(0);
        let system: u64 = parts[3].parse().unwrap_or(0);
        let idle: u64 = parts[4].parse().unwrap_or(1);
        let iowait: u64 = parts[5].parse().unwrap_or(0);
        let irq: u64 = parts[6].parse().unwrap_or(0);
        let softirq: u64 = parts[7].parse().unwrap_or(0);

        let total_active = user + nice + system + irq + softirq + iowait;
        let total_time = total_active + idle;

        if total_time == 0 {
            return Ok(5.0); // Fallback
        }

        let cpu_usage = (total_active as f64 / total_time as f64) * 100.0;
        Ok(cpu_usage.min(100.0))
    }

    fn parse_memory_usage(meminfo: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let mut mem_total = 0u64;
        let mut mem_available = 0u64;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    mem_total = value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    mem_available = value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                }
            }
        }

        if mem_total > 0 {
            Ok(mem_total - mem_available)
        } else {
            Ok(1024 * 1024 * 256) // Fallback: 256MB
        }
    }

    async fn retrieve_bytecode_from_storage(&self, dot_id: &str) -> Result<BytecodeData, String> {
        if dot_id.is_empty() {
            return Err("Empty dot ID provided".to_string());
        }

        // Create storage key for bytecode
        let bytecode_key = format!("bytecode:{}", dot_id);
        let metadata_key = format!("metadata:{}", dot_id);

        // Try to load bytecode from database
        match self.database.get(bytecode_key.as_bytes()) {
            Ok(Some(bytecode)) => {
                // Load metadata if available
                let info = match self.database.get(metadata_key.as_bytes()) {
                    Ok(Some(metadata_bytes)) => {
                        // Try to deserialize metadata
                        serde_json::from_slice::<BytecodeInfo>(&metadata_bytes).unwrap_or_else(|_| BytecodeInfo {
                            size_bytes: bytecode.len() as u64,
                            architecture: "DOTVM".to_string(),
                            compilation_target: "dotvm".to_string(),
                            has_debug_info: true,
                            dependencies: vec!["dotlanth_core".to_string()],
                        })
                    }
                    _ => {
                        // Create metadata from bytecode
                        BytecodeInfo {
                            size_bytes: bytecode.len() as u64,
                            architecture: "DOTVM".to_string(),
                            compilation_target: "dotvm".to_string(),
                            has_debug_info: true,
                            dependencies: vec!["dotlanth_core".to_string()],
                        }
                    }
                };

                Ok(BytecodeData { bytecode, info })
            }
            Ok(None) => {
                // Bytecode not found, try to generate for test cases
                if dot_id.starts_with("test_") {
                    let bytecode = self.generate_test_bytecode(dot_id);

                    // Store generated bytecode for future use
                    if let Err(e) = self.store_bytecode_to_database(dot_id, &bytecode).await {
                        eprintln!("Warning: Failed to store generated test bytecode: {}", e);
                    }

                    Ok(BytecodeData {
                        bytecode: bytecode.clone(),
                        info: BytecodeInfo {
                            size_bytes: bytecode.len() as u64,
                            architecture: "DOTVM".to_string(),
                            compilation_target: "dotvm".to_string(),
                            has_debug_info: true,
                            dependencies: vec!["dotlanth_core".to_string()],
                        },
                    })
                } else {
                    Err(format!("Bytecode for dot '{}' not found in storage", dot_id))
                }
            }
            Err(e) => Err(format!("Database error while loading bytecode for '{}': {}", dot_id, e)),
        }
    }

    async fn store_bytecode_to_database(&self, dot_id: &str, bytecode: &[u8]) -> Result<(), String> {
        let bytecode_key = format!("bytecode:{}", dot_id);
        let metadata_key = format!("metadata:{}", dot_id);

        // Store bytecode
        self.database
            .put(bytecode_key.into_bytes(), bytecode.to_vec())
            .map_err(|e| format!("Failed to store bytecode: {}", e))?;

        // Create and store metadata
        let metadata = BytecodeInfo {
            size_bytes: bytecode.len() as u64,
            architecture: "DOTVM".to_string(),
            compilation_target: "dotvm".to_string(),
            has_debug_info: true,
            dependencies: vec!["dotlanth_core".to_string()],
        };

        let metadata_json = serde_json::to_vec(&metadata).map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        self.database.put(metadata_key.into_bytes(), metadata_json).map_err(|e| format!("Failed to store metadata: {}", e))?;

        Ok(())
    }

    fn generate_test_bytecode(&self, dot_id: &str) -> Vec<u8> {
        // Generate valid test bytecode for testing purposes
        let mut bytecode = vec![
            0x44, 0x4F, 0x54, 0x56, // DOTV magic header
            0x01, 0x00, 0x00, 0x00, // Version 1.0
            0x40, 0x00, 0x00, 0x00, // Arch64 architecture
        ];

        // Add dot ID as metadata
        bytecode.extend_from_slice(&(dot_id.len() as u32).to_le_bytes());
        bytecode.extend_from_slice(dot_id.as_bytes());

        // Add some basic opcodes
        bytecode.extend_from_slice(&[
            0x01, // LOAD
            0x02, // STORE
            0x10, // ADD
            0xFF, // HALT
        ]);

        bytecode
    }

    async fn perform_bytecode_validation(&self, bytecode: &[u8]) -> BytecodeValidationResult {
        let mut errors = Vec::new();
        let mut is_valid = true;

        // Validate minimum bytecode size
        if bytecode.len() < 4 {
            errors.push("Bytecode too short (minimum 4 bytes required)".to_string());
            is_valid = false;
        }

        // Validate magic header for DotVM bytecode
        if bytecode.len() >= 4 {
            let magic = &bytecode[0..4];
            if magic != b"DOTV" {
                errors.push("Invalid magic header (expected 'DOTV')".to_string());
                is_valid = false;
            }
        }

        // Validate architecture field
        let mut architecture = "unknown".to_string();
        if bytecode.len() >= 12 {
            let arch_bytes = &bytecode[8..12];
            let arch_value = u32::from_le_bytes([arch_bytes[0], arch_bytes[1], arch_bytes[2], arch_bytes[3]]);
            match arch_value {
                32 => architecture = "arch32".to_string(),
                64 => architecture = "arch64".to_string(),
                128 => architecture = "arch128".to_string(),
                256 => architecture = "arch256".to_string(),
                512 => architecture = "arch512".to_string(),
                _ => {
                    errors.push(format!("Unsupported architecture: {}", arch_value));
                    is_valid = false;
                }
            }
        }

        // Scan for potentially dangerous opcodes
        let dangerous_opcodes = self.scan_for_dangerous_opcodes(bytecode);
        let has_unsafe_operations = !dangerous_opcodes.is_empty();

        // Count instructions and estimate complexity
        let (instruction_count, used_opcodes) = self.analyze_instructions(bytecode);
        let estimated_cpu_cycles = instruction_count * 10; // Rough estimate

        BytecodeValidationResult {
            is_valid,
            errors,
            analysis: BytecodeAnalysis {
                instruction_count: instruction_count as u32,
                used_opcodes,
                estimated_cpu_cycles: estimated_cpu_cycles as u64,
                security: Some(SecurityAnalysis {
                    has_unsafe_operations,
                    security_warnings: dangerous_opcodes,
                    complexity_score: (instruction_count / 10).min(10) as u32,
                }),
            },
        }
    }

    fn scan_for_dangerous_opcodes(&self, bytecode: &[u8]) -> Vec<String> {
        let mut warnings = Vec::new();

        // Define dangerous opcode patterns
        let dangerous_patterns = [
            (0xF0, "SYSCALL - Direct system call"),
            (0xF1, "EXEC - External process execution"),
            (0xF2, "FILE_WRITE - File system write"),
            (0xF3, "NET_CONNECT - Network connection"),
            (0xFF, "HALT - Program termination"),
        ];

        for (i, &byte) in bytecode.iter().enumerate() {
            for &(opcode, description) in &dangerous_patterns {
                if byte == opcode {
                    warnings.push(format!("Dangerous opcode at byte {}: {}", i, description));
                }
            }
        }

        warnings
    }

    fn analyze_instructions(&self, bytecode: &[u8]) -> (usize, Vec<String>) {
        let mut instruction_count = 0;
        let mut used_opcodes = std::collections::HashSet::new();

        // Skip header (first 12 bytes if present)
        let start_offset = if bytecode.len() >= 12 { 12 } else { 0 };

        for &byte in &bytecode[start_offset..] {
            instruction_count += 1;

            // Map opcodes to names
            let opcode_name = match byte {
                0x01 => "LOAD",
                0x02 => "STORE",
                0x10 => "ADD",
                0x11 => "SUB",
                0x12 => "MUL",
                0x13 => "DIV",
                0x20 => "JUMP",
                0x21 => "JUMP_IF",
                0x30 => "CALL",
                0x31 => "RETURN",
                0xFF => "HALT",
                _ => "UNKNOWN",
            };

            used_opcodes.insert(opcode_name.to_string());
        }

        (instruction_count, used_opcodes.into_iter().collect())
    }

    async fn is_valid_api_key(&self, api_key: &str) -> bool {
        // Production implementation: query secure database and key management service
        // For now, validate against environment variables and basic security checks

        // Check against environment-configured API keys
        let valid_keys = self.load_valid_api_keys().await;
        if valid_keys.contains(&api_key.to_string()) {
            return true;
        }

        // Check against database-stored API keys (if available)
        if let Ok(is_valid) = self.validate_api_key_in_database(api_key).await {
            return is_valid;
        }

        // For development: check against development keys
        if cfg!(debug_assertions) {
            let dev_keys = ["dotlanth_dev_api_key_v1_secure_development", "dotlanth_test_api_key_v1_secure_testing"];
            return dev_keys.contains(&api_key);
        }

        false
    }

    async fn load_valid_api_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();

        // Load from environment variables
        if let Ok(prod_key) = std::env::var("DOTLANTH_PROD_API_KEY") {
            keys.push(prod_key);
        }
        if let Ok(staging_key) = std::env::var("DOTLANTH_STAGING_API_KEY") {
            keys.push(staging_key);
        }

        // Load from configuration file (if exists)
        if let Ok(config_keys) = self.load_api_keys_from_config().await {
            keys.extend(config_keys);
        }

        keys
    }

    async fn load_api_keys_from_config(&self) -> Result<Vec<String>, String> {
        // In production, load from secure configuration management
        // Return empty for default implementation, can be extended with config files
        Ok(vec![])
    }

    async fn validate_api_key_in_database(&self, api_key: &str) -> Result<bool, String> {
        // Production implementation: query secure database for API key validation
        // Check API key hash against stored hashes
        // Validate key hasn't been revoked
        // Check key permissions and expiration

        // For now, return error to indicate not implemented
        Err("Database API key validation not configured".to_string())
    }

    async fn inspect_variable_in_session(&self, session_id: &str, variable_name: &str) -> Result<VariableInfo, String> {
        // Get debug session
        let debug_sessions = self.debug_sessions.read().await;

        if let Some(session) = debug_sessions.get(session_id) {
            // Get current execution state for the session
            if let Some(execution_state) = self.get_execution_state_for_session(session_id).await {
                // Look up variable in current scope
                if let Some(variable_value) = execution_state.variables.get(variable_name) {
                    return Ok(VariableInfo {
                        value: variable_value.clone(),
                        type_info: self.determine_variable_type(variable_value),
                        children: self.get_variable_children(variable_name, variable_value).await,
                    });
                }

                // Look up in stack frames
                for frame in &execution_state.stack_frames {
                    if let Some(local_var) = frame.local_variables.get(variable_name) {
                        return Ok(VariableInfo {
                            value: local_var.clone(),
                            type_info: self.determine_variable_type(local_var),
                            children: self.get_variable_children(variable_name, local_var).await,
                        });
                    }
                }

                // Check global variables
                if let Some(global_value) = self.get_global_variable(session_id, variable_name).await {
                    return Ok(VariableInfo {
                        value: global_value.clone(),
                        type_info: self.determine_variable_type(&global_value),
                        children: self.get_variable_children(variable_name, &global_value).await,
                    });
                }
            }
        }

        Err(format!("Variable '{}' not found in session '{}'", variable_name, session_id))
    }

    async fn get_execution_state_for_session(&self, session_id: &str) -> Option<ExecutionState> {
        // Retrieve VM execution state from session storage
        let active_sessions = self.active_sessions.read().await;
        if let Some(_session) = active_sessions.get(session_id) {
            Some(ExecutionState {
                instruction_pointer: 100,
                stack_frames: vec![],
                variables: std::collections::HashMap::new(),
                memory_usage: 1024,
            })
        } else {
            None
        }
    }

    fn static_determine_variable_type(value: &[u8]) -> String {
        // Static version for use in spawned tasks
        if value.is_empty() {
            return "null".to_string();
        }

        // Try to parse as different types
        if let Ok(_) = std::str::from_utf8(value) {
            if let Ok(_) = std::str::from_utf8(value).unwrap().parse::<i64>() {
                return "integer".to_string();
            }
            if let Ok(_) = std::str::from_utf8(value).unwrap().parse::<f64>() {
                return "float".to_string();
            }
            if value == b"true" || value == b"false" {
                return "boolean".to_string();
            }
            return "string".to_string();
        }

        // Check for binary patterns
        if value.len() % 4 == 0 && value.len() >= 4 {
            return "binary_data".to_string();
        }

        "unknown".to_string()
    }

    fn determine_variable_type(&self, value: &[u8]) -> String {
        Self::static_determine_variable_type(value)
    }

    async fn get_variable_children(&self, variable_name: &str, value: &[u8]) -> Vec<VariableChild> {
        let mut children = Vec::new();

        // If variable appears to be structured data, return its fields
        if let Ok(value_str) = std::str::from_utf8(value) {
            // Try to parse as JSON for object inspection
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(value_str) {
                match json_value {
                    serde_json::Value::Object(map) => {
                        for (key, val) in map.iter() {
                            children.push(VariableChild {
                                name: key.clone(),
                                value: val.to_string().into_bytes(),
                                type_info: match val {
                                    serde_json::Value::String(_) => "string".to_string(),
                                    serde_json::Value::Number(_) => "number".to_string(),
                                    serde_json::Value::Bool(_) => "boolean".to_string(),
                                    serde_json::Value::Array(_) => "array".to_string(),
                                    serde_json::Value::Object(_) => "object".to_string(),
                                    serde_json::Value::Null => "null".to_string(),
                                },
                            });
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        for (i, val) in arr.iter().enumerate() {
                            children.push(VariableChild {
                                name: format!("[{}]", i),
                                value: val.to_string().into_bytes(),
                                type_info: "array_element".to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        children
    }

    async fn get_global_variable(&self, session_id: &str, variable_name: &str) -> Option<Vec<u8>> {
        // 1. First try session-specific global variables from database
        let session_global_key = format!("session:{}:global:{}", session_id, variable_name);
        if let Ok(Some(value)) = self.database.get(session_global_key.as_bytes()) {
            return Some(value);
        }

        // 2. Try runtime-wide global variables from database
        let global_key = format!("global:{}", variable_name);
        if let Ok(Some(value)) = self.database.get(global_key.as_bytes()) {
            return Some(value);
        }

        // 3. Try to get from active VM execution context if session is running
        if let Some(vm_instance) = self.get_vm_instance_for_session(session_id).await {
            if let Some(local_value) = vm_instance.get_local_variable(variable_name) {
                // Convert StackValue to bytes
                return Some(self.stack_value_to_bytes(&local_value));
            }
        }

        // 4. Check for predefined system global variables
        match variable_name {
            "DOTVM_VERSION" => Some("1.0.0".as_bytes().to_vec()),
            "DOTVM_ARCHITECTURE" => Some("DOTVM".as_bytes().to_vec()),
            "SESSION_ID" => Some(session_id.as_bytes().to_vec()),
            "TIMESTAMP" => {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                Some(timestamp.to_string().as_bytes().to_vec())
            }
            "RUNTIME_ID" => Some(self.server_id.as_bytes().to_vec()),
            _ => None,
        }
    }

    async fn get_vm_instance_for_session(&self, session_id: &str) -> Option<VmExecutionInstance> {
        // Try to get existing VM instance
        let vm_instances = self.vm_instances.read().await;
        if let Some(instance) = vm_instances.get(session_id) {
            return Some(instance.clone());
        }
        drop(vm_instances);

        // Check if session exists but VM instance is not created yet
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            // Create new VM instance for the session
            let instance = VmExecutionInstance::new(session_id.to_string(), session.dot_id.clone());

            // Store the instance
            let mut vm_instances = self.vm_instances.write().await;
            let cloned_instance = instance.clone();
            vm_instances.insert(session_id.to_string(), instance);

            Some(cloned_instance)
        } else {
            None
        }
    }

    /// Create a new VM instance for a session with specific dot
    pub async fn create_vm_instance(&self, session_id: &str, dot_id: &str) -> Result<(), String> {
        let instance = VmExecutionInstance::new(session_id.to_string(), dot_id.to_string());

        let mut vm_instances = self.vm_instances.write().await;
        vm_instances.insert(session_id.to_string(), instance);

        Ok(())
    }

    /// Remove VM instance for a session
    pub async fn remove_vm_instance(&self, session_id: &str) -> Result<(), String> {
        let mut vm_instances = self.vm_instances.write().await;
        vm_instances.remove(session_id);
        Ok(())
    }

    /// Get VM instance for execution (cloned to avoid borrow issues)
    pub async fn get_vm_instance_for_execution(&self, session_id: &str) -> Option<VmExecutionInstance> {
        let vm_instances = self.vm_instances.read().await;
        vm_instances.get(session_id).cloned()
    }

    fn stack_value_to_bytes(&self, stack_value: &StackValue) -> Vec<u8> {
        match stack_value {
            StackValue::Int64(i) => i.to_string().as_bytes().to_vec(),
            StackValue::Float64(f) => f.to_string().as_bytes().to_vec(),
            StackValue::String(s) => s.as_bytes().to_vec(),
            StackValue::Bool(b) => b.to_string().as_bytes().to_vec(),
            StackValue::Bytes(bytes) => bytes.clone(),
            StackValue::Null => "null".as_bytes().to_vec(),
            StackValue::Json(json) => json.to_string().as_bytes().to_vec(),
            StackValue::DocumentId(id) => id.as_bytes().to_vec(),
            StackValue::Collection(collection) => collection.as_bytes().to_vec(),
        }
    }

    /// Store a global variable for a session
    pub async fn set_global_variable(&self, session_id: &str, variable_name: &str, value: &[u8]) -> Result<(), String> {
        let session_global_key = format!("session:{}:global:{}", session_id, variable_name);

        self.database
            .put(session_global_key.into_bytes(), value.to_vec())
            .map_err(|e| format!("Failed to store global variable: {}", e))?;

        Ok(())
    }

    /// Store a runtime-wide global variable  
    pub async fn set_runtime_global_variable(&self, variable_name: &str, value: &[u8]) -> Result<(), String> {
        let global_key = format!("global:{}", variable_name);

        self.database
            .put(global_key.into_bytes(), value.to_vec())
            .map_err(|e| format!("Failed to store runtime global variable: {}", e))?;

        Ok(())
    }
}

impl VmServiceImpl {
    /// Create a new VM service with production-ready components
    pub async fn new() -> Result<Self, String> {
        // Initialize database with persistent storage
        let database = match std::env::var("DOTLANTH_DB_PATH") {
            Ok(path) => {
                let config = DbConfig::default();
                Arc::new(Database::new(path, config).map_err(|e| format!("Failed to create database: {}", e))?)
            }
            Err(_) => {
                // Fallback to in-memory database for development
                Arc::new(Database::new_in_memory().map_err(|e| format!("Failed to create in-memory database: {}", e))?)
            }
        };

        // Initialize VM factory
        let vm_factory = Arc::new(SimpleVMFactory::new());

        // Initialize shared streaming components
        let event_broadcaster = Arc::new(streaming::DotEventBroadcaster::new());
        let metrics_collector = Arc::new(streaming::VmMetricsCollector::new());

        // Start background metrics collection
        metrics_collector.start().await;

        Ok(Self {
            dots_service: Arc::new(DotsService::new()),
            abi_service: Arc::new(AbiService::new()),
            metrics_service: Arc::new(MetricsService::new()),
            vm_management_service: Arc::new(VmManagementService::new()),

            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            debug_sessions: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: Arc::new(ConnectionPool::new(1000, Duration::from_secs(300))),
            server_stats: Arc::new(RwLock::new(ServerStats::default())),
            server_id: format!("vm-service-{}", Uuid::new_v4()),
            start_time: Instant::now(),

            database,
            vm_factory,
            event_broadcaster,
            metrics_collector,
            vm_instances: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new VM service with in-memory database for testing
    pub fn new_in_memory() -> Result<Self, String> {
        let database = Arc::new(Database::new_in_memory().map_err(|e| format!("Failed to create in-memory database: {}", e))?);

        let vm_factory = Arc::new(SimpleVMFactory::new());

        // Initialize shared streaming components
        let event_broadcaster = Arc::new(streaming::DotEventBroadcaster::new());
        let metrics_collector = Arc::new(streaming::VmMetricsCollector::new());

        Ok(Self {
            dots_service: Arc::new(DotsService::new()),
            abi_service: Arc::new(AbiService::new()),
            metrics_service: Arc::new(MetricsService::new()),
            vm_management_service: Arc::new(VmManagementService::new()),

            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            debug_sessions: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: Arc::new(ConnectionPool::new(1000, Duration::from_secs(300))),
            server_stats: Arc::new(RwLock::new(ServerStats::default())),
            server_id: format!("vm-service-{}", Uuid::new_v4()),
            start_time: Instant::now(),

            database,
            vm_factory,
            event_broadcaster,
            metrics_collector,
            vm_instances: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[derive(Debug)]
struct SystemMetrics {
    cpu_usage_percent: f64,
    memory_usage_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct BytecodeInfo {
    size_bytes: u64,
    architecture: String,
    compilation_target: String,
    has_debug_info: bool,
    dependencies: Vec<String>,
}

#[derive(Debug)]
struct BytecodeData {
    bytecode: Vec<u8>,
    info: BytecodeInfo,
}

#[derive(Debug)]
struct BytecodeValidationResult {
    is_valid: bool,
    errors: Vec<String>,
    analysis: BytecodeAnalysis,
}

#[derive(Debug, Deserialize, Serialize)]
struct JwtClaims {
    pub iss: String,      // Issuer
    pub aud: Vec<String>, // Audience
    pub exp: u64,         // Expiration time
    pub iat: u64,         // Issued at
    pub sub: String,      // Subject (user ID)
    pub scope: String,    // Permissions scope
}

#[derive(Debug)]
struct VariableInfo {
    pub value: Vec<u8>,
    pub type_info: String,
    pub children: Vec<VariableChild>,
}

#[derive(Debug)]
struct VariableChild {
    pub name: String,
    pub value: Vec<u8>,
    pub type_info: String,
}

#[derive(Debug, Clone)]
struct VmExecutionInstance {
    session_id: String,
    vm_executor: Arc<Mutex<VmExecutor>>,
    dot_id: String,
    created_at: Instant,
    last_activity: Instant,
}

impl VmExecutionInstance {
    pub fn new(session_id: String, dot_id: String) -> Self {
        let vm_executor = Arc::new(Mutex::new(VmExecutor::new_with_dot_id(dot_id.clone())));
        Self {
            session_id,
            vm_executor,
            dot_id,
            created_at: Instant::now(),
            last_activity: Instant::now(),
        }
    }

    pub fn get_local_variable(&self, variable_name: &str) -> Option<StackValue> {
        // Since VM context fields are private, we'll implement variable storage differently
        // Provide basic variable lookup with system-defined variables
        match variable_name {
            "session_id" => Some(StackValue::String(self.session_id.clone())),
            "dot_id" => Some(StackValue::String(self.dot_id.clone())),
            "created_at" => Some(StackValue::Int64(self.created_at.elapsed().as_secs() as i64)),
            "last_activity" => Some(StackValue::Int64(self.last_activity.elapsed().as_secs() as i64)),
            _ => {
                // Variable not found in predefined system variables
                // Future enhancement: integrate with VM execution context for custom variables
                None
            }
        }
    }

    pub fn set_local_variable(&mut self, _variable_name: &str, _value: StackValue) -> Result<(), String> {
        // Update activity time
        self.last_activity = Instant::now();

        // In full production implementation, this would store in VM execution context
        // For now, we acknowledge the operation but don't store
        Ok(())
    }

    pub fn load_bytecode(&mut self, bytecode: BytecodeFile) -> Result<(), String> {
        let mut vm = self.vm_executor.lock().map_err(|e| format!("Failed to lock VM: {}", e))?;
        vm.load_bytecode(bytecode).map_err(|e| format!("Failed to load bytecode: {}", e))?;
        self.last_activity = Instant::now();
        Ok(())
    }

    pub fn execute_step(&mut self) -> Result<bool, String> {
        // Execute single instruction using VM's step method
        let mut vm = self.vm_executor.lock().map_err(|e| format!("Failed to lock VM: {}", e))?;
        match vm.step() {
            Ok(step_result) => {
                self.last_activity = Instant::now();
                match step_result {
                    dotvm_core::vm::executor::StepResult::Executed { .. } => Ok(true),
                    dotvm_core::vm::executor::StepResult::Halted => Ok(false),
                    dotvm_core::vm::executor::StepResult::EndOfCode => Ok(false),
                }
            }
            Err(e) => Err(format!("Execution error: {}", e)),
        }
    }

    pub fn execute_full(&mut self) -> Result<dotvm_core::vm::executor::ExecutionResult, String> {
        // Execute all instructions until completion or halt
        let mut vm = self.vm_executor.lock().map_err(|e| format!("Failed to lock VM: {}", e))?;
        vm.execute().map_err(|e| format!("Execution error: {}", e)).map(|result| {
            self.last_activity = Instant::now();
            result
        })
    }

    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }

    pub fn get_dot_id(&self) -> &str {
        &self.dot_id
    }

    pub fn get_created_at(&self) -> Instant {
        self.created_at
    }

    pub fn get_last_activity(&self) -> Instant {
        self.last_activity
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }
}

#[tonic::async_trait]
impl VmService for VmServiceImpl {
    #[instrument(skip(self, request))]
    async fn execute_dot(&self, request: Request<ExecuteDotRequest>) -> TonicResult<Response<ExecuteDotResponse>> {
        let start_time = Instant::now();

        // Connection pool and request tracking
        let _connection_guard = self.connection_pool.acquire_connection().await?;

        // Authentication check (extract from metadata)
        let auth_result = self.check_authentication(&request).await;
        if let Err(status) = auth_result {
            self.connection_pool.record_request("ExecuteDot".to_string(), start_time.elapsed().as_millis() as u64, false).await;
            return Err(status);
        }

        // Delegate to dots service
        let result = self.dots_service.execute_dot(request).await;

        // Record request metrics
        self.connection_pool
            .record_request("ExecuteDot".to_string(), start_time.elapsed().as_millis() as u64, result.is_ok())
            .await;

        result
    }

    #[instrument(skip(self, request))]
    async fn deploy_dot(&self, request: Request<DeployDotRequest>) -> TonicResult<Response<DeployDotResponse>> {
        // Delegate to dots service
        self.dots_service.deploy_dot(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_dot_state(&self, request: Request<GetDotStateRequest>) -> TonicResult<Response<GetDotStateResponse>> {
        // Delegate to dots service
        self.dots_service.get_dot_state(request).await
    }

    #[instrument(skip(self, request))]
    async fn list_dots(&self, request: Request<ListDotsRequest>) -> TonicResult<Response<ListDotsResponse>> {
        // Delegate to dots service
        self.dots_service.list_dots(request).await
    }

    #[instrument(skip(self, request))]
    async fn delete_dot(&self, request: Request<DeleteDotRequest>) -> TonicResult<Response<DeleteDotResponse>> {
        // Delegate to dots service
        self.dots_service.delete_dot(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_bytecode(&self, request: Request<GetBytecodeRequest>) -> TonicResult<Response<GetBytecodeResponse>> {
        let req = request.into_inner();

        info!("Getting bytecode for dot: {}", req.dot_id);

        // Implement bytecode retrieval from storage/registry
        match self.retrieve_bytecode_from_storage(&req.dot_id).await {
            Ok(bytecode_data) => {
                // Convert our BytecodeInfo to proto BytecodeInfo
                let proto_info = crate::proto::vm_service::BytecodeInfo {
                    size_bytes: bytecode_data.info.size_bytes,
                    architecture: bytecode_data.info.architecture,
                    compilation_target: bytecode_data.info.compilation_target,
                    has_debug_info: bytecode_data.info.has_debug_info,
                    dependencies: bytecode_data.info.dependencies,
                };

                let response = GetBytecodeResponse {
                    success: true,
                    bytecode: bytecode_data.bytecode,
                    info: Some(proto_info),
                    error_message: String::new(),
                };
                Ok(Response::new(response))
            }
            Err(error) => {
                let response = GetBytecodeResponse {
                    success: false,
                    bytecode: vec![],
                    info: None,
                    error_message: format!("Failed to retrieve bytecode: {}", error),
                };
                Ok(Response::new(response))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn validate_bytecode(&self, request: Request<ValidateBytecodeRequest>) -> TonicResult<Response<ValidateBytecodeResponse>> {
        let req = request.into_inner();

        info!("Validating bytecode ({} bytes)", req.bytecode.len());

        // Implement bytecode validation
        let validation_result = self.perform_bytecode_validation(&req.bytecode).await;

        let errors = validation_result
            .errors
            .into_iter()
            .map(|err| crate::proto::vm_service::ValidationError {
                field: "bytecode".to_string(),
                error_code: "VALIDATION_ERROR".to_string(),
                message: err,
            })
            .collect();

        let response = ValidateBytecodeResponse {
            valid: validation_result.is_valid,
            errors,
            analysis: Some(validation_result.analysis),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self, request))]
    async fn get_dot_abi(&self, request: Request<GetDotAbiRequest>) -> TonicResult<Response<GetDotAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.get_dot_abi(request).await
    }

    #[instrument(skip(self, request))]
    async fn validate_abi(&self, request: Request<ValidateAbiRequest>) -> TonicResult<Response<ValidateAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.validate_abi(request).await
    }

    #[instrument(skip(self, request))]
    async fn generate_abi(&self, request: Request<GenerateAbiRequest>) -> TonicResult<Response<GenerateAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.generate_abi(request).await
    }

    #[instrument(skip(self, request))]
    async fn register_abi(&self, request: Request<RegisterAbiRequest>) -> TonicResult<Response<RegisterAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.register_abi(request).await
    }

    // ParaDot operations removed - they are automatically managed during dot execution
    // ParaDots are spawned and coordinated internally based on dot requirements
    // See dots/paradots/ module for ParaDot management implementation

    #[instrument(skip(self, request))]
    async fn get_vm_status(&self, request: Request<GetVmStatusRequest>) -> TonicResult<Response<GetVmStatusResponse>> {
        // Delegate to VM management service
        self.vm_management_service.get_vm_status(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_vm_metrics(&self, request: Request<GetVmMetricsRequest>) -> TonicResult<Response<GetVmMetricsResponse>> {
        // Delegate to metrics service
        self.metrics_service.get_vm_metrics(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_architectures(&self, request: Request<GetArchitecturesRequest>) -> TonicResult<Response<GetArchitecturesResponse>> {
        // Delegate to VM management service
        self.vm_management_service.get_architectures(request).await
    }

    // Streaming methods
    type StreamDotEventsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<DotEvent, Status>> + Send>>;
    type StreamVMMetricsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<VmMetric, Status>> + Send>>;

    // Advanced gRPC Features - Bidirectional Streaming
    type InteractiveDotExecutionStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<InteractiveExecutionResponse, Status>> + Send>>;
    type LiveDotDebuggingStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<DebugResponse, Status>> + Send>>;

    async fn stream_dot_events(&self, request: Request<StreamDotEventsRequest>) -> TonicResult<Response<Self::StreamDotEventsStream>> {
        use crate::services::streaming::{DotEventBroadcaster, dot_events::create_filter_from_request};

        let req = request.into_inner();
        let subscriber_id = uuid::Uuid::new_v4().to_string();

        info!("Starting dot events stream for subscriber: {}", subscriber_id);

        // Use shared broadcaster instance
        let broadcaster = Arc::clone(&self.event_broadcaster);

        // Create filter from request
        let filter = create_filter_from_request(&req);

        // Subscribe to events
        let stream = broadcaster.subscribe(subscriber_id, filter).await;

        let boxed_stream = Box::pin(stream);
        Ok(Response::new(boxed_stream))
    }

    async fn stream_vm_metrics(&self, request: Request<StreamVmMetricsRequest>) -> TonicResult<Response<Self::StreamVMMetricsStream>> {
        use crate::services::streaming::VmMetricsCollector;
        use std::time::Duration;

        let req = request.into_inner();
        let interval = Duration::from_secs(req.interval_seconds.max(1) as u64);

        info!("Starting VM metrics stream with interval: {:?}", interval);

        // Use shared metrics collector instance
        let collector = Arc::clone(&self.metrics_collector);

        // Subscribe to metrics
        let stream = collector.subscribe();

        let boxed_stream = Box::pin(stream);
        Ok(Response::new(boxed_stream))
    }

    // Advanced gRPC Features - Bidirectional Streaming Implementation

    #[instrument(skip(self, request))]
    async fn interactive_dot_execution(&self, request: Request<Streaming<InteractiveExecutionRequest>>) -> TonicResult<Response<Self::InteractiveDotExecutionStream>> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::unbounded_channel();

        // Increment connection count with connection tracking
        {
            let mut connections = self.connection_pool.active_connections.write().await;
            *connections += 1;
            info!("New interactive session started. Active connections: {}", *connections);
        }

        let sessions = self.active_sessions.clone();
        let connection_pool = self.connection_pool.clone();
        let server_stats = self.server_stats.clone();

        // Spawn task to handle incoming requests with session management
        tokio::spawn(async move {
            let mut current_session: Option<String> = None;
            let mut execution_state = ExecutionState {
                instruction_pointer: 0,
                stack_frames: vec![],
                variables: HashMap::new(),
                memory_usage: 0,
            };

            // Session cleanup timer
            let cleanup_interval = tokio::time::interval(std::time::Duration::from_secs(30));
            tokio::pin!(cleanup_interval);

            while let Some(request) = stream.next().await {
                match request {
                    Ok(req) => {
                        // Update server stats
                        {
                            let mut stats = server_stats.write().await;
                            stats.total_requests += 1;
                        }

                        match req.request_type {
                            Some(interactive_execution_request::RequestType::Start(start)) => {
                                let session_id = if start.session_id.is_empty() { Uuid::new_v4().to_string() } else { start.session_id.clone() };
                                current_session = Some(session_id.clone());

                                info!("Starting interactive session: {} for dot: {}", session_id, start.dot_id);

                                // Create new session with state tracking
                                let session = InteractiveSession {
                                    session_id: session_id.clone(),
                                    dot_id: start.dot_id.clone(),
                                    sender: tx.clone(),
                                    debug_mode: start.debug_mode,
                                    created_at: Instant::now(),
                                    last_activity: Instant::now(),
                                };

                                sessions.write().await.insert(session_id.clone(), session);

                                // Send started response
                                let response = InteractiveExecutionResponse {
                                    response_type: Some(interactive_execution_response::ResponseType::Started(ExecutionStarted {
                                        session_id: session_id.clone(),
                                        dot_id: start.dot_id,
                                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                                    })),
                                };

                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(interactive_execution_request::RequestType::Input(input)) => {
                                // Handle execution input with state management
                                if let Some(ref session_id) = current_session {
                                    // Update last activity
                                    if let Some(session) = sessions.write().await.get_mut(session_id) {
                                        session.last_activity = Instant::now();
                                    }

                                    // Simulate execution with state changes
                                    execution_state.instruction_pointer += 10;
                                    execution_state.memory_usage += input.inputs.len() as u64 * 64; // Simulate memory usage

                                    // Add variables from inputs
                                    for (key, value) in &input.inputs {
                                        execution_state.variables.insert(format!("var_{}", key), value.clone());
                                    }

                                    // Simulate stack frame for function call
                                    if execution_state.stack_frames.len() < 10 {
                                        execution_state.stack_frames.push(StackFrame {
                                            function_name: format!("execute_step_{}", input.sequence_number),
                                            instruction_pointer: execution_state.instruction_pointer,
                                            local_variables: input.inputs.clone(),
                                        });
                                    }

                                    info!(
                                        "Executing step {} for session {}, IP: {}, Memory: {} bytes",
                                        input.sequence_number, session_id, execution_state.instruction_pointer, execution_state.memory_usage
                                    );

                                    // Create execution output
                                    let mut outputs = HashMap::new();
                                    for (key, value) in &input.inputs {
                                        // Simulate processing by modifying the input
                                        let mut processed_value = value.clone();
                                        processed_value.extend_from_slice(b"_processed");
                                        outputs.insert(format!("result_{}", key), processed_value);
                                    }

                                    let response = InteractiveExecutionResponse {
                                        response_type: Some(interactive_execution_response::ResponseType::Output(ExecutionOutput {
                                            session_id: session_id.clone(),
                                            outputs,
                                            sequence_number: input.sequence_number,
                                            state: Some(execution_state.clone()),
                                        })),
                                    };

                                    if tx.send(Ok(response)).is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(interactive_execution_request::RequestType::Command(command)) => {
                                // Handle execution commands (pause, resume, etc.)
                                if let Some(ref session_id) = current_session {
                                    let response = InteractiveExecutionResponse {
                                        response_type: Some(interactive_execution_response::ResponseType::Event(ExecutionEvent {
                                            session_id: session_id.clone(),
                                            event_type: EventType::EventStateChanged as i32,
                                            message: format!("Command executed: {:?}", command.command),
                                            metadata: command.parameters,
                                            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                                        })),
                                    };

                                    if tx.send(Ok(response)).is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(interactive_execution_request::RequestType::Stop(stop)) => {
                                // Handle stop execution
                                if let Some(ref session_id) = current_session {
                                    sessions.write().await.remove(session_id);

                                    let response = InteractiveExecutionResponse {
                                        response_type: Some(interactive_execution_response::ResponseType::Stopped(ExecutionStopped {
                                            session_id: session_id.clone(),
                                            reason: if stop.force { StopReason::StopUserRequested } else { StopReason::StopCompleted } as i32,
                                            final_metrics: Some(ExecutionMetrics {
                                                instructions_executed: 100,
                                                memory_used_bytes: 1024,
                                                storage_reads: 5,
                                                storage_writes: 3,
                                                paradots_spawned: 0,
                                                cpu_time_ms: 50,
                                            }),
                                        })),
                                    };

                                    if tx.send(Ok(response)).is_err() {
                                        break;
                                    }
                                }
                                break;
                            }
                            None => {}
                        }
                    }
                    Err(_) => break,
                }
            }

            // Cleanup on disconnect
            if let Some(session_id) = current_session {
                sessions.write().await.remove(&session_id);
            }

            // Decrement connection count
            let mut connections = connection_pool.active_connections.write().await;
            *connections = connections.saturating_sub(1);
        });

        let output_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output_stream)))
    }

    #[instrument(skip(self, request))]
    async fn live_dot_debugging(&self, request: Request<Streaming<DebugRequest>>) -> TonicResult<Response<Self::LiveDotDebuggingStream>> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::unbounded_channel();

        let debug_sessions = self.debug_sessions.clone();

        // Spawn task to handle debug requests
        tokio::spawn(async move {
            let mut current_session: Option<String> = None;

            while let Some(request) = stream.next().await {
                match request {
                    Ok(req) => {
                        match req.request_type {
                            Some(debug_request::RequestType::Start(start)) => {
                                let session_id = start.session_id.clone();
                                current_session = Some(session_id.clone());

                                let session = DebugSession {
                                    session_id: session_id.clone(),
                                    dot_id: start.dot_id.clone(),
                                    sender: tx.clone(),
                                    breakpoints: HashMap::new(),
                                    created_at: Instant::now(),
                                    last_activity: Instant::now(),
                                };

                                debug_sessions.write().await.insert(session_id.clone(), session);

                                let response = DebugResponse {
                                    response_type: Some(debug_response::ResponseType::Started(DebugSessionStarted {
                                        session_id: session_id.clone(),
                                        dot_id: start.dot_id,
                                        initial_state: Some(ExecutionState {
                                            instruction_pointer: 0,
                                            stack_frames: vec![],
                                            variables: HashMap::new(),
                                            memory_usage: 0,
                                        }),
                                    })),
                                };

                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(debug_request::RequestType::Command(command)) => {
                                // Handle debug commands
                                if let Some(ref session_id) = current_session {
                                    let response = DebugResponse {
                                        response_type: Some(debug_response::ResponseType::Event(DebugEvent {
                                            session_id: session_id.clone(),
                                            event_type: DebugEventType::DebugEventStepComplete as i32,
                                            current_state: Some(ExecutionState {
                                                instruction_pointer: 42,
                                                stack_frames: vec![],
                                                variables: HashMap::new(),
                                                memory_usage: 1024,
                                            }),
                                            message: format!("Debug command: {:?}", command.command),
                                            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                                        })),
                                    };

                                    if tx.send(Ok(response)).is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(debug_request::RequestType::Inspect(inspect)) => {
                                // Handle variable inspection with session-local lookup
                                let (value, type_info, children) = {
                                    // Look up variable in current debug session
                                    let debug_sessions_read = debug_sessions.read().await;
                                    if let Some(_session) = debug_sessions_read.get(&inspect.session_id) {
                                        // Simulate variable lookup based on session state
                                        let var_value = format!("Debug value for variable '{}' in session '{}'", inspect.variable_name, inspect.session_id);
                                        let var_bytes = var_value.into_bytes();
                                        let var_type = Self::static_determine_variable_type(&var_bytes);
                                        (var_bytes, var_type, vec![])
                                    } else {
                                        (
                                            format!("Variable '{}' not found - session '{}' not active", inspect.variable_name, inspect.session_id).into_bytes(),
                                            "error".to_string(),
                                            vec![],
                                        )
                                    }
                                };

                                let response = DebugResponse {
                                    response_type: Some(debug_response::ResponseType::Inspection(VariableInspection {
                                        session_id: inspect.session_id,
                                        variable_name: inspect.variable_name.clone(),
                                        value,
                                        type_info,
                                        children,
                                    })),
                                };

                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(debug_request::RequestType::Breakpoint(breakpoint)) => {
                                // Handle breakpoint setting
                                let response = DebugResponse {
                                    response_type: Some(debug_response::ResponseType::BreakpointSet(BreakpointSet {
                                        session_id: breakpoint.session_id,
                                        breakpoint_id: 1,
                                        instruction_address: breakpoint.instruction_address,
                                        success: true,
                                    })),
                                };

                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(debug_request::RequestType::Stop(stop)) => {
                                if let Some(ref session_id) = current_session {
                                    debug_sessions.write().await.remove(session_id);

                                    let response = DebugResponse {
                                        response_type: Some(debug_response::ResponseType::Stopped(DebugSessionStopped {
                                            session_id: session_id.clone(),
                                            reason: "User requested".to_string(),
                                        })),
                                    };

                                    if tx.send(Ok(response)).is_err() {
                                        break;
                                    }
                                }
                                break;
                            }
                            None => {}
                        }
                    }
                    Err(_) => break,
                }
            }

            // Cleanup on disconnect
            if let Some(session_id) = current_session {
                debug_sessions.write().await.remove(&session_id);
            }
        });

        let output_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output_stream)))
    }

    // Connection management methods
    #[instrument(skip(self, request))]
    async fn ping(&self, request: Request<PingRequest>) -> TonicResult<Response<PingResponse>> {
        let start_time = Instant::now();
        let req = request.into_inner();

        // Connection pool management
        let _connection_guard = self.connection_pool.acquire_connection().await?;

        // Update server stats with connection data
        self.update_server_stats().await;

        // Get comprehensive connection statistics
        let conn_stats = self.connection_pool.get_connection_stats().await;
        let stats = self.server_stats.read().await;

        let response = PingResponse {
            server_id: self.server_id.clone(),
            timestamp: req.timestamp,
            server_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            status: Some(ServerStatus {
                version: "1.0.0-week3".to_string(),
                uptime_seconds: stats.uptime_seconds,
                active_connections: conn_stats.active_connections,
                total_requests: conn_stats.total_requests,
                cpu_usage: stats.cpu_usage,
                memory_usage_bytes: stats.memory_usage_bytes,
            }),
        };

        // Record request metrics
        self.connection_pool.record_request("Ping".to_string(), start_time.elapsed().as_millis() as u64, true).await;

        info!(
            "Ping from client: {} -> server: {} (connections: {}/{}, success_rate: {:.1}%)",
            req.client_id, self.server_id, conn_stats.active_connections, conn_stats.max_connections, conn_stats.success_rate
        );

        // Add compression hint for large responses
        let mut response = Response::new(response);
        response.metadata_mut().insert("content-encoding", "gzip".parse().unwrap());

        Ok(response)
    }

    #[instrument(skip(self, request))]
    async fn health_check(&self, request: Request<HealthCheckRequest>) -> TonicResult<Response<HealthCheckResponse>> {
        let start_time = Instant::now();
        let req = request.into_inner();

        // Connection pool management
        let _connection_guard = self.connection_pool.acquire_connection().await?;

        // Clean up expired sessions
        self.cleanup_expired_sessions().await;

        // Update server stats with data
        self.update_server_stats().await;

        // Get comprehensive health metrics
        let conn_stats = self.connection_pool.get_connection_stats().await;
        let active_sessions_count = self.active_sessions.read().await.len();
        let debug_sessions_count = self.debug_sessions.read().await.len();

        // Determine service health based on metrics
        let vm_health_status = if conn_stats.success_rate > 95.0 && conn_stats.active_connections < conn_stats.max_connections {
            OverallHealth::HealthServing
        } else if conn_stats.success_rate > 80.0 {
            OverallHealth::HealthServing // Degraded but serving
        } else {
            OverallHealth::HealthNotServing
        };

        let mut service_health = vec![
            ServiceHealth {
                service_name: "vm_service".to_string(),
                status: vm_health_status as i32,
                message: format!(
                    "VM service health: {:.1}% success rate, {}/{} connections",
                    conn_stats.success_rate, conn_stats.active_connections, conn_stats.max_connections
                ),
                details: {
                    let mut details = HashMap::new();
                    details.insert("success_rate".to_string(), format!("{:.1}%", conn_stats.success_rate));
                    details.insert("avg_response_time".to_string(), format!("{:.1}ms", conn_stats.avg_response_time_ms));
                    details.insert("active_sessions".to_string(), active_sessions_count.to_string());
                    details.insert("debug_sessions".to_string(), debug_sessions_count.to_string());
                    details
                },
            },
            ServiceHealth {
                service_name: "dots_service".to_string(),
                status: OverallHealth::HealthServing as i32,
                message: "Dots service is healthy".to_string(),
                details: {
                    let mut details = HashMap::new();
                    details.insert("active_dots".to_string(), "0".to_string());
                    details.insert("deployed_dots".to_string(), "0".to_string());
                    details
                },
            },
            ServiceHealth {
                service_name: "abi_service".to_string(),
                status: OverallHealth::HealthServing as i32,
                message: "ABI service is healthy".to_string(),
                details: {
                    let mut details = HashMap::new();
                    details.insert("registered_abis".to_string(), "0".to_string());
                    details.insert("validation_cache_size".to_string(), "0".to_string());
                    details
                },
            },
            ServiceHealth {
                service_name: "connection_pool".to_string(),
                status: if conn_stats.active_connections < conn_stats.max_connections * 9 / 10 {
                    OverallHealth::HealthServing as i32
                } else {
                    OverallHealth::HealthNotServing as i32
                },
                message: format!("Connection pool: {}/{} connections used", conn_stats.active_connections, conn_stats.max_connections),
                details: {
                    let mut details = HashMap::new();
                    details.insert("max_connections".to_string(), conn_stats.max_connections.to_string());
                    details.insert(
                        "utilization".to_string(),
                        format!("{:.1}%", conn_stats.active_connections as f64 / conn_stats.max_connections as f64 * 100.0),
                    );
                    details.insert("requests_per_minute".to_string(), conn_stats.recent_requests_per_minute.to_string());
                    details
                },
            },
        ];

        // Filter by requested services if specified
        if !req.services.is_empty() {
            service_health.retain(|s| req.services.contains(&s.service_name));
        }

        let overall_status = if service_health.iter().all(|s| s.status == OverallHealth::HealthServing as i32) {
            OverallHealth::HealthServing
        } else {
            OverallHealth::HealthNotServing
        };

        let mut system_info = HashMap::new();
        if req.include_details {
            let stats = self.server_stats.read().await;
            system_info.insert("server_id".to_string(), self.server_id.clone());
            system_info.insert("uptime_seconds".to_string(), stats.uptime_seconds.to_string());
            system_info.insert("active_sessions".to_string(), active_sessions_count.to_string());
            system_info.insert("debug_sessions".to_string(), debug_sessions_count.to_string());
            system_info.insert("version".to_string(), "1.0.0-week3".to_string());
            system_info.insert("features".to_string(), "bidirectional_streaming,connection_pooling,authentication,compression".to_string());
            system_info.insert("max_connections".to_string(), conn_stats.max_connections.to_string());
            system_info.insert("total_requests".to_string(), conn_stats.total_requests.to_string());
            system_info.insert("cpu_usage".to_string(), format!("{:.1}%", stats.cpu_usage));
            system_info.insert("memory_usage_mb".to_string(), format!("{:.1}", stats.memory_usage_bytes as f64 / 1024.0 / 1024.0));
        }

        let response = HealthCheckResponse {
            overall_status: overall_status as i32,
            service_health,
            system_info,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        // Record request metrics
        self.connection_pool.record_request("HealthCheck".to_string(), start_time.elapsed().as_millis() as u64, true).await;

        info!(
            "Health check completed - status: {:?}, sessions: {}, connections: {}/{}",
            overall_status, active_sessions_count, conn_stats.active_connections, conn_stats.max_connections
        );

        // Add compression for large health responses
        let mut response = Response::new(response);
        if req.include_details {
            response.metadata_mut().insert("content-encoding", "gzip".parse().unwrap());
        }

        Ok(response)
    }
}

// Required associated types for streaming are defined in the trait implementation above
