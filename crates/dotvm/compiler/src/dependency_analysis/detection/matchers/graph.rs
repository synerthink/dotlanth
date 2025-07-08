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

//! Graph pattern matcher

use crate::dependency_analysis::core::traits::PatternMatcher;
use crate::dependency_analysis::detection::{MatchResult, Pattern};

/// Matches graph-shaped patterns
pub struct GraphMatcher;

impl PatternMatcher for GraphMatcher {
    type Pattern = Pattern;
    type Context = String;

    fn matches(&self, _pattern: &Self::Pattern, _context: &Self::Context) -> bool {
        // graph matching not implemented
        false
    }

    fn confidence(&self) -> f32 {
        0.5
    }

    fn extract_info(&self, _pattern: &Self::Pattern, _context: &Self::Context) -> () {
        // Extract information when a match is found
        ()
    }
}
