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

//! Role-Based Access Control (RBAC) system
//!
//! This module provides a comprehensive RBAC implementation with:
//! - Hierarchical roles with inheritance
//! - Permission-based access control
//! - Dot-level permissions
//! - Resource-based permissions
//! - Audit logging
//! - Performance-optimized permission checking

pub mod audit;
pub mod cache;
pub mod dot_permissions;
pub mod manager;
pub mod middleware;
pub mod permissions;
pub mod roles;
pub mod system;

pub use audit::*;
pub use cache::*;
pub use dot_permissions::*;
pub use manager::*;
pub use middleware::*;
pub use permissions::*;
pub use roles::*;
pub use system::*;
