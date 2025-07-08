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

//! Patch point tracking utilities

use super::traits::PatchPoint;

/// Create and manage patch points in a writer buffer
pub struct PatchTracker {
    // Additional state may be added here
}

impl PatchTracker {
    /// Register a new patch point
    pub fn new_point(&self, offset: usize) -> PatchPoint {
        PatchPoint { offset }
    }
}
