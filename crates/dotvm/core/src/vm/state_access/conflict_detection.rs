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

use super::read_write_analysis::StateAccessTracker;
use std::collections::HashSet;

/// Detects conflicting access patterns between operations
pub struct ConflictDetector;

impl ConflictDetector {
    /// Identifies conflicts between multiple access trackers
    /// # Arguments
    /// - `trackers`: Slice of access trackers to analyze
    ///
    /// # Returns
    /// Vector of conflict descriptions with conflict types and counts:
    /// - WW: Write-Write conflicts
    /// - WR: Write-Read conflicts
    /// - RW: Read-Write conflicts
    pub fn detect_conflicts(trackers: &[&StateAccessTracker]) -> Vec<String> {
        let mut conflicts = Vec::new();

        for (i, t1) in trackers.iter().enumerate() {
            let t1_writes: HashSet<_> = t1.writes.keys().collect();
            let t1_reads: HashSet<_> = t1.reads.keys().collect();

            for t2 in trackers.iter().skip(i + 1) {
                let t2_writes: HashSet<_> = t2.writes.keys().collect();
                let t2_reads: HashSet<_> = t2.reads.keys().collect();

                let write_write = t1_writes.intersection(&t2_writes).count();
                let write_read = t1_writes.intersection(&t2_reads).count();
                let read_write = t1_reads.intersection(&t2_writes).count();

                if write_write > 0 || write_read > 0 || read_write > 0 {
                    conflicts.push(format!(
                        "Conflict between ops {} and {}: WW={}, WR={}, RW={}",
                        i + 1,
                        trackers.iter().position(|t| *t == *t2).unwrap() + 1,
                        write_write,
                        write_read,
                        read_write
                    ));
                }
            }
        }

        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates conflict detection between operations:
    /// 1. Create writer tracking 'balance' writes
    /// 2. Create reader tracking 'balance' reads
    /// 3. Verify system detects write-read conflict
    #[test]
    fn test_conflict_detection() {
        let mut t1 = StateAccessTracker::new();
        t1.track_write("balance");

        let mut t2 = StateAccessTracker::new();
        t2.track_read("balance");

        let conflicts = ConflictDetector::detect_conflicts(&[&t1, &t2]);
        assert!(!conflicts.is_empty(), "Failed to detect write-read conflict on 'balance'");
    }
}
