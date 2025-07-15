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

//! Optimization caching components

use crate::transpiler::types::TranspiledFunction;

/// Trait for caching optimization results (stub)
pub trait OptimizationCache {
    /// Attempt to retrieve cached results
    fn get(&self, key: &str) -> Option<Vec<TranspiledFunction>>;
    /// Insert results into the cache
    fn insert(&mut self, key: String, data: Vec<TranspiledFunction>);
}

/// No-op cache that does not store anything
pub struct NoopCache;

impl OptimizationCache for NoopCache {
    fn get(&self, _key: &str) -> Option<Vec<TranspiledFunction>> {
        None
    }
    fn insert(&mut self, _key: String, _data: Vec<TranspiledFunction>) {}
}
