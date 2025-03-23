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

use std::collections::HashMap;

/// Tracks read/write operations on state variables with access counts
#[derive(Debug, Default, Clone, PartialEq)]
pub struct StateAccessTracker {
    /// Map of variable names to number of read accesses
    pub reads: HashMap<String, usize>,
    /// Map of variable names to number of write accesses
    pub writes: HashMap<String, usize>,
}

impl StateAccessTracker {
    /// Creates a new empty tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a read access to a state variable
    /// # Arguments
    /// - `variable`: Name of the accessed variable
    pub fn track_read(&mut self, variable: impl Into<String>) {
        let var = variable.into();
        *self.reads.entry(var).or_insert(0) += 1;
    }

    /// Records a write access to a state variable
    /// # Arguments
    /// - `variable`: Name of the modified variable
    pub fn track_write(&mut self, variable: impl Into<String>) {
        let var = variable.into();
        *self.writes.entry(var).or_insert(0) += 1;
    }

    /// Combines two trackers' access records
    /// # Arguments
    /// - `other`: Tracker to merge with current instance
    pub fn merge(&mut self, other: &Self) {
        for (var, count) in &other.reads {
            *self.reads.entry(var.clone()).or_insert(0) += count;
        }
        for (var, count) in &other.writes {
            *self.writes.entry(var.clone()).or_insert(0) += count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies basic read/write tracking functionality:
    /// 1. Track single read operation
    /// 2. Track single write operation
    /// 3. Validate variable presence in access maps
    #[test]
    fn test_basic_tracking() {
        let mut tracker = StateAccessTracker::new();
        tracker.track_read("balance");
        tracker.track_write("balance");

        assert!(tracker.reads.contains_key("balance"), "Read tracking failed for 'balance'");
        assert!(tracker.writes.contains_key("balance"), "Write tracking failed for 'balance'");
    }

    /// Tests tracker merging functionality:
    /// 1. Create two separate trackers
    /// 2. Merge their access records
    /// 3. Verify combined access map contains both sources
    #[test]
    fn test_merge_operations() {
        let mut tracker1 = StateAccessTracker::new();
        tracker1.track_read("a");

        let mut tracker2 = StateAccessTracker::new();
        tracker2.track_write("b");

        tracker1.merge(&tracker2);

        assert!(tracker1.reads.contains_key("a"), "Merge failed for read operations");
        assert!(tracker1.writes.contains_key("b"), "Merge failed for write operations");
    }
}
