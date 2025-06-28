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

// Common types and utilities for the storage engine

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Forward declaration for use in Storage trait
use crate::storage_engine::file_format::Page;

/// Represents a unique identifier for a database instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatabaseId(pub u64);

/// Represents a unique identifier for a database version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VersionId(pub u64);

impl fmt::Display for VersionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Storage configuration options
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Path to the storage files
    pub path: PathBuf,
    /// Size of each page in bytes
    pub page_size: usize,
    /// Maximum number of pages to keep in the buffer pool
    pub buffer_pool_size: usize,
    /// Whether to use direct I/O (bypassing filesystem cache)
    pub direct_io: bool,
    /// Size of the WAL in bytes
    pub wal_size: usize,
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Maximum dirty pages before forced flush
    pub max_dirty_pages: usize,
    /// Background writer thread count
    pub writer_threads: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./data"),
            page_size: 4096,
            buffer_pool_size: 10000,
            direct_io: false,
            wal_size: 64 * 1024 * 1024, // 64 MB
            flush_interval_ms: 1000,
            max_dirty_pages: 1000,
            writer_threads: 2,
        }
    }
}

/// Error types specific to the storage engine
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Page {0} not found")]
    PageNotFound(u64),

    #[error("Buffer pool is full")]
    BufferPoolFull,

    #[error("Transaction aborted: {0}")]
    TransactionAborted(String),

    #[error("Version {0} not found")]
    VersionNotFound(VersionId),

    #[error("Corrupted storage: {0}")]
    Corruption(String),

    #[error("Buffer error: {0}")]
    Buffer(String),

    #[error("WAL error: {0}")]
    Wal(String),

    #[error("Concurrency error: {0}")]
    Concurrency(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Result type for storage operations
pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Helper function to create storage I/O errors
pub fn io_error(kind: ErrorKind, msg: &str) -> StorageError {
    StorageError::Io(Error::new(kind, msg))
}

/// Generate a unique timestamp for versioning
pub fn generate_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_nanos() as u64
}

/// Calculate CRC32 checksum for data integrity
pub fn calculate_checksum(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Represents a storage device type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageDevice {
    /// Regular file system
    File,
    /// Direct block device
    BlockDevice,
}

/// Safely open a file or block device for storage
pub fn open_storage<P: AsRef<Path>>(path: P, device_type: StorageDevice, create: bool, direct_io: bool) -> StorageResult<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);

    if create {
        options.create(true);
    }

    #[cfg(target_os = "linux")]
    if direct_io {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_DIRECT);
    }

    match device_type {
        StorageDevice::File => Ok(options.open(path)?),
        StorageDevice::BlockDevice => {
            #[cfg(unix)]
            {
                Ok(options.open(path)?)
            }
            #[cfg(not(unix))]
            {
                Err(StorageError::Io(io::Error::new(io::ErrorKind::Unsupported, "Block device access is only supported on Unix systems")))
            }
        }
    }
}

/// Asynchronous I/O operation handle
pub struct IoHandle {
    // Implementation would depend on the async I/O library used
    // This is a simple placeholder
    pub(crate) inner: Arc<()>,
}

/// Trait for storage components that need initialization
pub trait Initializable {
    /// Initialize the component
    fn init(&mut self) -> StorageResult<()>;

    /// Check if the component is initialized
    fn is_initialized(&self) -> bool;
}

/// Trait for components that need periodic flushing to disk
pub trait Flushable {
    /// Flush any in-memory data to disk
    fn flush(&mut self) -> StorageResult<()>;
}

/// A trait for asynchronous I/O operations
pub trait AsyncIO: Send + Sync {
    fn read_page(&self, page_id: u64, buffer: &mut [u8]) -> StorageResult<usize>;
    fn write_page(&self, page_id: u64, buffer: &[u8]) -> StorageResult<usize>;
    fn sync(&self) -> StorageResult<()>;
}

/// A trait defining storage operations
pub trait Storage: Send + Sync {
    fn read_page(&self, page_id: u64) -> StorageResult<Page>;
    fn write_page(&self, page: &Page) -> StorageResult<()>;
    fn allocate_page(&mut self) -> StorageResult<u64>;
    fn free_page(&mut self, page_id: u64) -> StorageResult<()>;
    fn flush(&self) -> StorageResult<()>;
    fn close(&mut self) -> StorageResult<()>;
}
