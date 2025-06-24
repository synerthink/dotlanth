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

// Storage Engine Module
// Provides a persistent storage system with ACID guarantees

pub mod buffer_manager;
pub mod deadlock_detector;
pub mod file_format;
pub mod isolation;
pub mod lib;
pub mod mvcc;
pub mod occ;
pub mod page_manager;
pub mod transaction;
pub mod wal;

// Public exports
pub use buffer_manager::{Buffer, BufferManager, BufferPool, BufferStats};
pub use deadlock_detector::{DeadlockCycle, DeadlockDetector, DeadlockResolutionPolicy, DeadlockStatistics, WaitForEdge};
pub use file_format::{FileFormat, Page, PageId, PageType};
pub use isolation::{IsolationLevelEnforcer, IsolationStatistics, LockManager, LockStatistics, LockType};
pub use lib::{AsyncIO, DatabaseId, Flushable, Initializable, Storage, StorageConfig, StorageDevice, StorageError, StorageResult, VersionId, calculate_checksum, generate_timestamp};
pub use mvcc::{MVCCManager, MVCCStatistics, TransactionSnapshot, VersionInfo};
pub use occ::{ConflictResolution, ConflictResolutionStrategy, ConflictType, OCCManager, OCCStatistics, OCCTransaction, OCCTransactionManager, ValidationContext};
pub use page_manager::{PageAllocation, PageManager};
pub use transaction::{IsolationLevel, Transaction, TransactionManager, TransactionState};
pub use wal::{LogEntry, LogSequenceNumber, WriteAheadLog};
