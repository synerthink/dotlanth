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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Failed to allocate memory: {0}")]
    AllocationFailed(String),

    #[error("Memory allocation too large: requested {requested}, maximum {maximum}")]
    AllocationTooLarge { requested: usize, maximum: usize },

    #[error("Invalid memory alignment: {0}")]
    InvalidAlignment(usize),

    #[error("Memory protection error: {0}")]
    ProtectionError(String),

    #[error("Page table error: {0}")]
    PageTableError(String),

    #[error("Invalid memory address: {0:#x}")]
    InvalidAddress(usize),

    #[error("Memory pool error: {0}")]
    PoolError(String),

    #[error("Out of memory: requested {requested} bytes, available {available}")]
    OutOfMemory { requested: usize, available: usize },

    #[error("Memory mapping error: {0}")]
    MappingError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid memory handle")]
    InvalidHandle,

    #[error("Memory already deallocated")]
    AlreadyDeallocated,

    #[error("Unsupported protection mechanism")]
    UnsupportedProtection,

    #[error("Unsupported operating system")]
    UnsupportedOS,

    #[error("Unsupported architecture")]
    UnsupportedArch,

    #[error("Memory fragmentation error: {0}")]
    FragmentationError(String),

    #[error("TLB error: {0}")]
    TLBError(String),

    #[error("Invalid size: available {available} bytes, size cannot be zero")]
    InvalidSize { available: usize },

    #[error("Out of virtual address space")]
    OutOfVirtualMemory,

    #[error("Invalid memory region: {0}")]
    InvalidRegion(String),

    #[error("Allocation error: {0}")]
    AllocationError(String),
}

pub type MemoryResult<T> = Result<T, MemoryError>;
