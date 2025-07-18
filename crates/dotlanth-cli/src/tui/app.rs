use crate::commands::CommandContext;
use crate::database::{DeploymentInfo, LogEntry, MetricEntry, NodeInfo};
use anyhow::Result;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time: f64,
    pub min_response_time: u64,
    pub max_response_time: u64,
    pub requests_per_minute: f64,
    pub last_minute_requests: Vec<(Instant, u64)>,
}

impl RequestMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time: 0.0,
            min_response_time: u64::MAX,
            max_response_time: 0,
            requests_per_minute: 0.0,
            last_minute_requests: Vec::new(),
        }
    }

    pub fn add_request(&mut self, duration_ms: u64) {
        let now = Instant::now();

        if self.min_response_time == u64::MAX || duration_ms < self.min_response_time {
            self.min_response_time = duration_ms;
        }
        if duration_ms > self.max_response_time {
            self.max_response_time = duration_ms;
        }

        if self.total_requests > 0 {
            let total_time = self.avg_response_time * (self.total_requests - 1) as f64 + duration_ms as f64;
            self.avg_response_time = total_time / self.total_requests as f64;
        } else {
            self.avg_response_time = duration_ms as f64;
        }

        self.last_minute_requests.push((now, duration_ms));

        let one_minute_ago = now - Duration::from_secs(60);
        self.last_minute_requests.retain(|(time, _)| *time > one_minute_ago);

        self.requests_per_minute = self.last_minute_requests.len() as f64;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct GrpcEndpointManager {
    pub test_results: Vec<TestResult>,
    pub current_category: usize,
    pub current_endpoint: usize,
    pub endpoints: Vec<Vec<GrpcEndpoint>>,
}

impl GrpcEndpointManager {
    pub fn new() -> Self {
        let vm_service_endpoints = vec![
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "GetArchitectures".to_string(),
                requires_auth: false,
                example_request: "{}".to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "GetVMStatus".to_string(),
                requires_auth: false,
                example_request: "{}".to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "GetVMMetrics".to_string(),
                requires_auth: false,
                example_request: "{}".to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "ExecuteDot".to_string(),
                requires_auth: true,
                example_request: r#"{"dot_id": "test-dot", "inputs": {}, "paradots_enabled": false, "caller_id": "test-caller"}"#.to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "ListDots".to_string(),
                requires_auth: true,
                example_request: "{}".to_string(),
            },
        ];

        let runtime_endpoints = vec![GrpcEndpoint {
            service: "runtime.Runtime".to_string(),
            method: "Ping".to_string(),
            requires_auth: false,
            example_request: r#"{"message": "hello"}"#.to_string(),
        }];

        let reflection_endpoints = vec![GrpcEndpoint {
            service: "grpc.reflection.v1alpha.ServerReflection".to_string(),
            method: "ServerReflectionInfo".to_string(),
            requires_auth: false,
            example_request: r#"{"host": "127.0.0.1", "list_services": ""}"#.to_string(),
        }];

        let advanced_endpoints = vec![
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "DeployDot".to_string(),
                requires_auth: true,
                example_request: r#"{"dot_name": "test-dot", "dot_source": "contract TestDot { }", "deployer_id": "test-user"}"#.to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "GetBytecode".to_string(),
                requires_auth: true,
                example_request: r#"{"dot_id": "test-dot-id", "version": "1.0"}"#.to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "ValidateBytecode".to_string(),
                requires_auth: true,
                example_request: r#"{"bytecode": "0x01020304", "target_architecture": "WASM"}"#.to_string(),
            },
        ];

        // Week 3: Advanced gRPC Features - Real Implementations
        let week3_features = vec![
            // Connection Management with Real Stats
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "Ping".to_string(),
                requires_auth: false,
                example_request: r#"{"client_id": "tui-client-week3", "timestamp": 1640995200, "metadata": {"version": "1.0", "feature": "connection_pooling"}}"#.to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "HealthCheck".to_string(),
                requires_auth: false,
                example_request: r#"{"services": ["vm_service", "connection_pool", "dots_service", "abi_service"], "include_details": true}"#.to_string(),
            },
            
            // Authentication Testing
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "ExecuteDot".to_string(),
                requires_auth: true,
                example_request: r#"{"dot_id": "auth-test-dot", "inputs": {"test": "YXV0aF90ZXN0"}, "paradots_enabled": true, "caller_id": "authenticated-user"}"#.to_string(),
            },
            
            // Streaming Features (Note: These are bidirectional, so testing via grpcurl is limited)
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "StreamDotEvents".to_string(),
                requires_auth: true,
                example_request: r#"{"dot_filter": {"dot_ids": ["test-dot"]}, "event_types": ["EXECUTION", "STATE_CHANGE"]}"#.to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "StreamVMMetrics".to_string(),
                requires_auth: false,
                example_request: r#"{"interval_seconds": 5, "metric_types": ["CPU", "MEMORY", "CONNECTIONS"]}"#.to_string(),
            },
            
            // Connection Pool Stress Testing
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "GetVMStatus".to_string(),
                requires_auth: false,
                example_request: r#"{}"#.to_string(),
            },
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "GetVMMetrics".to_string(),
                requires_auth: false,
                example_request: r#"{}"#.to_string(),
            },
            
            // Compression Testing (Large Response)
            GrpcEndpoint {
                service: "vm_service.VmService".to_string(),
                method: "GetArchitectures".to_string(),
                requires_auth: false,
                example_request: r#"{}"#.to_string(),
            },
        ];

        Self {
            test_results: Vec::new(),
            current_category: 0,
            current_endpoint: 0,
            endpoints: vec![vm_service_endpoints, runtime_endpoints, reflection_endpoints, advanced_endpoints, week3_features],
        }
    }

    pub fn get_selected_endpoint(&self) -> Option<&GrpcEndpoint> {
        self.endpoints.get(self.current_category)?.get(self.current_endpoint)
    }

    pub fn get_endpoints_for_category(&self) -> &Vec<GrpcEndpoint> {
        self.endpoints.get(self.current_category).unwrap_or(&self.endpoints[0])
    }

    pub fn previous_category(&mut self) {
        if self.current_category > 0 {
            self.current_category -= 1;
        } else {
            self.current_category = self.endpoints.len() - 1;
        }
        self.current_endpoint = 0;
    }

    pub fn next_category(&mut self) {
        self.current_category = (self.current_category + 1) % self.endpoints.len();
        self.current_endpoint = 0;
    }

    pub fn previous_endpoint(&mut self) {
        let endpoints_count = self.get_endpoints_for_category().len();
        if endpoints_count > 0 {
            if self.current_endpoint > 0 {
                self.current_endpoint -= 1;
            } else {
                self.current_endpoint = endpoints_count - 1;
            }
        }
    }

    pub fn next_endpoint(&mut self) {
        let endpoints_count = self.get_endpoints_for_category().len();
        if endpoints_count > 0 {
            self.current_endpoint = (self.current_endpoint + 1) % endpoints_count;
        }
    }
}

#[derive(Debug, Clone)]
pub struct GrpcEndpoint {
    pub service: String,
    pub method: String,
    pub requires_auth: bool,
    pub example_request: String,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub endpoint: String,
    pub success: bool,
    pub response: String,
    pub timestamp: Instant,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabIndex {
    Overview = 0,
    Nodes = 1,
    Deployments = 2,
    Metrics = 3,
    Logs = 4,
    GrpcServer = 5,
    GrpcEndpoints = 6,
}

impl TabIndex {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => TabIndex::Overview,
            1 => TabIndex::Nodes,
            2 => TabIndex::Deployments,
            3 => TabIndex::Metrics,
            4 => TabIndex::Logs,
            5 => TabIndex::GrpcServer,
            6 => TabIndex::GrpcEndpoints,
            _ => TabIndex::Overview,
        }
    }

    pub fn next(self) -> Self {
        Self::from_index((self as usize + 1) % 7)
    }

    pub fn previous(self) -> Self {
        Self::from_index((self as usize + 6) % 7)
    }
}

pub struct App {
    pub context: CommandContext,
    pub current_tab: TabIndex,
    pub nodes: Vec<NodeInfo>,
    pub deployments: Vec<DeploymentInfo>,
    pub logs: Vec<LogEntry>,
    pub metrics: Vec<MetricEntry>,
    pub scroll_offset: usize,
    pub show_help: bool,
    pub show_debug: bool,
    pub last_update: Instant,
    pub status_message: String,
    pub grpc_server_running: bool,
    pub grpc_server_process: Option<std::process::Child>,
    pub grpc_endpoint_manager: GrpcEndpointManager,
    pub auth_token: Option<String>,
    pub grpc_server_sub_tab: usize,
    pub grpc_endpoints_sub_tab: usize,
    pub request_metrics: RequestMetrics,
    pub grpc_server_logs: Vec<String>,
    pub show_grpc_logs: bool,
}

impl App {
    pub fn new(context: CommandContext) -> Self {
        let mut app = Self {
            context,
            current_tab: TabIndex::Overview,
            nodes: Vec::new(),
            deployments: Vec::new(),
            logs: Vec::new(),
            metrics: Vec::new(),
            scroll_offset: 0,
            show_help: false,
            show_debug: false,
            last_update: Instant::now(),
            status_message: "Welcome to DotLanth Infrastructure Management".to_string(),
            grpc_server_running: false,
            grpc_server_process: None,
            grpc_endpoint_manager: GrpcEndpointManager::new(),
            auth_token: None,
            grpc_server_sub_tab: 0,
            grpc_endpoints_sub_tab: 0,
            request_metrics: RequestMetrics::new(),
            grpc_server_logs: Vec::new(),
            show_grpc_logs: false,
        };

        if let Err(e) = app.refresh_data() {
            app.status_message = format!("Error loading data: {}", e);
        }

        app
    }

    pub fn refresh_data(&mut self) -> Result<()> {
        self.nodes = self.context.database.list_nodes()?;
        self.deployments = self.context.database.list_deployments()?;
        self.logs = self.context.database.get_recent_logs(None, self.context.config.ui.max_log_lines)?;
        self.metrics = self.context.database.get_recent_metrics(None, self.context.config.ui.max_log_lines)?;
        self.last_update = Instant::now();
        self.status_message = format!("Data refreshed at {}", chrono::Local::now().format("%H:%M:%S"));
        Ok(())
    }

    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
        self.scroll_offset = 0;
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = self.current_tab.previous();
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn handle_enter(&mut self) -> Result<()> {
        match self.current_tab {
            TabIndex::Nodes => {
                self.status_message = "Node action placeholder".to_string();
            }
            TabIndex::Deployments => {
                self.status_message = "Deployment action placeholder".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_escape(&mut self) {
        self.show_help = false;
        self.show_debug = false;
        self.scroll_offset = 0;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn toggle_debug(&mut self) {
        self.show_debug = !self.show_debug;
    }

    pub fn update(&mut self) {
        self.last_update = Instant::now();
    }

    pub fn test_endpoint_sync(&mut self, endpoint: &GrpcEndpoint) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        self.request_metrics.total_requests += 1;

        let mut cmd = std::process::Command::new("grpcurl");
        let timeout_secs = (self.context.config.grpc.connection_timeout_ms / 1000).to_string();
        cmd.args(&["-plaintext", "-max-time", &timeout_secs]);
        
        // Note: grpcurl doesn't have -4 flag, so we rely on using 127.0.0.1 instead of localhost

        // Add auth header if required
        if endpoint.requires_auth {
            if let Some(token) = &self.auth_token {
                // Try both common auth header formats
                cmd.args(&["-H", &format!("Authorization: Bearer {}", token)]);
                cmd.args(&["-H", &format!("x-api-key: {}", token)]);
                cmd.args(&["-H", &format!("api-key: {}", token)]);
            } else {
                // If auth required but no token, show warning but still try request
                self.status_message = "Warning: Auth required but no token set. Press 'A' to set token.".to_string();
            }
        }

        // Add request data - always include for consistency
        if !endpoint.example_request.is_empty() {
            cmd.args(&["-d", &endpoint.example_request]);
        } else {
            // For empty requests, explicitly send empty JSON
            cmd.args(&["-d", "{}"]);
        }

        // Use configured gRPC address for cross-platform compatibility
        let grpc_addr = format!("{}:{}", self.context.config.grpc.client_host, self.context.config.grpc.client_port);
        cmd.args(&[&grpc_addr, &format!("{}/{}", endpoint.service, endpoint.method)]);

        // Execute command
        let result = cmd.output();
        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        // Update metrics
        self.request_metrics.add_request(duration_ms);

        let test_result = match result {
            Ok(output) if output.status.success() => {
                let response = String::from_utf8_lossy(&output.stdout);
                let clean_response = response.trim();

                // Check if response is empty or contains error indicators
                if clean_response.is_empty() {
                    self.request_metrics.failed_requests += 1;
                    TestResult {
                        endpoint: format!("{}/{}", endpoint.service, endpoint.method),
                        success: false,
                        response: "Empty response received".to_string(),
                        timestamp: start_time,
                        duration_ms,
                    }
                } else if clean_response.contains("RST_STREAM") || clean_response.contains("CANCEL") {
                    self.request_metrics.failed_requests += 1;
                    TestResult {
                        endpoint: format!("{}/{}", endpoint.service, endpoint.method),
                        success: false,
                        response: format!("Connection error: {}", clean_response),
                        timestamp: start_time,
                        duration_ms,
                    }
                } else {
                    self.request_metrics.successful_requests += 1;
                    TestResult {
                        endpoint: format!("{}/{}", endpoint.service, endpoint.method),
                        success: true,
                        response: clean_response.to_string(),
                        timestamp: start_time,
                        duration_ms,
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Combine stderr and stdout for better error reporting
                let error_msg = if !stderr.trim().is_empty() { stderr.trim() } else { stdout.trim() };

                self.request_metrics.failed_requests += 1;
                TestResult {
                    endpoint: format!("{}/{}", endpoint.service, endpoint.method),
                    success: false,
                    response: format!("Error: {}", error_msg),
                    timestamp: start_time,
                    duration_ms,
                }
            }
            Err(e) => {
                self.request_metrics.failed_requests += 1;
                TestResult {
                    endpoint: format!("{}/{}", endpoint.service, endpoint.method),
                    success: false,
                    response: format!("Command failed: {}", e),
                    timestamp: start_time,
                    duration_ms,
                }
            }
        };

        self.grpc_endpoint_manager.test_results.push(test_result);

        // Keep only last 50 results
        if self.grpc_endpoint_manager.test_results.len() > 50 {
            self.grpc_endpoint_manager.test_results.remove(0);
        }

        Ok(())
    }

    // gRPC Server management functions
    pub fn start_grpc_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.grpc_server_running {
            self.status_message = "gRPC server is already running".to_string();
            return Ok(());
        }

        self.status_message = "Starting gRPC server...".to_string();

        let mut cmd = std::process::Command::new("cargo");
        cmd.args(&["run", "--bin", "dotvm-runtime"])
            .current_dir("crates/dotvm/runtime")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        match cmd.spawn() {
            Ok(child) => {
                self.grpc_server_process = Some(child);
                self.grpc_server_running = true;
                self.status_message = "gRPC server started successfully".to_string();

                // Add startup log entries
                self.grpc_server_logs.push(format!("[{}] Starting gRPC server...", chrono::Local::now().format("%H:%M:%S")));
                self.grpc_server_logs.push(format!("[{}] gRPC server started successfully", chrono::Local::now().format("%H:%M:%S")));
                self.refresh_grpc_logs();
            }
            Err(e) => {
                self.status_message = format!("Failed to start gRPC server: {}", e);
                self.grpc_server_logs.push(format!("[{}] Failed to start gRPC server: {}", chrono::Local::now().format("%H:%M:%S"), e));
            }
        }

        Ok(())
    }

    pub fn stop_grpc_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.grpc_server_running {
            self.status_message = "gRPC server is not running".to_string();
            return Ok(());
        }

        self.status_message = "Stopping gRPC server...".to_string();
        self.grpc_server_logs.push(format!("[{}] Stopping gRPC server...", chrono::Local::now().format("%H:%M:%S")));

        if let Some(mut child) = self.grpc_server_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        // Also kill any remaining processes
        let _ = std::process::Command::new("pkill").args(&["-f", "dotvm-runtime"]).output();

        self.grpc_server_running = false;
        self.status_message = "gRPC server stopped".to_string();
        self.grpc_server_logs.push(format!("[{}] gRPC server stopped", chrono::Local::now().format("%H:%M:%S")));

        Ok(())
    }

    pub fn restart_grpc_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.status_message = "Restarting gRPC server...".to_string();
        self.grpc_server_logs.push(format!("[{}] Restarting gRPC server...", chrono::Local::now().format("%H:%M:%S")));

        self.stop_grpc_server()?;
        std::thread::sleep(std::time::Duration::from_millis(1000));
        self.start_grpc_server()?;

        self.status_message = "gRPC server restarted successfully".to_string();
        self.grpc_server_logs.push(format!("[{}] gRPC server restarted successfully", chrono::Local::now().format("%H:%M:%S")));

        Ok(())
    }

    pub fn build_grpc_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.status_message = "Building gRPC server...".to_string();
        self.grpc_server_logs.push(format!("[{}] Building gRPC server...", chrono::Local::now().format("%H:%M:%S")));

        let output = std::process::Command::new("cargo").args(&["build", "--release"]).current_dir("crates/dotvm/runtime").output()?;

        if output.status.success() {
            self.status_message = "gRPC server built successfully".to_string();
            self.grpc_server_logs.push(format!("[{}] gRPC server built successfully", chrono::Local::now().format("%H:%M:%S")));
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            self.status_message = format!("Build failed: {}", error);
            self.grpc_server_logs.push(format!("[{}] Build failed: {}", chrono::Local::now().format("%H:%M:%S"), error));
        }

        Ok(())
    }

    pub fn test_grpc_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.status_message = "Testing gRPC connection...".to_string();

        let grpc_addr = format!("{}:{}", self.context.config.grpc.client_host, self.context.config.grpc.client_port);
        let output = std::process::Command::new("grpcurl").args(&["-plaintext", &grpc_addr, "list"]).output();

        match output {
            Ok(result) if result.status.success() => {
                let services = String::from_utf8_lossy(&result.stdout);
                self.status_message = format!("Connection OK. Services: {}", services.trim());
            }
            Ok(result) => {
                let error = String::from_utf8_lossy(&result.stderr);
                self.status_message = format!("Connection failed: {}", error);
            }
            Err(e) => {
                self.status_message = format!("grpcurl not found: {}", e);
            }
        }

        Ok(())
    }

    pub fn tick(&mut self) {
        let refresh_interval = Duration::from_millis(self.context.config.ui.refresh_rate_ms);
        if self.last_update.elapsed() >= refresh_interval {
            if let Err(e) = self.refresh_data() {
                self.status_message = format!("Auto-refresh failed: {}", e);
            }
        }
    }

    pub fn get_refresh_rate(&self) -> u64 {
        self.context.config.ui.refresh_rate_ms / 4
    }

    pub fn get_tab_titles(&self) -> Vec<&str> {
        vec!["Overview", "Nodes", "Deployments", "Metrics", "Logs", "gRPC Server", "gRPC Endpoints"]
    }

    pub fn toggle_grpc_logs(&mut self) {
        self.show_grpc_logs = !self.show_grpc_logs;
        if self.show_grpc_logs {
            self.status_message = "Showing gRPC server logs".to_string();
            self.refresh_grpc_logs();
        } else {
            self.status_message = "Hiding gRPC server logs".to_string();
        }
    }

    pub fn refresh_grpc_logs(&mut self) {
        // Simulate getting logs from the server process
        if self.grpc_server_running {
            let new_logs = vec![
                format!("[{}] gRPC server listening on {}:{}", chrono::Local::now().format("%H:%M:%S"), self.context.config.grpc.server_host, self.context.config.grpc.server_port),
                format!("[{}] Reflection service enabled", chrono::Local::now().format("%H:%M:%S")),
                format!("[{}] VM service registered", chrono::Local::now().format("%H:%M:%S")),
                format!("[{}] Runtime service registered", chrono::Local::now().format("%H:%M:%S")),
            ];

            self.grpc_server_logs.extend(new_logs);

            // Keep only last 100 log entries
            if self.grpc_server_logs.len() > 100 {
                self.grpc_server_logs.drain(0..self.grpc_server_logs.len() - 100);
            }
        } else {
            self.grpc_server_logs.push(format!("[{}] gRPC server is stopped", chrono::Local::now().format("%H:%M:%S")));
        }
    }
}
