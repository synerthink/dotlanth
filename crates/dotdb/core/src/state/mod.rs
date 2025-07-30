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

pub mod db_interface;
pub mod diff;
pub mod dot_storage_layout;
pub mod mpt;
pub mod pruning;
pub mod snapshot;
pub mod versioning;

// Re-export commonly used types
pub use db_interface::{Database, DbConfig, DbError, MptStorageAdapter, create_in_memory_mpt, create_persistent_mpt};
pub use diff::StateDiff;
pub use dot_storage_layout::{DotAddress, DotStorageLayout, StorageLayoutError, StorageValue, StorageVariable, StorageVariableType};
pub use mpt::{MPTError, MerklePatriciaTrie, StateProof};
pub use pruning::{PruningPolicy, StatePruner};
pub use snapshot::{SnapshotManager, StateSnapshot};
pub use versioning::{DotStateVersion, DotUpgradeInfo, DotVersionManager, DotVersioningError, DotVersioningStatistics, LayoutChange, LayoutChangeType, StateVersionId, UpgradeType};
