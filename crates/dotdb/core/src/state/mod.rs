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
//!     ContractVersionManager,
//!     StateVersionId,
//!     ContractAddress,
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
//! // Manage snapshots with versioning integration
//! let mut snapshot_manager: SnapshotManager<dotdb_core::state::MptStorageAdapter> = SnapshotManager::new(SnapshotConfig::default());
//!
//! // Create global snapshot
//! let global_snapshot = snapshot_manager.create_snapshot("global_1".to_string(), &trie, Some(100), Some("Genesis state".to_string())).unwrap();
//!
//! // Create contract-specific snapshot with versioning
//! let contract_address = ContractAddress::from([1u8; 20]);
//! let contract_snapshot = snapshot_manager.create_contract_snapshot(
//!     "contract_1".to_string(),
//!     contract_address,
//!     &trie,
//!     Some(101),
//!     Some("Contract deployed".to_string())
//! ).unwrap();
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

pub mod contract_storage_layout;
pub mod db_interface;
pub mod diff;
pub mod mpt;
pub mod pruning;
pub mod snapshot;
pub mod versioning;

// Re-export commonly used types
pub use contract_storage_layout::{ContractAddress, ContractStorageLayout, StorageLayoutError, StorageValue, StorageVariable, StorageVariableType};
pub use db_interface::{Database, DbConfig, DbError, MptStorageAdapter, create_in_memory_mpt, create_persistent_mpt};
pub use diff::StateDiff;
pub use mpt::{MPTError, MerklePatriciaTrie, StateProof};
pub use pruning::{PruningPolicy, StatePruner};
pub use snapshot::{SnapshotManager, StateSnapshot};
pub use versioning::{
    ContractStateVersion, ContractUpgradeInfo, ContractVersionManager, ContractVersioningError, ContractVersioningStatistics, LayoutChange, LayoutChangeType, StateVersionId, UpgradeType,
};
