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
