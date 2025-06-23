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

//! Statistics Collection System
//!
//! This module provides comprehensive statistics collection and analysis
//! for query optimization in DotDB. It collects various metadata about
//! data distribution, table characteristics, and access patterns to
//! enable cost-based query planning.
//!
//! # Core Components
//!
//! ## Histogram Management
//! - Collect and maintain histograms for data distribution analysis
//! - Support for various data types and ranges
//! - Configurable bucket strategies
//!
//! ## Cardinality Estimation
//! - Track unique value counts for columns and indexes
//! - HyperLogLog-based approximate counting for large datasets
//! - Exact counting for smaller datasets
//!
//! ## Common Value Tracking
//! - Identify and track most frequently accessed values
//! - Support for top-k frequent items
//! - Configurable frequency thresholds
//!
//! ## Access Pattern Analysis
//! - Monitor query patterns and access frequencies
//! - Track hot and cold data regions
//! - Temporal access pattern analysis
//!
//! # Usage
//!
//! ```rust
//! use dotdb_core::statistics::{
//!     StatisticsCollector,
//!     StatisticsConfig,
//!     Histogram,
//!     CardinalityEstimator,
//!     AccessPatternTracker,
//! };
//!
//! // Create a statistics collector
//! let config = StatisticsConfig::default();
//! let collector = StatisticsCollector::new(config);
//!
//! // Create cardinality estimator
//! let mut cardinality_estimator = CardinalityEstimator::new(
//!     dotdb_core::statistics::CardinalityMethod::Exact
//! ).unwrap();
//!
//! // Add some values
//! cardinality_estimator.add(&"user1@example.com");
//! cardinality_estimator.add(&"user2@example.com");
//!
//! // Get cardinality estimate
//! let cardinality = cardinality_estimator.estimate();
//! assert_eq!(cardinality, 2);
//!
//! // Create access pattern tracker
//! let mut tracker = AccessPatternTracker::new(1000);
//! tracker.record_access("key1");
//! tracker.record_access("key2");
//!
//! let stats = tracker.get_access_stats();
//! assert_eq!(stats.total_accesses, 2);
//! ```

pub mod access_patterns;
pub mod cardinality;
pub mod collector;
pub mod histogram;

// Re-export commonly used types
pub use access_patterns::{AccessPattern, AccessPatternTracker, AccessStats, PatternType, TemporalAccessPattern};
pub use cardinality::{CardinalityEstimator, CardinalityMethod, HyperLogLogEstimator};
pub use collector::{StatisticsCollector, StatisticsConfig, StatisticsError, StatisticsResult, UpdateStrategy};
pub use histogram::{Bucket, BucketStrategy, Histogram, HistogramType, ValueRange};
