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

//! gRPC service implementations - modular architecture

// Modular services
pub mod abi;
pub mod dots;
pub mod metrics;
pub mod vm_management;

// Unified VM service that coordinates all sub-services
pub mod vm_service;

// Re-export main services
pub use abi::AbiService;
pub use dots::DotsService;
pub use metrics::MetricsService;
pub use vm_management::VmManagementService;
pub use vm_service::VmServiceImpl;
