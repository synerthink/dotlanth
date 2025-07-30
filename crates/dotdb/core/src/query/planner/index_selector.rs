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

use crate::indices::{IndexStats, IndexType};

#[derive(Debug, Error)]
pub enum IndexSelectionError {
    #[error("No suitable index found for query: {0}")]
    NoSuitableIndex(String),
    #[error("Index statistics unavailable: {0}")]
    StatisticsUnavailable(String),
    #[error("Invalid query predicate: {0}")]
    InvalidPredicate(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexUsageHint {
    FullScan,
    IndexScan { index_name: String, selectivity: f64 },
    CompositeIndex { index_name: String, fields: Vec<String> },
    MultipleIndexes { indexes: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendation {
    pub usage_hint: IndexUsageHint,
    pub estimated_cost: f64,
    pub confidence: f64,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPredicate {
    pub column: String,
    pub operator: PredicateOperator,
    pub value: PredicateValue,
    pub selectivity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredicateOperator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    In,
    Between,
    Like,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredicateValue {
    Single(String),
    List(Vec<String>),
    Range(String, String),
    Null,
}

pub struct IndexSelector {
    available_indexes: HashMap<String, IndexInfo>,
    statistics: HashMap<String, IndexStats>,
}

#[derive(Debug, Clone)]
pub struct IndexInfo {
    index_type: IndexType,
    columns: Vec<String>,
    cardinality: u64,
    is_unique: bool,
    size_bytes: u64,
}

impl IndexSelector {
    pub fn new() -> Self {
        Self {
            available_indexes: HashMap::new(),
            statistics: HashMap::new(),
        }
    }

    pub fn register_index(&mut self, name: String, info: IndexInfo) {
        self.available_indexes.insert(name, info);
    }

    pub fn select_best_index(&self, predicates: &[QueryPredicate], table_size: u64) -> Result<IndexRecommendation, IndexSelectionError> {
        if predicates.is_empty() {
            return Ok(IndexRecommendation {
                usage_hint: IndexUsageHint::FullScan,
                estimated_cost: table_size as f64,
                confidence: 1.0,
                reasoning: "No predicates, full scan required".to_string(),
            });
        }

        let mut candidates = Vec::new();

        // Evaluate each available index
        for (index_name, index_info) in &self.available_indexes {
            if let Some(score) = self.evaluate_index(index_info, predicates, table_size) {
                candidates.push((index_name.clone(), score));
            }
        }

        if candidates.is_empty() {
            let full_scan_cost = table_size as f64;
            return Ok(IndexRecommendation {
                usage_hint: IndexUsageHint::FullScan,
                estimated_cost: full_scan_cost,
                confidence: 0.8,
                reasoning: "No suitable indexes found".to_string(),
            });
        }

        // Sort by score (lower is better)
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let (best_index, best_cost) = &candidates[0];
        let index_info = &self.available_indexes[best_index];

        let selectivity = self.calculate_combined_selectivity(predicates);

        Ok(IndexRecommendation {
            usage_hint: IndexUsageHint::IndexScan {
                index_name: best_index.clone(),
                selectivity,
            },
            estimated_cost: *best_cost,
            confidence: 0.9,
            reasoning: format!("Selected {best_index} index with estimated cost {best_cost}"),
        })
    }

    fn evaluate_index(&self, index_info: &IndexInfo, predicates: &[QueryPredicate], table_size: u64) -> Option<f64> {
        let matching_predicates = predicates.iter().filter(|p| index_info.columns.contains(&p.column)).count();

        if matching_predicates == 0 {
            return None;
        }

        let selectivity = self.calculate_combined_selectivity(predicates);
        let estimated_rows = (table_size as f64 * selectivity) as u64;

        // Base cost calculation
        let index_scan_cost = (index_info.size_bytes / 8192) as f64; // Assume 8KB pages
        let data_access_cost = estimated_rows as f64 * 1.2; // Random I/O penalty

        Some(index_scan_cost + data_access_cost)
    }

    fn calculate_combined_selectivity(&self, predicates: &[QueryPredicate]) -> f64 {
        predicates
            .iter()
            .map(|p| p.selectivity.unwrap_or(0.1)) // Default selectivity
            .fold(1.0, |acc, sel| acc * sel)
    }

    pub fn recommend_composite_index(&self, predicates: &[QueryPredicate]) -> Option<Vec<String>> {
        if predicates.len() < 2 {
            return None;
        }

        let columns: Vec<_> = predicates.iter().map(|p| p.column.clone()).collect();

        // Sort by selectivity (most selective first)
        let mut sorted_predicates = predicates.to_vec();
        sorted_predicates.sort_by(|a, b| {
            let sel_a = a.selectivity.unwrap_or(0.1);
            let sel_b = b.selectivity.unwrap_or(0.1);
            sel_a.partial_cmp(&sel_b).unwrap()
        });

        Some(sorted_predicates.into_iter().map(|p| p.column).collect())
    }
}

impl Default for IndexSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_selector_creation() {
        let selector = IndexSelector::new();
        assert!(selector.available_indexes.is_empty());
    }

    #[test]
    fn test_full_scan_recommendation() {
        let selector = IndexSelector::new();
        let predicates = vec![];

        let recommendation = selector.select_best_index(&predicates, 1000).unwrap();
        assert!(matches!(recommendation.usage_hint, IndexUsageHint::FullScan));
    }

    #[test]
    fn test_index_registration() {
        let mut selector = IndexSelector::new();

        let index_info = IndexInfo {
            index_type: IndexType::BPlusTree,
            columns: vec!["id".to_string()],
            cardinality: 1000,
            is_unique: true,
            size_bytes: 8192,
        };

        selector.register_index("idx_id".to_string(), index_info);
        assert_eq!(selector.available_indexes.len(), 1);
    }

    #[test]
    fn test_composite_index_recommendation() {
        let selector = IndexSelector::new();

        let predicates = vec![
            QueryPredicate {
                column: "name".to_string(),
                operator: PredicateOperator::Equal,
                value: PredicateValue::Single("test".to_string()),
                selectivity: Some(0.1),
            },
            QueryPredicate {
                column: "age".to_string(),
                operator: PredicateOperator::Greater,
                value: PredicateValue::Single("25".to_string()),
                selectivity: Some(0.3),
            },
        ];

        let recommendation = selector.recommend_composite_index(&predicates);
        assert!(recommendation.is_some());
        assert_eq!(recommendation.unwrap().len(), 2);
    }
}
