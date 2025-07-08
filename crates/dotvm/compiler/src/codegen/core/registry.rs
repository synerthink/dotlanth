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

//! Registry for available codegen engines and section generators

use std::collections::HashMap;

/// Registry of registered generators keyed by name or type
pub struct GeneratorRegistry {
    // TODO: store factory closures or instances
    engines: HashMap<String, Box<dyn std::any::Any>>,
}

impl GeneratorRegistry {
    /// Create a new, empty registry
    pub fn new() -> Self {
        GeneratorRegistry { engines: HashMap::new() }
    }

    /// Register a new generator instance
    pub fn register<T: 'static>(&mut self, name: String, generator: T) {
        self.engines.insert(name, Box::new(generator));
    }

    /// Retrieve a generator by name
    pub fn get<T: 'static>(&self, name: &str) -> Option<&T> {
        self.engines.get(name)?.downcast_ref()
    }
}
