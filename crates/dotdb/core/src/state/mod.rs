//! State Management System
//!
//! This module provides a comprehensive state management system for the blockchain,
//! implementing various components for efficient state storage, synchronization,
//! and maintenance.
//!
//! # Core Components
//!
//! ## Merkle Patricia Trie (MPT)
//! - Efficient key-value storage with cryptographic proofs
//! - Secure state root calculation
//! - Optimized node encoding and storage
//!
//! ## State Difference System
//! - Efficient state comparison and synchronization
//! - Change tracking and application
//! - Forward and reverse diff operations
//!
//! ## State Snapshot System
//! - Point-in-time state capture
//! - Efficient snapshot management
//! - Metadata and versioning support
//!
//! ## State Pruning System
//! - Configurable pruning strategies
//! - Storage optimization
//! - Automatic cleanup of old state data
//!
//! # Usage
//!
//! ```rust
//! use dotdb_core::state::{
//!     MerklePatriciaTrie,
//!     StateDiff,
//!     SnapshotManager,
//!     StatePruner,
//!     PruningPolicy,
//!     create_persistent_mpt,
//!     create_in_memory_mpt,
//!     DbConfig,
//! };
//! use dotdb_core::state::snapshot::SnapshotConfig;
//!
//! // Create a new persistent trie with database backend
//! let mut trie = create_persistent_mpt("./data", Some(DbConfig::default())).unwrap();
//!
//! // Or create an in-memory trie for testing
//! let mut trie = create_in_memory_mpt().unwrap();
//!
//! // Track state changes
//! let mut diff = StateDiff::new([0; 32], [1; 32]);
//!
//! // Manage snapshots  
//! let snapshot_manager: SnapshotManager<dotdb_core::state::MptStorageAdapter> = SnapshotManager::new(SnapshotConfig::default());
//!
//! // Configure pruning
//! let pruner: StatePruner<dotdb_core::state::MptStorageAdapter> = StatePruner::new(PruningPolicy::default());
//! ```
//!
//! # Performance Considerations
//!
//! - Efficient memory usage through node sharing
//! - Optimized state synchronization
//! - Configurable pruning strategies
//! - Thread-safe operations
//!
//! # Error Handling
//!
//! All operations return `Result` types with specific error variants
//! for different failure scenarios. See individual component
//! documentation for detailed error handling information.

pub mod db_interface;
pub mod diff;
pub mod mpt;
pub mod pruning;
pub mod snapshot;

// Re-export commonly used types
pub use db_interface::{Database, DbConfig, DbError, MptStorageAdapter, create_in_memory_mpt, create_persistent_mpt};
pub use diff::StateDiff;
pub use mpt::{MPTError, MerklePatriciaTrie, StateProof};
pub use pruning::{PruningPolicy, StatePruner};
pub use snapshot::{SnapshotManager, StateSnapshot};
