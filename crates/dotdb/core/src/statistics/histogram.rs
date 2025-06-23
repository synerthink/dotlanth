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
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors that can occur during histogram operations
#[derive(Debug, Error)]
pub enum HistogramError {
    #[error("Invalid bucket configuration: {0}")]
    InvalidBucket(String),
    #[error("Value out of range: {0}")]
    ValueOutOfRange(String),
    #[error("Empty histogram")]
    EmptyHistogram,
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Type of histogram based on data distribution strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistogramType {
    /// Equal-width buckets
    EqualWidth,
    /// Equal-frequency buckets
    EqualFrequency,
    /// Custom bucket boundaries
    Custom,
}

/// Bucket creation strategy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BucketStrategy {
    /// Fixed number of equal-width buckets
    FixedWidth { bucket_count: usize },
    /// Fixed number of equal-frequency buckets
    FixedFrequency { bucket_count: usize },
    /// Custom boundaries
    CustomBoundaries { boundaries: Vec<f64> },
}

/// Represents a value range for histogram buckets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueRange {
    pub min: f64,
    pub max: f64,
    pub inclusive_min: bool,
    pub inclusive_max: bool,
}

impl ValueRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            inclusive_min: true,
            inclusive_max: false,
        }
    }

    pub fn contains(&self, value: f64) -> bool {
        let min_check = if self.inclusive_min { value >= self.min } else { value > self.min };

        let max_check = if self.inclusive_max { value <= self.max } else { value < self.max };

        min_check && max_check
    }

    pub fn width(&self) -> f64 {
        self.max - self.min
    }
}

/// A histogram bucket containing count and value information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bucket {
    pub range: ValueRange,
    pub count: u64,
    pub distinct_values: u64,
    pub most_common_value: Option<f64>,
    pub mcv_frequency: u64,
}

impl Bucket {
    pub fn new(range: ValueRange) -> Self {
        Self {
            range,
            count: 0,
            distinct_values: 0,
            most_common_value: None,
            mcv_frequency: 0,
        }
    }

    pub fn add_value(&mut self, value: f64, frequency: u64) {
        if !self.range.contains(value) {
            return;
        }

        self.count += frequency;

        if self.most_common_value.is_none() || frequency > self.mcv_frequency {
            self.most_common_value = Some(value);
            self.mcv_frequency = frequency;
        }
    }

    pub fn density(&self) -> f64 {
        if self.range.width() == 0.0 { 0.0 } else { self.count as f64 / self.range.width() }
    }

    pub fn selectivity(&self, total_count: u64) -> f64 {
        if total_count == 0 { 0.0 } else { self.count as f64 / total_count as f64 }
    }
}

/// Histogram for analyzing data distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub histogram_type: HistogramType,
    pub buckets: Vec<Bucket>,
    pub total_count: u64,
    pub null_count: u64,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub created_at: u64,
    pub last_updated: u64,
}

impl Histogram {
    pub fn new(histogram_type: HistogramType) -> Self {
        Self {
            histogram_type,
            buckets: Vec::new(),
            total_count: 0,
            null_count: 0,
            min_value: None,
            max_value: None,
            created_at: crate::storage_engine::generate_timestamp(),
            last_updated: crate::storage_engine::generate_timestamp(),
        }
    }

    pub fn create_with_strategy(strategy: BucketStrategy, data: &[f64]) -> Result<Self, HistogramError> {
        if data.is_empty() {
            return Err(HistogramError::EmptyHistogram);
        }

        let mut sorted_data = data.to_vec();
        sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min_val = sorted_data[0];
        let max_val = sorted_data[sorted_data.len() - 1];

        let buckets = match &strategy {
            BucketStrategy::FixedWidth { bucket_count } => Self::create_equal_width_buckets(*bucket_count, min_val, max_val, &sorted_data)?,
            BucketStrategy::FixedFrequency { bucket_count } => Self::create_equal_frequency_buckets(*bucket_count, &sorted_data)?,
            BucketStrategy::CustomBoundaries { boundaries } => Self::create_custom_buckets(boundaries.clone(), &sorted_data)?,
        };

        let histogram_type = match strategy {
            BucketStrategy::FixedWidth { .. } => HistogramType::EqualWidth,
            BucketStrategy::FixedFrequency { .. } => HistogramType::EqualFrequency,
            BucketStrategy::CustomBoundaries { .. } => HistogramType::Custom,
        };

        let mut histogram = Self::new(histogram_type);
        histogram.buckets = buckets;
        histogram.total_count = data.len() as u64;
        histogram.min_value = Some(min_val);
        histogram.max_value = Some(max_val);

        Ok(histogram)
    }

    fn create_equal_width_buckets(bucket_count: usize, min_val: f64, max_val: f64, data: &[f64]) -> Result<Vec<Bucket>, HistogramError> {
        if bucket_count == 0 {
            return Err(HistogramError::InvalidBucket("Bucket count must be greater than 0".to_string()));
        }

        let width = (max_val - min_val) / bucket_count as f64;
        let mut buckets = Vec::with_capacity(bucket_count);

        // Create buckets
        for i in 0..bucket_count {
            let bucket_min = min_val + (i as f64 * width);
            let bucket_max = if i == bucket_count - 1 { max_val } else { min_val + ((i + 1) as f64 * width) };

            let range = ValueRange {
                min: bucket_min,
                max: bucket_max,
                inclusive_min: true,
                inclusive_max: i == bucket_count - 1,
            };

            buckets.push(Bucket::new(range));
        }

        // Populate buckets with data
        let mut value_counts: BTreeMap<i64, u64> = BTreeMap::new();
        for &value in data {
            let key = (value * 1000.0) as i64; // Simple precision handling
            *value_counts.entry(key).or_insert(0) += 1;
        }

        for (value_key, count) in value_counts {
            let value = value_key as f64 / 1000.0;
            let bucket_index = if value == max_val { bucket_count - 1 } else { ((value - min_val) / width).floor() as usize };

            if bucket_index < buckets.len() {
                buckets[bucket_index].add_value(value, count);
            }
        }

        Ok(buckets)
    }

    fn create_equal_frequency_buckets(bucket_count: usize, data: &[f64]) -> Result<Vec<Bucket>, HistogramError> {
        if bucket_count == 0 {
            return Err(HistogramError::InvalidBucket("Bucket count must be greater than 0".to_string()));
        }

        let target_frequency = data.len() / bucket_count;
        let mut buckets = Vec::new();
        let mut current_bucket_start = 0;

        for i in 0..bucket_count {
            let bucket_end = if i == bucket_count - 1 {
                data.len()
            } else {
                std::cmp::min(current_bucket_start + target_frequency, data.len())
            };

            if current_bucket_start < data.len() && bucket_end > current_bucket_start {
                let min_val = data[current_bucket_start];
                let max_val = data[bucket_end - 1];

                let range = ValueRange {
                    min: min_val,
                    max: max_val,
                    inclusive_min: true,
                    inclusive_max: i == bucket_count - 1,
                };

                let mut bucket = Bucket::new(range);
                bucket.count = (bucket_end - current_bucket_start) as u64;

                buckets.push(bucket);
                current_bucket_start = bucket_end;
            }
        }

        Ok(buckets)
    }

    fn create_custom_buckets(boundaries: Vec<f64>, data: &[f64]) -> Result<Vec<Bucket>, HistogramError> {
        if boundaries.len() < 2 {
            return Err(HistogramError::InvalidBucket("At least 2 boundaries required".to_string()));
        }

        let mut sorted_boundaries = boundaries;
        sorted_boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut buckets = Vec::new();
        for i in 0..sorted_boundaries.len() - 1 {
            let range = ValueRange {
                min: sorted_boundaries[i],
                max: sorted_boundaries[i + 1],
                inclusive_min: true,
                inclusive_max: i == sorted_boundaries.len() - 2,
            };
            buckets.push(Bucket::new(range));
        }

        // Populate buckets
        for &value in data {
            for bucket in &mut buckets {
                if bucket.range.contains(value) {
                    bucket.add_value(value, 1);
                    break;
                }
            }
        }

        Ok(buckets)
    }

    pub fn estimate_selectivity(&self, value: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        for bucket in &self.buckets {
            if bucket.range.contains(value) {
                return bucket.selectivity(self.total_count);
            }
        }

        0.0
    }

    pub fn estimate_range_selectivity(&self, min: f64, max: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        let mut total_count = 0u64;

        for bucket in &self.buckets {
            // Check if bucket overlaps with the range
            if bucket.range.max > min && bucket.range.min < max {
                // Calculate overlap ratio
                let overlap_min = bucket.range.min.max(min);
                let overlap_max = bucket.range.max.min(max);
                let overlap_ratio = (overlap_max - overlap_min) / bucket.range.width();

                total_count += (bucket.count as f64 * overlap_ratio) as u64;
            }
        }

        total_count as f64 / self.total_count as f64
    }

    pub fn get_bucket_for_value(&self, value: f64) -> Option<&Bucket> {
        self.buckets.iter().find(|bucket| bucket.range.contains(value))
    }

    pub fn non_null_count(&self) -> u64 {
        self.total_count - self.null_count
    }

    pub fn null_fraction(&self) -> f64 {
        if self.total_count == 0 { 0.0 } else { self.null_count as f64 / self.total_count as f64 }
    }

    pub fn update_timestamp(&mut self) {
        self.last_updated = crate::storage_engine::generate_timestamp();
    }

    pub fn merge_with(&mut self, other: &Histogram) -> Result<(), HistogramError> {
        if self.histogram_type != other.histogram_type {
            return Err(HistogramError::InvalidBucket("Cannot merge histograms of different types".to_string()));
        }

        self.total_count += other.total_count;
        self.null_count += other.null_count;

        // Update min/max values
        if let Some(other_min) = other.min_value {
            self.min_value = Some(self.min_value.map_or(other_min, |min| min.min(other_min)));
        }

        if let Some(other_max) = other.max_value {
            self.max_value = Some(self.max_value.map_or(other_max, |max| max.max(other_max)));
        }

        self.update_timestamp();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_range_contains() {
        let range = ValueRange::new(10.0, 20.0);
        assert!(range.contains(15.0));
        assert!(range.contains(10.0));
        assert!(!range.contains(20.0));
        assert!(!range.contains(5.0));
        assert!(!range.contains(25.0));
    }

    #[test]
    fn test_bucket_creation() {
        let range = ValueRange::new(0.0, 100.0);
        let mut bucket = Bucket::new(range);

        bucket.add_value(50.0, 5);
        bucket.add_value(75.0, 3);
        bucket.add_value(25.0, 8);

        assert_eq!(bucket.count, 16);
        assert_eq!(bucket.most_common_value, Some(25.0));
        assert_eq!(bucket.mcv_frequency, 8);
    }

    #[test]
    fn test_equal_width_histogram() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let strategy = BucketStrategy::FixedWidth { bucket_count: 5 };

        let histogram = Histogram::create_with_strategy(strategy, &data).unwrap();

        assert_eq!(histogram.buckets.len(), 5);
        assert_eq!(histogram.total_count, 10);
        assert_eq!(histogram.min_value, Some(1.0));
        assert_eq!(histogram.max_value, Some(10.0));
    }

    #[test]
    fn test_equal_frequency_histogram() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let strategy = BucketStrategy::FixedFrequency { bucket_count: 3 };

        let histogram = Histogram::create_with_strategy(strategy, &data).unwrap();

        assert_eq!(histogram.buckets.len(), 3);
        assert_eq!(histogram.total_count, 10);
    }

    #[test]
    fn test_custom_boundaries_histogram() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let boundaries = vec![0.0, 3.0, 7.0, 11.0];
        let strategy = BucketStrategy::CustomBoundaries { boundaries };

        let histogram = Histogram::create_with_strategy(strategy, &data).unwrap();

        assert_eq!(histogram.buckets.len(), 3);
        assert_eq!(histogram.total_count, 10);
    }

    #[test]
    fn test_selectivity_estimation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let strategy = BucketStrategy::FixedWidth { bucket_count: 5 };
        let histogram = Histogram::create_with_strategy(strategy, &data).unwrap();

        let selectivity = histogram.estimate_selectivity(3.0);
        assert!(selectivity > 0.0);
        assert!(selectivity <= 1.0);
    }

    #[test]
    fn test_range_selectivity_estimation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let strategy = BucketStrategy::FixedWidth { bucket_count: 5 };
        let histogram = Histogram::create_with_strategy(strategy, &data).unwrap();

        let selectivity = histogram.estimate_range_selectivity(3.0, 7.0);
        assert!(selectivity > 0.0);
        assert!(selectivity <= 1.0);
    }

    #[test]
    fn test_histogram_merge() {
        let data1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let data2 = vec![6.0, 7.0, 8.0, 9.0, 10.0];
        let strategy = BucketStrategy::FixedWidth { bucket_count: 3 };

        let mut hist1 = Histogram::create_with_strategy(strategy.clone(), &data1).unwrap();
        let hist2 = Histogram::create_with_strategy(strategy, &data2).unwrap();

        hist1.merge_with(&hist2).unwrap();
        assert_eq!(hist1.total_count, 10);
    }

    #[test]
    fn test_empty_histogram_error() {
        let data: Vec<f64> = vec![];
        let strategy = BucketStrategy::FixedWidth { bucket_count: 5 };

        let result = Histogram::create_with_strategy(strategy, &data);
        assert!(matches!(result, Err(HistogramError::EmptyHistogram)));
    }

    #[test]
    fn test_invalid_bucket_count() {
        let data = vec![1.0, 2.0, 3.0];
        let strategy = BucketStrategy::FixedWidth { bucket_count: 0 };

        let result = Histogram::create_with_strategy(strategy, &data);
        assert!(matches!(result, Err(HistogramError::InvalidBucket(_))));
    }
}
