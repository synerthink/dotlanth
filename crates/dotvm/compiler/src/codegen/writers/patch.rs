// Dotlanth
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
