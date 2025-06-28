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

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use thiserror::Error;

/// Errors that can occur during cardinality estimation
#[derive(Debug, Error)]
pub enum CardinalityError {
    #[error("Invalid precision parameter: {0}")]
    InvalidPrecision(u8),
    #[error("Estimation overflow")]
    EstimationOverflow,
    #[error("Empty dataset")]
    EmptyDataset,
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Method used for cardinality estimation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardinalityMethod {
    /// Exact counting using HashSet (for small datasets)
    Exact,
    /// HyperLogLog probabilistic counting (for large datasets)
    HyperLogLog { precision: u8 },
    /// Adaptive method that switches based on dataset size
    Adaptive { threshold: usize, precision: u8 },
}

impl Default for CardinalityMethod {
    fn default() -> Self {
        Self::Adaptive { threshold: 10000, precision: 14 }
    }
}

/// HyperLogLog-based cardinality estimator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperLogLogEstimator {
    precision: u8,
    buckets: Vec<u8>,
    bucket_count: usize,
    alpha: f64,
}

impl HyperLogLogEstimator {
    pub fn new(precision: u8) -> Result<Self, CardinalityError> {
        if !(4..=16).contains(&precision) {
            return Err(CardinalityError::InvalidPrecision(precision));
        }

        let bucket_count = 1 << precision;
        let alpha = Self::calculate_alpha(bucket_count);

        Ok(Self {
            precision,
            buckets: vec![0; bucket_count],
            bucket_count,
            alpha,
        })
    }

    fn calculate_alpha(bucket_count: usize) -> f64 {
        match bucket_count {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / bucket_count as f64),
        }
    }

    pub fn add<T: Hash>(&mut self, value: &T) {
        let hash = self.hash_value(value);
        let bucket_index = (hash >> (64 - self.precision)) as usize;
        let leading_zeros = ((hash << self.precision) | (1 << self.precision)).leading_zeros() + 1;

        self.buckets[bucket_index] = self.buckets[bucket_index].max(leading_zeros as u8);
    }

    pub fn estimate(&self) -> u64 {
        let raw_estimate = self.alpha * (self.bucket_count as f64).powi(2) / self.buckets.iter().map(|&b| 2.0_f64.powi(-(b as i32))).sum::<f64>();

        // Apply bias correction and small/large range corrections
        let corrected_estimate = if raw_estimate <= 2.5 * self.bucket_count as f64 {
            // Small range correction
            let zero_count = self.buckets.iter().filter(|&&b| b == 0).count();
            if zero_count > 0 {
                (self.bucket_count as f64) * (self.bucket_count as f64 / zero_count as f64).ln()
            } else {
                raw_estimate
            }
        } else if raw_estimate <= (1.0 / 30.0) * (1u64 << 32) as f64 {
            // No correction needed
            raw_estimate
        } else {
            // Large range correction
            -((1u64 << 32) as f64) * (1.0 - raw_estimate / ((1u64 << 32) as f64)).ln()
        };

        corrected_estimate.round() as u64
    }

    pub fn merge(&mut self, other: &HyperLogLogEstimator) -> Result<(), CardinalityError> {
        if self.precision != other.precision {
            return Err(CardinalityError::InvalidPrecision(other.precision));
        }

        for (i, &other_value) in other.buckets.iter().enumerate() {
            self.buckets[i] = self.buckets[i].max(other_value);
        }

        Ok(())
    }

    fn hash_value<T: Hash>(&self, value: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    pub fn reset(&mut self) {
        self.buckets.fill(0);
    }

    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.buckets.len()
    }
}

/// Main cardinality estimator that can use different methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardinalityEstimator {
    method: CardinalityMethod,
    exact_set: Option<HashSet<u64>>,
    hll_estimator: Option<HyperLogLogEstimator>,
    count: u64,
    created_at: u64,
    last_updated: u64,
}

impl CardinalityEstimator {
    pub fn new(method: CardinalityMethod) -> Result<Self, CardinalityError> {
        let (exact_set, hll_estimator) = match &method {
            CardinalityMethod::Exact => (Some(HashSet::new()), None),
            CardinalityMethod::HyperLogLog { precision } => (None, Some(HyperLogLogEstimator::new(*precision)?)),
            CardinalityMethod::Adaptive { precision, .. } => (Some(HashSet::new()), Some(HyperLogLogEstimator::new(*precision)?)),
        };

        Ok(Self {
            method,
            exact_set,
            hll_estimator,
            count: 0,
            created_at: crate::storage_engine::generate_timestamp(),
            last_updated: crate::storage_engine::generate_timestamp(),
        })
    }

    pub fn add<T: Hash>(&mut self, value: &T) {
        self.count += 1;
        self.last_updated = crate::storage_engine::generate_timestamp();

        let hash = self.hash_value(value);

        match &self.method {
            CardinalityMethod::Exact => {
                if let Some(ref mut set) = self.exact_set {
                    set.insert(hash);
                }
            }
            CardinalityMethod::HyperLogLog { .. } => {
                if let Some(ref mut hll) = self.hll_estimator {
                    hll.add(value);
                }
            }
            CardinalityMethod::Adaptive { threshold, .. } => {
                if let Some(ref mut set) = self.exact_set {
                    if set.len() < *threshold {
                        set.insert(hash);
                    } else {
                        // Switch to HyperLogLog
                        if let Some(ref mut hll) = self.hll_estimator {
                            // Add all existing values to HLL
                            for &existing_hash in set.iter() {
                                hll.add(&existing_hash);
                            }
                            hll.add(value);
                        }
                        self.exact_set = None;
                    }
                } else if let Some(ref mut hll) = self.hll_estimator {
                    hll.add(value);
                }
            }
        }
    }

    pub fn estimate(&self) -> u64 {
        match &self.method {
            CardinalityMethod::Exact => self.exact_set.as_ref().map_or(0, |set| set.len() as u64),
            CardinalityMethod::HyperLogLog { .. } => self.hll_estimator.as_ref().map_or(0, |hll| hll.estimate()),
            CardinalityMethod::Adaptive { .. } => {
                if let Some(ref set) = self.exact_set {
                    set.len() as u64
                } else if let Some(ref hll) = self.hll_estimator {
                    hll.estimate()
                } else {
                    0
                }
            }
        }
    }

    pub fn merge(&mut self, other: &CardinalityEstimator) -> Result<(), CardinalityError> {
        if std::mem::discriminant(&self.method) != std::mem::discriminant(&other.method) {
            return Err(CardinalityError::InvalidPrecision(0));
        }

        self.count += other.count;
        self.last_updated = crate::storage_engine::generate_timestamp();

        if let (Some(self_set), Some(other_set)) = (&mut self.exact_set, &other.exact_set) {
            self_set.extend(other_set.iter());
        }

        if let (Some(self_hll), Some(other_hll)) = (&mut self.hll_estimator, &other.hll_estimator) {
            self_hll.merge(other_hll)?;
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.last_updated = crate::storage_engine::generate_timestamp();

        if let Some(ref mut set) = self.exact_set {
            set.clear();
        }

        if let Some(ref mut hll) = self.hll_estimator {
            hll.reset();
        }
    }

    pub fn memory_usage(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();

        if let Some(ref set) = self.exact_set {
            size += set.capacity() * std::mem::size_of::<u64>();
        }

        if let Some(ref hll) = self.hll_estimator {
            size += hll.memory_usage();
        }

        size
    }

    pub fn accuracy_estimate(&self) -> f64 {
        match &self.method {
            CardinalityMethod::Exact => 1.0,
            CardinalityMethod::HyperLogLog { precision } => 1.04 / (2.0_f64.powf(*precision as f64 / 2.0)).sqrt(),
            CardinalityMethod::Adaptive { precision, .. } => {
                if self.exact_set.is_some() {
                    1.0
                } else {
                    1.04 / (2.0_f64.powf(*precision as f64 / 2.0)).sqrt()
                }
            }
        }
    }

    pub fn get_method(&self) -> &CardinalityMethod {
        &self.method
    }

    pub fn total_count(&self) -> u64 {
        self.count
    }

    fn hash_value<T: Hash>(&self, value: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

/// Utility structure to track cardinalities for multiple columns/attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiColumnCardinalityTracker {
    estimators: HashMap<String, CardinalityEstimator>,
    default_method: CardinalityMethod,
}

impl MultiColumnCardinalityTracker {
    pub fn new(default_method: CardinalityMethod) -> Self {
        Self {
            estimators: HashMap::new(),
            default_method,
        }
    }

    pub fn add_column(&mut self, column_name: &str) -> Result<(), CardinalityError> {
        let estimator = CardinalityEstimator::new(self.default_method.clone())?;
        self.estimators.insert(column_name.to_string(), estimator);
        Ok(())
    }

    pub fn add_value<T: Hash>(&mut self, column_name: &str, value: &T) -> Result<(), CardinalityError> {
        if !self.estimators.contains_key(column_name) {
            self.add_column(column_name)?;
        }

        if let Some(estimator) = self.estimators.get_mut(column_name) {
            estimator.add(value);
        }

        Ok(())
    }

    pub fn get_cardinality(&self, column_name: &str) -> Option<u64> {
        self.estimators.get(column_name).map(|e| e.estimate())
    }

    pub fn get_all_cardinalities(&self) -> HashMap<String, u64> {
        self.estimators.iter().map(|(name, estimator)| (name.clone(), estimator.estimate())).collect()
    }

    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.estimators.iter().map(|(k, v)| k.len() + v.memory_usage()).sum::<usize>()
    }

    pub fn reset_column(&mut self, column_name: &str) {
        if let Some(estimator) = self.estimators.get_mut(column_name) {
            estimator.reset();
        }
    }

    pub fn reset_all(&mut self) {
        for estimator in self.estimators.values_mut() {
            estimator.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperloglog_creation() {
        let hll = HyperLogLogEstimator::new(14).unwrap();
        assert_eq!(hll.precision, 14);
        assert_eq!(hll.bucket_count, 16384);
    }

    #[test]
    fn test_hyperloglog_invalid_precision() {
        let result = HyperLogLogEstimator::new(3);
        assert!(matches!(result, Err(CardinalityError::InvalidPrecision(3))));

        let result = HyperLogLogEstimator::new(17);
        assert!(matches!(result, Err(CardinalityError::InvalidPrecision(17))));
    }

    #[test]
    fn test_exact_cardinality_estimator() {
        let mut estimator = CardinalityEstimator::new(CardinalityMethod::Exact).unwrap();

        estimator.add(&"value1");
        estimator.add(&"value2");
        estimator.add(&"value1"); // duplicate
        estimator.add(&"value3");

        assert_eq!(estimator.estimate(), 3);
        assert_eq!(estimator.total_count(), 4);
    }

    #[test]
    fn test_hyperloglog_cardinality_estimator() {
        let mut estimator = CardinalityEstimator::new(CardinalityMethod::HyperLogLog { precision: 14 }).unwrap();

        // Add many unique values
        for i in 0..10000 {
            estimator.add(&format!("value{}", i));
        }

        let estimate = estimator.estimate();
        let error_rate = (estimate as f64 - 10000.0).abs() / 10000.0;

        // HyperLogLog should be reasonably accurate (within ~2% for this precision)
        assert!(error_rate < 0.05);
    }

    #[test]
    fn test_adaptive_cardinality_estimator() {
        let mut estimator = CardinalityEstimator::new(CardinalityMethod::Adaptive { threshold: 100, precision: 14 }).unwrap();

        // Add values below threshold (should use exact counting)
        for i in 0..50 {
            estimator.add(&format!("value{}", i));
        }

        assert_eq!(estimator.estimate(), 50);
        assert!(estimator.exact_set.is_some());

        // Add more values to exceed threshold
        for i in 50..200 {
            estimator.add(&format!("value{}", i));
        }

        // Should have switched to HyperLogLog
        assert!(estimator.exact_set.is_none());
        let estimate = estimator.estimate();
        assert!(estimate >= 180 && estimate <= 220); // Approximate range
    }

    #[test]
    fn test_hyperloglog_merge() {
        let mut hll1 = HyperLogLogEstimator::new(12).unwrap();
        let mut hll2 = HyperLogLogEstimator::new(12).unwrap();

        // Add different values to each estimator
        for i in 0..1000 {
            hll1.add(&format!("value{}", i));
        }

        for i in 500..1500 {
            hll2.add(&format!("value{}", i));
        }

        let estimate1 = hll1.estimate();
        let estimate2 = hll2.estimate();

        hll1.merge(&hll2).unwrap();
        let merged_estimate = hll1.estimate();

        // Merged estimate should be approximately the union cardinality (1500)
        assert!(merged_estimate > estimate1);
        assert!(merged_estimate > estimate2);
        assert!(merged_estimate >= 1400 && merged_estimate <= 1600);
    }

    #[test]
    fn test_multi_column_tracker() {
        let mut tracker = MultiColumnCardinalityTracker::new(CardinalityMethod::Exact);

        tracker.add_value("column1", &"value1").unwrap();
        tracker.add_value("column1", &"value2").unwrap();
        tracker.add_value("column1", &"value1").unwrap(); // duplicate

        tracker.add_value("column2", &"other1").unwrap();
        tracker.add_value("column2", &"other2").unwrap();
        tracker.add_value("column2", &"other3").unwrap();

        assert_eq!(tracker.get_cardinality("column1"), Some(2));
        assert_eq!(tracker.get_cardinality("column2"), Some(3));
        assert_eq!(tracker.get_cardinality("nonexistent"), None);

        let all_cardinalities = tracker.get_all_cardinalities();
        assert_eq!(all_cardinalities.len(), 2);
        assert_eq!(all_cardinalities["column1"], 2);
        assert_eq!(all_cardinalities["column2"], 3);
    }

    #[test]
    fn test_estimator_reset() {
        let mut estimator = CardinalityEstimator::new(CardinalityMethod::Exact).unwrap();

        estimator.add(&"value1");
        estimator.add(&"value2");
        assert_eq!(estimator.estimate(), 2);

        estimator.reset();
        assert_eq!(estimator.estimate(), 0);
        assert_eq!(estimator.total_count(), 0);
    }

    #[test]
    fn test_memory_usage_calculation() {
        let estimator = CardinalityEstimator::new(CardinalityMethod::Exact).unwrap();
        let usage = estimator.memory_usage();
        assert!(usage > 0);

        let hll_estimator = CardinalityEstimator::new(CardinalityMethod::HyperLogLog { precision: 14 }).unwrap();
        let hll_usage = hll_estimator.memory_usage();
        assert!(hll_usage > 0);
    }
}
