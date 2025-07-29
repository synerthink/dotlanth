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

// State Management module for ParaDots
// This module provides advanced mechanisms for concurrent state operations
// and integrity verification.

pub mod lib;
pub mod mvcc;
pub mod snapshot;
pub mod tree;
pub mod verification;

// Public re-exports
pub use lib::{Error, Result, StateKey, StateValue};
pub use mvcc::{MVCCStore, Version, VersionedValue, WriteOperation};
pub use snapshot::{Snapshot, SnapshotManager, SnapshotMetadata};
pub use tree::{MerkleNode, MerkleProof, MerkleTree, StateHash};
pub use verification::{Validator, VerificationError, VerificationResult};
