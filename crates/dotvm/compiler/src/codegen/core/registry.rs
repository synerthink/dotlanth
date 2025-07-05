// Dotlanth
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
