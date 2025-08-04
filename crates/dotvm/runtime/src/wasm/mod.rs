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

//! DotVM WASM Runtime

pub mod async_bridge;
pub mod error;
pub mod execution;
pub mod host_functions;
pub mod instance;
pub mod interpreter;
pub mod management;
pub mod memory;
pub mod module;
pub mod runtime;
pub mod transpiler;

// Re-export main types (specific imports to avoid conflicts)
pub use async_bridge::AsyncWasmBridge;
pub use error::{WasmError, WasmResult};
pub use execution::{
    CallFrame, CallTraceEntry, ExecutionContext, ExecutionMetrics, ExecutionState, ExecutionStatistics, FrameMetadata, FunctionSignature, HostFunction, ImportedModule, StackStatistics, ValueType,
    WasmExecutionExtensions, WasmStack,
};
pub use host_functions::{
    AsyncExecutionState, AsyncHostFunction, AsyncHostFunctionInterface, ExecutionError, HostError, ResourceUsage, SecurityContext as HostSecurityContext, SecurityLevel as HostSecurityLevel,
    ValidationError,
};
pub use instance::{InstanceMetadata, InstanceState, WasmInstance};
pub use management::{MonitorConfig, PerformanceMetrics, ResourceLimiter, ResourceUsage as MgmtResourceUsage, RuntimeManager, SecurityContext, SecurityLevel, SecurityPolicy, WasmMonitor};
pub use memory::{MemoryStats, WasmMemory};
pub use module::{PerformanceHints, SecurityMetadata, ValidationStatus, WasmModule};
pub use runtime::{
    DotVMWasmRuntime, ExecutionConfig, MemoryConfig, RuntimeStatistics, RuntimeStats, SecurityConfig, SecurityIssue, SecuritySeverity, StoreConfig, StoreStatistics, ValidationConfig,
    ValidationError as RuntimeValidationError, ValidationErrorType, ValidationLocation, ValidationResult, ValidationStats, ValidationWarning, WasmRuntimeConfig, WasmValidator,
};
pub use transpiler::{TranspilerConfig, TranspilerStatistics, WasmTranspiler};

// Re-export key types for convenience
pub use execution::ExecutionContext as WasmExecutionContext;
pub use instance::ExportedFunction as InstanceExportedFunction;
