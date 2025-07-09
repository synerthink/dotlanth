//! Optimization profiling and performance analysis

pub mod hotspots;
pub mod profiler;
pub mod suggestions;

pub use hotspots::{Hotspot, HotspotDetector, HotspotType};
pub use profiler::{OptimizationProfiler, ProfilerConfig};
pub use suggestions::{OptimizationSuggester, OptimizationSuggestion};
