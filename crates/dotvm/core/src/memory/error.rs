use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Failed to allocate memory: {0}")]
    AllocationFailed(String),

    #[error("Memory allocation too large: requested {requested}, maximum {maximum}")]
    AllocationTooLarge {
        requested: usize,
        maximum: usize,
    },

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
    OutOfMemory {
        requested: usize,
        available: usize,
    },

    #[error("Memory mapping error: {0}")]
    MappingError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid memory handle")]
    InvalidHandle,

    #[error("Memory already deallocated")]
    AlreadyDeallocated,

    #[error("Memory fragmentation error: {0}")]
    FragmentationError(String),

    #[error("TLB error: {0}")]
    TLBError(String),
}

pub type MemoryResult<T> = Result<T, MemoryError>;