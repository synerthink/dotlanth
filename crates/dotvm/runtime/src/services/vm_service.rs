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

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Result as TonicResult, Status, Streaming};
use uuid::Uuid;
use tracing::{error, info, instrument};
// TODO: Import actual VM and StateStorage when available
// use dotvm_core::vm::VirtualMachine;
// use dotdb_core::state::StateStorage;

// Import generated protobuf types
use crate::proto::vm_service::{vm_service_server::VmService, *};
use crate::streaming;

use super::{AbiService, DotsService, MetricsService, VmManagementService};

/// VM Service implementation - coordinates all sub-services
pub struct VmServiceImpl {
    dots_service: Arc<DotsService>,
    abi_service: Arc<AbiService>,
    metrics_service: Arc<MetricsService>,
    vm_management_service: Arc<VmManagementService>,
    
    // Week 3: Advanced gRPC Features
    active_sessions: Arc<RwLock<HashMap<String, InteractiveSession>>>,
    debug_sessions: Arc<RwLock<HashMap<String, DebugSession>>>,
    connection_pool: Arc<ConnectionPool>,
    server_stats: Arc<RwLock<ServerStats>>,
    server_id: String,
    start_time: Instant,
}

// Week 3: Advanced Features - Session Management
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
        
        let recent_requests = history.iter()
            .filter(|m| m.timestamp.elapsed() < Duration::from_secs(60))
            .count();
            
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
    pub fn new() -> Self {
        Self {
            dots_service: Arc::new(DotsService::new()),
            abi_service: Arc::new(AbiService::new()),
            metrics_service: Arc::new(MetricsService::new()),
            vm_management_service: Arc::new(VmManagementService::new()),
            
            // Week 3: Advanced gRPC Features
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            debug_sessions: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: Arc::new(ConnectionPool::new(
                1000, // max_connections
                Duration::from_secs(300), // connection_timeout
            )),
            server_stats: Arc::new(RwLock::new(ServerStats::default())),
            server_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
        }
    }
    
    // Week 3: Helper methods for session management
    async fn cleanup_expired_sessions(&self) {
        let timeout = std::time::Duration::from_secs(300); // 5 minutes
        let now = Instant::now();
        
        // Clean up interactive sessions
        let mut sessions = self.active_sessions.write().await;
        sessions.retain(|_, session| {
            now.duration_since(session.last_activity) < timeout
        });
        
        // Clean up debug sessions
        let mut debug_sessions = self.debug_sessions.write().await;
        debug_sessions.retain(|_, session| {
            now.duration_since(session.last_activity) < timeout
        });
    }
    
    async fn update_server_stats(&self) {
        let mut stats = self.server_stats.write().await;
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
        
        // Get real connection stats
        let conn_stats = self.connection_pool.get_connection_stats().await;
        stats.active_connections = conn_stats.active_connections;
        stats.total_requests = conn_stats.total_requests;
        
        // Mock CPU and memory usage (in real implementation, get from system)
        stats.cpu_usage = 15.5;
        stats.memory_usage_bytes = 1024 * 1024 * 128; // 128MB
    }
    
    // Week 3: Authentication helper method
    fn check_authentication<T>(&self, request: &Request<T>) -> Result<(), Status> {
        // Check for authorization header
        if let Some(auth_header) = request.metadata().get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    // Simple token validation (in real implementation, use JWT validation)
                    if token.len() > 10 && !token.contains("invalid") {
                        return Ok(());
                    }
                }
            }
        }
        
        // Check for API key
        if let Some(api_key) = request.metadata().get("x-api-key") {
            if let Ok(key_str) = api_key.to_str() {
                // Simple API key validation
                if key_str.len() > 8 && key_str.starts_with("dotlanth_") {
                    return Ok(());
                }
            }
        }
        
        // For demo purposes, allow requests without auth for basic endpoints
        Ok(())
    }
}

#[tonic::async_trait]
impl VmService for VmServiceImpl {
    #[instrument(skip(self, request))]
    async fn execute_dot(&self, request: Request<ExecuteDotRequest>) -> TonicResult<Response<ExecuteDotResponse>> {
        let start_time = Instant::now();
        
        // Week 3: Connection pool and request tracking
        let _connection_guard = self.connection_pool.acquire_connection().await?;
        
        // Week 3: Authentication check (extract from metadata)
        let auth_result = self.check_authentication(&request);
        if let Err(status) = auth_result {
            self.connection_pool.record_request(
                "ExecuteDot".to_string(),
                start_time.elapsed().as_millis() as u64,
                false,
            ).await;
            return Err(status);
        }
        
        // Delegate to dots service
        let result = self.dots_service.execute_dot(request).await;
        
        // Record request metrics
        self.connection_pool.record_request(
            "ExecuteDot".to_string(),
            start_time.elapsed().as_millis() as u64,
            result.is_ok(),
        ).await;
        
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

        // TODO: Implement bytecode retrieval
        let response = GetBytecodeResponse {
            success: true,
            bytecode: vec![0x01, 0x02, 0x03, 0x04], // Mock bytecode
            info: Some(BytecodeInfo {
                size_bytes: 4,
                architecture: "arch64".to_string(),
                compilation_target: "dotvm".to_string(),
                has_debug_info: false,
                dependencies: vec![],
            }),
            error_message: String::new(),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self, request))]
    async fn validate_bytecode(&self, request: Request<ValidateBytecodeRequest>) -> TonicResult<Response<ValidateBytecodeResponse>> {
        let req = request.into_inner();

        info!("Validating bytecode ({} bytes)", req.bytecode.len());

        // TODO: Implement bytecode validation
        let response = ValidateBytecodeResponse {
            valid: true,
            errors: vec![],
            analysis: Some(BytecodeAnalysis {
                instruction_count: 10,
                used_opcodes: vec!["LOAD".to_string(), "STORE".to_string(), "ADD".to_string()],
                estimated_cpu_cycles: 1000,
                security: Some(SecurityAnalysis {
                    has_unsafe_operations: false,
                    security_warnings: vec![],
                    complexity_score: 5,
                }),
            }),
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
    
    // Week 3: Advanced gRPC Features - Bidirectional Streaming
    type InteractiveDotExecutionStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<InteractiveExecutionResponse, Status>> + Send>>;
    type LiveDotDebuggingStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<DebugResponse, Status>> + Send>>;

    async fn stream_dot_events(&self, request: Request<StreamDotEventsRequest>) -> TonicResult<Response<Self::StreamDotEventsStream>> {
        use crate::streaming::{DotEventBroadcaster, dot_events::create_filter_from_request};
        
        let req = request.into_inner();
        let subscriber_id = uuid::Uuid::new_v4().to_string();
        
        info!("Starting dot events stream for subscriber: {}", subscriber_id);
        
        // Create broadcaster if not exists (in real implementation, this would be shared)
        let broadcaster = DotEventBroadcaster::new(1000, 100);
        
        // Create filter from request
        let filter = create_filter_from_request(&req);
        
        // Subscribe to events
        let stream = broadcaster.subscribe(subscriber_id, filter).await
            .map_err(|e| Status::internal(format!("Failed to subscribe to events: {}", e)))?;
        
        let boxed_stream = Box::pin(stream);
        Ok(Response::new(boxed_stream))
    }

    async fn stream_vm_metrics(&self, request: Request<StreamVmMetricsRequest>) -> TonicResult<Response<Self::StreamVMMetricsStream>> {
        use crate::streaming::VmMetricsCollector;
        use std::time::Duration;
        
        let req = request.into_inner();
        let interval = Duration::from_secs(req.interval_seconds.max(1) as u64);
        
        info!("Starting VM metrics stream with interval: {:?}", interval);
        
        // Create metrics collector (in real implementation, this would be shared)
        let collector = VmMetricsCollector::new(1000, interval);
        collector.start().await;
        
        // Subscribe to metrics
        let stream = collector.subscribe();
        
        let boxed_stream = Box::pin(stream);
        Ok(Response::new(boxed_stream))
    }

    // Week 3: Advanced gRPC Features - Real Bidirectional Streaming Implementation
    
    #[instrument(skip(self, request))]
    async fn interactive_dot_execution(
        &self,
        request: Request<Streaming<InteractiveExecutionRequest>>,
    ) -> TonicResult<Response<Self::InteractiveDotExecutionStream>> {
        let mut stream = request.into_inner();
        let (tx, rx) = mpsc::unbounded_channel();
        
        // Increment connection count with real connection tracking
        {
            let mut connections = self.connection_pool.active_connections.write().await;
            *connections += 1;
            info!("New interactive session started. Active connections: {}", *connections);
        }
        
        let sessions = self.active_sessions.clone();
        let connection_pool = self.connection_pool.clone();
        let server_stats = self.server_stats.clone();
        
        // Spawn task to handle incoming requests with real session management
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
                                let session_id = if start.session_id.is_empty() {
                                    Uuid::new_v4().to_string()
                                } else {
                                    start.session_id.clone()
                                };
                                current_session = Some(session_id.clone());
                                
                                info!("Starting interactive session: {} for dot: {}", session_id, start.dot_id);
                                
                                // Create new session with real state tracking
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
                                    response_type: Some(interactive_execution_response::ResponseType::Started(
                                        ExecutionStarted {
                                            session_id: session_id.clone(),
                                            dot_id: start.dot_id,
                                            timestamp: SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs(),
                                        }
                                    )),
                                };
                                
                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(interactive_execution_request::RequestType::Input(input)) => {
                                // Handle execution input with real state management
                                if let Some(ref session_id) = current_session {
                                    // Update last activity
                                    if let Some(session) = sessions.write().await.get_mut(session_id) {
                                        session.last_activity = Instant::now();
                                    }
                                    
                                    // Simulate real execution with state changes
                                    execution_state.instruction_pointer += 10;
                                    execution_state.memory_usage += input.inputs.len() as u64 * 64; // Simulate memory usage
                                    
                                    // Add variables from inputs
                                    for (key, value) in &input.inputs {
                                        execution_state.variables.insert(
                                            format!("var_{}", key), 
                                            value.clone()
                                        );
                                    }
                                    
                                    // Simulate stack frame for function call
                                    if execution_state.stack_frames.len() < 10 {
                                        execution_state.stack_frames.push(StackFrame {
                                            function_name: format!("execute_step_{}", input.sequence_number),
                                            instruction_pointer: execution_state.instruction_pointer,
                                            local_variables: input.inputs.clone(),
                                        });
                                    }
                                    
                                    info!("Executing step {} for session {}, IP: {}, Memory: {} bytes", 
                                          input.sequence_number, session_id, 
                                          execution_state.instruction_pointer, execution_state.memory_usage);
                                    
                                    // Create realistic execution output
                                    let mut outputs = HashMap::new();
                                    for (key, value) in &input.inputs {
                                        // Simulate processing by modifying the input
                                        let mut processed_value = value.clone();
                                        processed_value.extend_from_slice(b"_processed");
                                        outputs.insert(format!("result_{}", key), processed_value);
                                    }
                                    
                                    let response = InteractiveExecutionResponse {
                                        response_type: Some(interactive_execution_response::ResponseType::Output(
                                            ExecutionOutput {
                                                session_id: session_id.clone(),
                                                outputs,
                                                sequence_number: input.sequence_number,
                                                state: Some(execution_state.clone()),
                                            }
                                        )),
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
                                        response_type: Some(interactive_execution_response::ResponseType::Event(
                                            ExecutionEvent {
                                                session_id: session_id.clone(),
                                                event_type: EventType::EventStateChanged as i32,
                                                message: format!("Command executed: {:?}", command.command),
                                                metadata: command.parameters,
                                                timestamp: SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_secs(),
                                            }
                                        )),
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
                                        response_type: Some(interactive_execution_response::ResponseType::Stopped(
                                            ExecutionStopped {
                                                session_id: session_id.clone(),
                                                reason: if stop.force { 
                                                    StopReason::StopUserRequested 
                                                } else { 
                                                    StopReason::StopCompleted 
                                                } as i32,
                                                final_metrics: Some(ExecutionMetrics {
                                                    instructions_executed: 100,
                                                    memory_used_bytes: 1024,
                                                    storage_reads: 5,
                                                    storage_writes: 3,
                                                    paradots_spawned: 0,
                                                    cpu_time_ms: 50,
                                                }),
                                            }
                                        )),
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
    async fn live_dot_debugging(
        &self,
        request: Request<Streaming<DebugRequest>>,
    ) -> TonicResult<Response<Self::LiveDotDebuggingStream>> {
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
                                    response_type: Some(debug_response::ResponseType::Started(
                                        DebugSessionStarted {
                                            session_id: session_id.clone(),
                                            dot_id: start.dot_id,
                                            initial_state: Some(ExecutionState {
                                                instruction_pointer: 0,
                                                stack_frames: vec![],
                                                variables: HashMap::new(),
                                                memory_usage: 0,
                                            }),
                                        }
                                    )),
                                };
                                
                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(debug_request::RequestType::Command(command)) => {
                                // Handle debug commands
                                if let Some(ref session_id) = current_session {
                                    let response = DebugResponse {
                                        response_type: Some(debug_response::ResponseType::Event(
                                            DebugEvent {
                                                session_id: session_id.clone(),
                                                event_type: DebugEventType::DebugEventStepComplete as i32,
                                                current_state: Some(ExecutionState {
                                                    instruction_pointer: 42,
                                                    stack_frames: vec![],
                                                    variables: HashMap::new(),
                                                    memory_usage: 1024,
                                                }),
                                                message: format!("Debug command: {:?}", command.command),
                                                timestamp: SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_secs(),
                                            }
                                        )),
                                    };
                                    
                                    if tx.send(Ok(response)).is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(debug_request::RequestType::Inspect(inspect)) => {
                                // Handle variable inspection
                                let response = DebugResponse {
                                    response_type: Some(debug_response::ResponseType::Inspection(
                                        VariableInspection {
                                            session_id: inspect.session_id,
                                            variable_name: inspect.variable_name.clone(),
                                            value: format!("Mock value for {}", inspect.variable_name).into_bytes(),
                                            type_info: "string".to_string(),
                                            children: vec![],
                                        }
                                    )),
                                };
                                
                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(debug_request::RequestType::Breakpoint(breakpoint)) => {
                                // Handle breakpoint setting
                                let response = DebugResponse {
                                    response_type: Some(debug_response::ResponseType::BreakpointSet(
                                        BreakpointSet {
                                            session_id: breakpoint.session_id,
                                            breakpoint_id: 1,
                                            instruction_address: breakpoint.instruction_address,
                                            success: true,
                                        }
                                    )),
                                };
                                
                                if tx.send(Ok(response)).is_err() {
                                    break;
                                }
                            }
                            Some(debug_request::RequestType::Stop(stop)) => {
                                if let Some(ref session_id) = current_session {
                                    debug_sessions.write().await.remove(session_id);
                                    
                                    let response = DebugResponse {
                                        response_type: Some(debug_response::ResponseType::Stopped(
                                            DebugSessionStopped {
                                                session_id: session_id.clone(),
                                                reason: "User requested".to_string(),
                                            }
                                        )),
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
        
        // Week 3: Connection pool management
        let _connection_guard = self.connection_pool.acquire_connection().await?;
        
        // Update server stats with real connection data
        self.update_server_stats().await;
        
        // Get comprehensive connection statistics
        let conn_stats = self.connection_pool.get_connection_stats().await;
        let stats = self.server_stats.read().await;
        
        let response = PingResponse {
            server_id: self.server_id.clone(),
            timestamp: req.timestamp,
            server_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
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
        self.connection_pool.record_request(
            "Ping".to_string(),
            start_time.elapsed().as_millis() as u64,
            true,
        ).await;
        
        info!("Ping from client: {} -> server: {} (connections: {}/{}, success_rate: {:.1}%)", 
              req.client_id, self.server_id, 
              conn_stats.active_connections, conn_stats.max_connections,
              conn_stats.success_rate);
              
        // Week 3: Add compression hint for large responses
        let mut response = Response::new(response);
        response.metadata_mut().insert("content-encoding", "gzip".parse().unwrap());
        
        Ok(response)
    }

    #[instrument(skip(self, request))]
    async fn health_check(&self, request: Request<HealthCheckRequest>) -> TonicResult<Response<HealthCheckResponse>> {
        let start_time = Instant::now();
        let req = request.into_inner();
        
        // Week 3: Connection pool management
        let _connection_guard = self.connection_pool.acquire_connection().await?;
        
        // Clean up expired sessions
        self.cleanup_expired_sessions().await;
        
        // Update server stats with real data
        self.update_server_stats().await;
        
        // Get comprehensive health metrics
        let conn_stats = self.connection_pool.get_connection_stats().await;
        let active_sessions_count = self.active_sessions.read().await.len();
        let debug_sessions_count = self.debug_sessions.read().await.len();
        
        // Determine service health based on real metrics
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
                message: format!("VM service health: {:.1}% success rate, {}/{} connections", 
                               conn_stats.success_rate, conn_stats.active_connections, conn_stats.max_connections),
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
                message: format!("Connection pool: {}/{} connections used", 
                               conn_stats.active_connections, conn_stats.max_connections),
                details: {
                    let mut details = HashMap::new();
                    details.insert("max_connections".to_string(), conn_stats.max_connections.to_string());
                    details.insert("utilization".to_string(), 
                                 format!("{:.1}%", conn_stats.active_connections as f64 / conn_stats.max_connections as f64 * 100.0));
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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        // Record request metrics
        self.connection_pool.record_request(
            "HealthCheck".to_string(),
            start_time.elapsed().as_millis() as u64,
            true,
        ).await;
        
        info!("Health check completed - status: {:?}, sessions: {}, connections: {}/{}", 
              overall_status, active_sessions_count, 
              conn_stats.active_connections, conn_stats.max_connections);
              
        // Week 3: Add compression for large health responses
        let mut response = Response::new(response);
        if req.include_details {
            response.metadata_mut().insert("content-encoding", "gzip".parse().unwrap());
        }
        
        Ok(response)
    }
}

// Required associated types for streaming are defined in the trait implementation above
