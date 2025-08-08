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

//! # Security Module
//!
//! Comprehensive capability-based security system for custom opcodes.
//! Provides sandboxing, resource limits, audit logging, permission checking,
//! isolation management, and security policy enforcement.

pub mod audit_logger;
pub mod capability_manager;
pub mod errors;
pub mod isolation_manager;
pub mod permission_checker;
pub mod policy_enforcer;
pub mod resource_limiter;
pub mod sandbox;
pub mod types;

// Re-export main types
pub use audit_logger::{AuditEvent, AuditLogger};
pub use capability_manager::{Capability, CapabilityManager};
pub use errors::{AuditError, IsolationError, PermissionError, PolicyError, ResourceError, SecurityError};
pub use isolation_manager::{IsolationContext, IsolationManager};
pub use permission_checker::{Permission, PermissionChecker};
pub use policy_enforcer::{PolicyEnforcer, SecurityPolicy};
pub use resource_limiter::{ResourceLimiter, ResourceLimits, ResourceUsage};
pub use sandbox::SecuritySandbox;
pub use types::{CustomOpcode, DotVMContext, OpcodeType, SecurityLevel};
