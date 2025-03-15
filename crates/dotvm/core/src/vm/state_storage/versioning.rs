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

use super::state_storage::StateStorageError;
use serde::{Deserialize, Serialize};
use std::fmt; // assuming StateStorageError is defined in mod.rs

/// The current/latest version that our system supports.
pub const LATEST_VERSION: u32 = 2;

/// A wrapper struct that encapsulates state data along with a version number.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct VersionedState<T> {
    /// Version of the state data.
    pub version: u32,
    /// The actual state data.
    pub data: T,
}

/// Trait defining the interface responsible for migrating state data from older versions
/// to the latest version.
pub trait StateVersioner<T>
where
    T: Clone,
{
    /// Migrates a given VersionedState from an older version to the latest supported version.
    /// Returns a new VersionedState that is guaranteed to have version == LATEST_VERSION.
    ///
    /// # Arguments
    ///
    /// * `old` - A VersionedState instance that may be in an older version.
    ///
    /// # Errors
    /// Returns an error (using StateStorageError) if migration cannot be performed.
    fn migrate(&self, old: VersionedState<T>) -> Result<VersionedState<T>, StateStorageError>;
}

/// DefaultStateVersioner is our production implementation of state versioning.
/// Its migration logic is left unimplemented to drive development with TDD.
pub struct DefaultStateVersioner;

impl DefaultStateVersioner {
    /// Creates a new instance of DefaultStateVersioner.
    pub fn new() -> Self {
        DefaultStateVersioner
    }
}

impl<T> StateVersioner<T> for DefaultStateVersioner
where
    T: Clone,
{
    fn migrate(&self, old: VersionedState<T>) -> Result<VersionedState<T>, StateStorageError> {
        if old.version > LATEST_VERSION {
            return Err(StateStorageError::Unknown);
        }
        if old.version == LATEST_VERSION {
            return Ok(old);
        }
        Ok(VersionedState {
            version: LATEST_VERSION,
            data: old.data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// For testing purposes, we define a simple TestState struct.
    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct TestState {
        pub field: String,
    }

    /// Test that a state with an older version (e.g., version 1) migrates to LATEST_VERSION
    /// without data loss.
    #[test]
    fn test_migrate_from_old_version() {
        let state = TestState { field: "old_data".into() };
        let old_state = VersionedState { version: 1, data: state.clone() };
        let migrator = DefaultStateVersioner::new();
        // Expected behavior: migration should upgrade version 1 to LATEST_VERSION
        let migrated = migrator.migrate(old_state).expect("Migration should succeed");
        assert_eq!(migrated.version, LATEST_VERSION, "Migrated version must match LATEST_VERSION");
        assert_eq!(migrated.data, state, "State data must remain unchanged during migration");
    }

    /// Test that a state already at LATEST_VERSION remains unchanged after migration.
    #[test]
    fn test_migrate_latest_version_unchanged() {
        let state = TestState { field: "current_data".into() };
        let current_state = VersionedState {
            version: LATEST_VERSION,
            data: state.clone(),
        };
        let migrator = DefaultStateVersioner::new();
        let migrated = migrator.migrate(current_state.clone()).expect("Migration should succeed");
        assert_eq!(migrated.version, LATEST_VERSION, "State version remains at LATEST_VERSION");
        assert_eq!(migrated.data, current_state.data, "State data remains unchanged");
    }

    /// Test that attempting to migrate a state with an unsupported (future) version returns an error.
    #[test]
    fn test_migrate_future_version_fails() {
        let state = TestState { field: "future_data".into() };
        let future_state = VersionedState {
            version: LATEST_VERSION + 1,
            data: state,
        };
        let migrator = DefaultStateVersioner::new();
        let result = migrator.migrate(future_state);
        assert!(result.is_err(), "Migrating a future version should return an error");
        assert_eq!(result.err().unwrap(), StateStorageError::Unknown);
    }

    /// Additional test: Migrate from a version of zero to ensure proper upgrade.
    #[test]
    fn test_migrate_from_version_zero() {
        let state = TestState { field: "zero_version_data".into() };
        let zero_version_state = VersionedState { version: 0, data: state.clone() };
        let migrator = DefaultStateVersioner::new();
        let migrated = migrator.migrate(zero_version_state).expect("Migration from zero version should succeed");
        assert_eq!(migrated.version, LATEST_VERSION, "Zero version should be upgraded to LATEST_VERSION");
        assert_eq!(migrated.data, state, "State data should remain unchanged when migrating from version zero");
    }

    /// Additional test: Verify that applying migration twice is idempotent.
    #[test]
    fn test_double_migration_idempotence() {
        let state = TestState { field: "idempotent_data".into() };
        let initial_state = VersionedState { version: 1, data: state.clone() };
        let migrator = DefaultStateVersioner::new();
        let first_migration = migrator.migrate(initial_state).expect("First migration should succeed");
        let second_migration = migrator.migrate(first_migration.clone()).expect("Second migration should be idempotent");
        assert_eq!(second_migration.version, LATEST_VERSION, "Version remains LATEST_VERSION after double migration");
        assert_eq!(second_migration.data, first_migration.data, "State data remains unchanged on subsequent migrations");
    }

    /// Additional test: Migrate a vector of states and ensure each state is updated correctly.
    #[test]
    fn test_migrate_multiple_states() {
        let states: Vec<VersionedState<TestState>> = vec![
            VersionedState {
                version: 1,
                data: TestState { field: "first".into() },
            },
            VersionedState {
                version: 0,
                data: TestState { field: "second".into() },
            },
            VersionedState {
                version: LATEST_VERSION,
                data: TestState { field: "third".into() },
            },
        ];
        let migrator = DefaultStateVersioner::new();
        let migrated_states: Vec<_> = states.into_iter().map(|s| migrator.migrate(s).expect("Migration should succeed for each state")).collect();
        for migrated in migrated_states {
            assert_eq!(migrated.version, LATEST_VERSION, "All migrated states should have LATEST_VERSION");
        }
    }
}
