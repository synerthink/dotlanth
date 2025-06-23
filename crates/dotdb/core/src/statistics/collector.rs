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
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::RwLock;

use super::{AccessPatternTracker, BucketStrategy, CardinalityEstimator, CardinalityMethod, Histogram};

#[derive(Debug, Error)]
pub enum StatisticsError {
    #[error("Table not found: {0}")]
    TableNotFound(String),
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Collection failed: {0}")]
    CollectionFailed(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

pub type StatisticsResult<T> = Result<T, StatisticsError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateStrategy {
    Immediate,
    Periodic { interval_seconds: u64 },
    OnThreshold { change_threshold: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsConfig {
    pub max_histogram_buckets: usize,
    pub cardinality_method: CardinalityMethod,
    pub update_strategy: UpdateStrategy,
    pub access_pattern_history_size: usize,
    pub enable_temporal_patterns: bool,
    pub statistics_retention_days: u32,
}

impl Default for StatisticsConfig {
    fn default() -> Self {
        Self {
            max_histogram_buckets: 100,
            cardinality_method: CardinalityMethod::default(),
            update_strategy: UpdateStrategy::Periodic { interval_seconds: 3600 },
            access_pattern_history_size: 10000,
            enable_temporal_patterns: true,
            statistics_retention_days: 30,
        }
    }
}

#[derive(Debug)]
struct TableStatistics {
    histograms: HashMap<String, Histogram>,
    cardinality_estimators: HashMap<String, CardinalityEstimator>,
    access_tracker: AccessPatternTracker,
    row_count: u64,
    last_updated: u64,
}

#[derive(Debug)]
pub struct StatisticsCollector {
    config: StatisticsConfig,
    table_stats: RwLock<HashMap<String, TableStatistics>>,
    created_at: u64,
}

impl StatisticsCollector {
    pub fn new(config: StatisticsConfig) -> Self {
        Self {
            config,
            table_stats: RwLock::new(HashMap::new()),
            created_at: crate::storage_engine::generate_timestamp(),
        }
    }

    pub async fn collect_table_statistics(&self, table_name: &str) -> StatisticsResult<()> {
        let mut stats = self.table_stats.write().await;

        let table_stats = stats.entry(table_name.to_string()).or_insert_with(|| TableStatistics {
            histograms: HashMap::new(),
            cardinality_estimators: HashMap::new(),
            access_tracker: AccessPatternTracker::new(self.config.access_pattern_history_size),
            row_count: 0,
            last_updated: crate::storage_engine::generate_timestamp(),
        });

        table_stats.last_updated = crate::storage_engine::generate_timestamp();
        Ok(())
    }

    pub async fn update_histogram(&self, table: &str, column: &str, data: &[f64]) -> StatisticsResult<()> {
        let strategy = BucketStrategy::FixedWidth {
            bucket_count: self.config.max_histogram_buckets,
        };

        let histogram = Histogram::create_with_strategy(strategy, data).map_err(|e| StatisticsError::CollectionFailed(e.to_string()))?;

        let mut stats = self.table_stats.write().await;
        let table_stats = stats.get_mut(table).ok_or_else(|| StatisticsError::TableNotFound(table.to_string()))?;

        table_stats.histograms.insert(column.to_string(), histogram);
        table_stats.last_updated = crate::storage_engine::generate_timestamp();

        Ok(())
    }

    pub async fn get_histogram(&self, table: &str, column: &str) -> StatisticsResult<Option<Histogram>> {
        let stats = self.table_stats.read().await;
        let table_stats = stats.get(table).ok_or_else(|| StatisticsError::TableNotFound(table.to_string()))?;

        Ok(table_stats.histograms.get(column).cloned())
    }

    pub async fn get_cardinality_estimate(&self, table: &str, column: &str) -> StatisticsResult<u64> {
        let stats = self.table_stats.read().await;
        let table_stats = stats.get(table).ok_or_else(|| StatisticsError::TableNotFound(table.to_string()))?;

        Ok(table_stats.cardinality_estimators.get(column).map(|est| est.estimate()).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_statistics_collector() {
        let config = StatisticsConfig::default();
        let collector = StatisticsCollector::new(config);

        collector.collect_table_statistics("test_table").await.unwrap();

        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        collector.update_histogram("test_table", "test_column", &data).await.unwrap();

        let histogram = collector.get_histogram("test_table", "test_column").await.unwrap();
        assert!(histogram.is_some());
    }
}
