// Dotlanth
//! Generation context holds shared state during codegen

/// Shared context passed to section generators and writer pipelines
pub struct GenerationContext {
    // TODO: Define symbol tables, offsets, flags, etc.
}

impl GenerationContext {
    /// Create a new generation context
    pub fn new() -> Self {
        GenerationContext {}
    }
}
