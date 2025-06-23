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
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Type of access pattern
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Sequential,
    Random,
    HotSpot,
    Temporal,
    Range,
}

/// Access pattern statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessStats {
    pub total_accesses: u64,
    pub unique_keys: u64,
    pub sequential_ratio: f64,
    pub hot_key_ratio: f64,
    pub average_gap: f64,
    pub peak_access_time: Option<u64>,
    pub access_frequency: f64,
}

/// Represents an access pattern detected in the data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub start_time: u64,
    pub end_time: u64,
    pub key_range: Option<(String, String)>,
    pub access_count: u64,
    pub metadata: HashMap<String, String>,
}

/// Temporal access pattern for time-series analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAccessPattern {
    pub time_window: Duration,
    pub access_counts: VecDeque<(u64, u64)>, // (timestamp, count)
    pub peak_hours: Vec<u8>,
    pub trend: f64, // positive for increasing, negative for decreasing
    pub seasonality: Option<Duration>,
}

impl TemporalAccessPattern {
    pub fn new(time_window: Duration) -> Self {
        Self {
            time_window,
            access_counts: VecDeque::new(),
            peak_hours: Vec::new(),
            trend: 0.0,
            seasonality: None,
        }
    }

    pub fn add_access(&mut self, timestamp: u64) {
        let now = timestamp;

        // Remove old entries outside the time window
        let cutoff = now.saturating_sub(self.time_window.as_secs());
        while let Some(&(ts, _)) = self.access_counts.front() {
            if ts < cutoff {
                self.access_counts.pop_front();
            } else {
                break;
            }
        }

        // Add or update current timestamp
        if let Some(&mut (ref mut last_ts, ref mut count)) = self.access_counts.back_mut() {
            if *last_ts == now {
                *count += 1;
                return;
            }
        }

        self.access_counts.push_back((now, 1));
        self.update_analysis();
    }

    fn update_analysis(&mut self) {
        if self.access_counts.len() < 2 {
            return;
        }

        // Calculate trend
        let counts: Vec<f64> = self.access_counts.iter().map(|(_, count)| *count as f64).collect();
        self.trend = self.calculate_trend(&counts);

        // Update peak hours
        self.update_peak_hours();
    }

    fn calculate_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        let denominator = n * sum_x2 - sum_x.powi(2);
        if denominator.abs() < f64::EPSILON { 0.0 } else { (n * sum_xy - sum_x * sum_y) / denominator }
    }

    fn update_peak_hours(&mut self) {
        let mut hour_counts: [u64; 24] = [0; 24];

        for &(timestamp, count) in &self.access_counts {
            if let Ok(duration) = UNIX_EPOCH.elapsed() {
                let secs_since_epoch = duration.as_secs();
                let hour = ((timestamp % 86400) / 3600) as usize;
                if hour < 24 {
                    hour_counts[hour] += count;
                }
            }
        }

        let max_count = *hour_counts.iter().max().unwrap_or(&0);
        let threshold = max_count * 8 / 10; // 80% of max

        self.peak_hours = hour_counts.iter().enumerate().filter(|(_, count)| **count >= threshold).map(|(hour, _)| hour as u8).collect();
    }

    pub fn get_access_rate(&self) -> f64 {
        if self.access_counts.is_empty() {
            return 0.0;
        }

        let total_accesses: u64 = self.access_counts.iter().map(|(_, count)| count).sum();
        let time_span = if let (Some(&(first, _)), Some(&(last, _))) = (self.access_counts.front(), self.access_counts.back()) {
            last.saturating_sub(first).max(1)
        } else {
            1
        };

        total_accesses as f64 / time_span as f64
    }

    pub fn is_peak_hour(&self, hour: u8) -> bool {
        self.peak_hours.contains(&hour)
    }
}

/// Main access pattern tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPatternTracker {
    access_history: VecDeque<(String, u64)>, // (key, timestamp)
    pattern_history: Vec<AccessPattern>,
    temporal_patterns: HashMap<String, TemporalAccessPattern>,
    key_frequencies: HashMap<String, u64>,
    sequential_threshold: f64,
    hot_key_threshold: f64,
    max_history_size: usize,
    created_at: u64,
    last_updated: u64,
}

impl AccessPatternTracker {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            access_history: VecDeque::with_capacity(max_history_size),
            pattern_history: Vec::new(),
            temporal_patterns: HashMap::new(),
            key_frequencies: HashMap::new(),
            sequential_threshold: 0.7,
            hot_key_threshold: 0.1,
            max_history_size,
            created_at: crate::storage_engine::generate_timestamp(),
            last_updated: crate::storage_engine::generate_timestamp(),
        }
    }

    pub fn record_access(&mut self, key: &str) {
        let timestamp = crate::storage_engine::generate_timestamp();
        self.last_updated = timestamp;

        // Update key frequency
        *self.key_frequencies.entry(key.to_string()).or_insert(0) += 1;

        // Add to access history
        self.access_history.push_back((key.to_string(), timestamp));
        if self.access_history.len() > self.max_history_size {
            if let Some((old_key, _)) = self.access_history.pop_front() {
                // Decrement frequency for removed access
                if let Some(freq) = self.key_frequencies.get_mut(&old_key) {
                    *freq = freq.saturating_sub(1);
                    if *freq == 0 {
                        self.key_frequencies.remove(&old_key);
                    }
                }
            }
        }

        // Update temporal patterns
        self.temporal_patterns
            .entry(key.to_string())
            .or_insert_with(|| TemporalAccessPattern::new(Duration::from_secs(3600))) // 1 hour window
            .add_access(timestamp);

        // Analyze patterns periodically
        if self.access_history.len() % 100 == 0 {
            self.analyze_patterns();
        }
    }

    pub fn analyze_patterns(&mut self) {
        let current_time = crate::storage_engine::generate_timestamp();

        // Detect sequential patterns
        if let Some(sequential_pattern) = self.detect_sequential_pattern() {
            self.pattern_history.push(sequential_pattern);
        }

        // Detect hot spots
        if let Some(hotspot_pattern) = self.detect_hotspot_pattern() {
            self.pattern_history.push(hotspot_pattern);
        }

        // Detect temporal patterns
        for temporal_pattern in self.detect_temporal_patterns() {
            self.pattern_history.push(temporal_pattern);
        }

        // Clean up old patterns (keep only last 1000)
        if self.pattern_history.len() > 1000 {
            self.pattern_history.drain(0..self.pattern_history.len() - 1000);
        }
    }

    fn detect_sequential_pattern(&self) -> Option<AccessPattern> {
        if self.access_history.len() < 10 {
            return None;
        }

        let recent_accesses: Vec<_> = self.access_history.iter().rev().take(100).collect();

        let mut sequential_count = 0;
        let mut prev_key: Option<&str> = None;

        for (key, _) in &recent_accesses {
            if let Some(prev) = prev_key {
                if self.is_sequential_key(prev, key) {
                    sequential_count += 1;
                }
            }
            prev_key = Some(key);
        }

        let sequential_ratio = sequential_count as f64 / (recent_accesses.len() - 1) as f64;

        if sequential_ratio >= self.sequential_threshold {
            let start_time = recent_accesses.last().map(|(_, ts)| *ts).unwrap_or(0);
            let end_time = recent_accesses.first().map(|(_, ts)| *ts).unwrap_or(0);

            Some(AccessPattern {
                pattern_type: PatternType::Sequential,
                confidence: sequential_ratio,
                start_time,
                end_time,
                key_range: None,
                access_count: recent_accesses.len() as u64,
                metadata: HashMap::new(),
            })
        } else {
            None
        }
    }

    fn detect_hotspot_pattern(&self) -> Option<AccessPattern> {
        if self.key_frequencies.is_empty() {
            return None;
        }

        let total_accesses: u64 = self.key_frequencies.values().sum();
        let mut hot_keys = Vec::new();

        for (key, &freq) in &self.key_frequencies {
            let ratio = freq as f64 / total_accesses as f64;
            if ratio >= self.hot_key_threshold {
                hot_keys.push((key.clone(), freq, ratio));
            }
        }

        if !hot_keys.is_empty() {
            hot_keys.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by frequency desc

            let total_hot_accesses: u64 = hot_keys.iter().map(|(_, freq, _)| freq).sum();
            let confidence = total_hot_accesses as f64 / total_accesses as f64;

            let mut metadata = HashMap::new();
            metadata.insert("hot_key_count".to_string(), hot_keys.len().to_string());
            metadata.insert("top_key".to_string(), hot_keys[0].0.clone());
            metadata.insert("top_key_frequency".to_string(), hot_keys[0].1.to_string());

            Some(AccessPattern {
                pattern_type: PatternType::HotSpot,
                confidence,
                start_time: self.created_at,
                end_time: self.last_updated,
                key_range: None,
                access_count: total_hot_accesses,
                metadata,
            })
        } else {
            None
        }
    }

    fn detect_temporal_patterns(&self) -> Vec<AccessPattern> {
        let mut patterns = Vec::new();

        for (key, temporal_pattern) in &self.temporal_patterns {
            if temporal_pattern.access_counts.len() < 5 {
                continue;
            }

            let access_rate = temporal_pattern.get_access_rate();
            if access_rate > 1.0 {
                // More than 1 access per second
                let mut metadata = HashMap::new();
                metadata.insert("access_rate".to_string(), access_rate.to_string());
                metadata.insert("trend".to_string(), temporal_pattern.trend.to_string());
                metadata.insert("peak_hours".to_string(), format!("{:?}", temporal_pattern.peak_hours));

                patterns.push(AccessPattern {
                    pattern_type: PatternType::Temporal,
                    confidence: (access_rate / 10.0).min(1.0), // Normalize to 0-1
                    start_time: temporal_pattern.access_counts.front().map(|(ts, _)| *ts).unwrap_or(0),
                    end_time: temporal_pattern.access_counts.back().map(|(ts, _)| *ts).unwrap_or(0),
                    key_range: Some((key.clone(), key.clone())),
                    access_count: temporal_pattern.access_counts.iter().map(|(_, count)| count).sum(),
                    metadata,
                });
            }
        }

        patterns
    }

    fn is_sequential_key(&self, key1: &str, key2: &str) -> bool {
        // Simple heuristic: check if keys are numerically sequential
        if let (Ok(num1), Ok(num2)) = (key1.parse::<i64>(), key2.parse::<i64>()) {
            (num2 - num1).abs() == 1
        } else {
            // For string keys, check if they're lexicographically close
            let diff = key1.len().abs_diff(key2.len());
            diff <= 1 && key1.chars().zip(key2.chars()).filter(|(a, b)| a != b).count() <= 2
        }
    }

    pub fn get_access_stats(&self) -> AccessStats {
        let total_accesses = self.access_history.len() as u64;
        let unique_keys = self.key_frequencies.len() as u64;

        let sequential_ratio = if total_accesses > 1 {
            let mut sequential_count = 0;
            let accesses: Vec<_> = self.access_history.iter().collect();

            for i in 1..accesses.len() {
                if self.is_sequential_key(&accesses[i - 1].0, &accesses[i].0) {
                    sequential_count += 1;
                }
            }

            sequential_count as f64 / (total_accesses - 1) as f64
        } else {
            0.0
        };

        let hot_key_ratio = if total_accesses > 0 {
            let hot_accesses: u64 = self.key_frequencies.values().filter(|&&freq| freq as f64 / total_accesses as f64 >= self.hot_key_threshold).sum();
            hot_accesses as f64 / total_accesses as f64
        } else {
            0.0
        };

        let average_gap = if self.access_history.len() > 1 {
            let time_diffs: Vec<_> = self
                .access_history
                .iter()
                .zip(self.access_history.iter().skip(1))
                .map(|((_, t1), (_, t2))| t2.saturating_sub(*t1))
                .collect();

            time_diffs.iter().sum::<u64>() as f64 / time_diffs.len() as f64
        } else {
            0.0
        };

        let peak_access_time = self
            .key_frequencies
            .iter()
            .max_by_key(|(_, freq)| *freq)
            .and_then(|(key, _)| self.access_history.iter().find(|(k, _)| k == key).map(|(_, ts)| *ts));

        let access_frequency = if !self.access_history.is_empty() {
            let time_span = self.last_updated.saturating_sub(self.created_at).max(1);
            total_accesses as f64 / time_span as f64
        } else {
            0.0
        };

        AccessStats {
            total_accesses,
            unique_keys,
            sequential_ratio,
            hot_key_ratio,
            average_gap,
            peak_access_time,
            access_frequency,
        }
    }

    pub fn get_patterns(&self) -> &[AccessPattern] {
        &self.pattern_history
    }

    pub fn get_patterns_by_type(&self, pattern_type: PatternType) -> Vec<&AccessPattern> {
        self.pattern_history.iter().filter(|pattern| pattern.pattern_type == pattern_type).collect()
    }

    pub fn get_hot_keys(&self, top_n: usize) -> Vec<(String, u64)> {
        let mut sorted_keys: Vec<_> = self.key_frequencies.iter().collect();
        sorted_keys.sort_by(|a, b| b.1.cmp(a.1));

        sorted_keys.into_iter().take(top_n).map(|(key, &freq)| (key.clone(), freq)).collect()
    }

    pub fn get_temporal_pattern(&self, key: &str) -> Option<&TemporalAccessPattern> {
        self.temporal_patterns.get(key)
    }

    pub fn set_thresholds(&mut self, sequential_threshold: f64, hot_key_threshold: f64) {
        self.sequential_threshold = sequential_threshold.clamp(0.0, 1.0);
        self.hot_key_threshold = hot_key_threshold.clamp(0.0, 1.0);
    }

    pub fn clear_history(&mut self) {
        self.access_history.clear();
        self.pattern_history.clear();
        self.temporal_patterns.clear();
        self.key_frequencies.clear();
        self.last_updated = crate::storage_engine::generate_timestamp();
    }

    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.access_history.capacity() * std::mem::size_of::<(String, u64)>()
            + self.pattern_history.capacity() * std::mem::size_of::<AccessPattern>()
            + self.temporal_patterns.iter().map(|(k, v)| k.len() + std::mem::size_of_val(v)).sum::<usize>()
            + self.key_frequencies.iter().map(|(k, _)| k.len()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_access_pattern() {
        let mut pattern = TemporalAccessPattern::new(Duration::from_secs(3600));

        let base_time = 1000000;
        pattern.add_access(base_time);
        pattern.add_access(base_time + 100);
        pattern.add_access(base_time + 200);

        assert_eq!(pattern.access_counts.len(), 3);
        assert!(pattern.get_access_rate() > 0.0);
    }

    #[test]
    fn test_access_pattern_tracker() {
        let mut tracker = AccessPatternTracker::new(1000);

        // Record some accesses
        tracker.record_access("key1");
        tracker.record_access("key2");
        tracker.record_access("key3");
        tracker.record_access("key1"); // duplicate

        let stats = tracker.get_access_stats();
        assert_eq!(stats.total_accesses, 4);
        assert_eq!(stats.unique_keys, 3);

        let hot_keys = tracker.get_hot_keys(5);
        assert_eq!(hot_keys[0].0, "key1");
        assert_eq!(hot_keys[0].1, 2);
    }

    #[test]
    fn test_sequential_pattern_detection() {
        let mut tracker = AccessPatternTracker::new(1000);

        // Add sequential numeric keys
        for i in 1..=20 {
            tracker.record_access(&i.to_string());
        }

        tracker.analyze_patterns();
        let patterns = tracker.get_patterns_by_type(PatternType::Sequential);
        assert!(!patterns.is_empty());
        assert!(patterns[0].confidence >= 0.7);
    }

    #[test]
    fn test_hotspot_pattern_detection() {
        let mut tracker = AccessPatternTracker::new(1000);

        // Create a hot key by accessing it many times
        for _ in 0..50 {
            tracker.record_access("hot_key");
        }

        // Add some other keys
        for i in 1..=10 {
            tracker.record_access(&format!("cold_key_{}", i));
        }

        tracker.analyze_patterns();
        let patterns = tracker.get_patterns_by_type(PatternType::HotSpot);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_pattern_thresholds() {
        let mut tracker = AccessPatternTracker::new(1000);

        tracker.set_thresholds(0.8, 0.2);
        // Thresholds should be clamped to valid range
        tracker.set_thresholds(1.5, -0.1);

        // Should not panic and values should be valid
        let stats = tracker.get_access_stats();
        assert!(stats.sequential_ratio >= 0.0);
        assert!(stats.hot_key_ratio >= 0.0);
    }

    #[test]
    fn test_memory_usage() {
        let tracker = AccessPatternTracker::new(1000);
        let usage = tracker.memory_usage();
        assert!(usage > 0);
    }

    #[test]
    fn test_clear_history() {
        let mut tracker = AccessPatternTracker::new(1000);

        tracker.record_access("test_key");
        assert_eq!(tracker.get_access_stats().total_accesses, 1);

        tracker.clear_history();
        assert_eq!(tracker.get_access_stats().total_accesses, 0);
        assert_eq!(tracker.get_access_stats().unique_keys, 0);
    }
}
