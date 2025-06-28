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

use crate::fs::FileMetadata;
use std::collections::HashMap;

/// Represents the type of compaction strategy to be used.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompactionStrategyType {
    SizeTiered,
    Leveled,
    TimeWindow,
    Custom,
}

/// Represents a compaction task, including the files to be compacted and task metadata.
#[derive(Debug, Clone)]
pub struct CompactionTask {
    pub id: u64,
    pub strategy_type: CompactionStrategyType,
    pub input_files: Vec<FileMetadata>,
    pub estimated_output_size: u64,
    pub priority: u8, // 0-255, higher is more priority
    pub created_at: std::time::SystemTime,
}

/// Configuration for a compaction strategy, including thresholds and parameters.
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub strategy_type: CompactionStrategyType,
    pub max_file_size: u64,
    pub min_merge_files: usize,
    pub max_merge_files: usize,
    pub size_ratio_threshold: f64,
    pub level_multiplier: usize,
    pub max_levels: usize,
    pub time_window_size: std::time::Duration,
}

impl Default for CompactionConfig {
    /// Returns a default configuration for compaction strategies.
    fn default() -> Self {
        Self {
            strategy_type: CompactionStrategyType::SizeTiered,
            max_file_size: 100 * 1024 * 1024, // 100MB
            min_merge_files: 2,
            max_merge_files: 10,
            size_ratio_threshold: 0.5,
            level_multiplier: 10,
            max_levels: 7,
            time_window_size: std::time::Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Trait for all compaction strategies. Each strategy must implement these methods.
pub trait CompactionStrategy {
    /// Determines if compaction should be triggered for the given files.
    fn should_compact(&self, files: &[FileMetadata]) -> bool;
    /// Selects files to be compacted and returns a list of compaction tasks.
    fn select_files_for_compaction(&self, files: &[FileMetadata]) -> Vec<CompactionTask>;
    /// Returns the type of the compaction strategy.
    fn strategy_type(&self) -> CompactionStrategyType;
}

/// Implements the size-tiered compaction strategy, grouping files by similar sizes.
pub struct SizeTieredStrategy {
    config: CompactionConfig,
}

impl SizeTieredStrategy {
    /// Creates a new SizeTieredStrategy with the given configuration.
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Groups files into buckets based on their size (power-of-2 buckets).
    fn group_files_by_size(&self, files: &[FileMetadata]) -> HashMap<u64, Vec<FileMetadata>> {
        let mut groups = HashMap::new();

        for file in files {
            // Group files into size buckets (power of 2)
            let size_bucket = if file.size == 0 {
                0
            } else {
                let log_size = (file.size as f64).log2() as u64;
                1u64 << log_size
            };

            groups.entry(size_bucket).or_insert_with(Vec::new).push(file.clone());
        }

        groups
    }
}

impl CompactionStrategy for SizeTieredStrategy {
    /// Returns true if any size group has enough files to trigger compaction.
    fn should_compact(&self, files: &[FileMetadata]) -> bool {
        if files.len() < self.config.min_merge_files {
            return false;
        }

        let groups = self.group_files_by_size(files);

        // Check if any group has enough files for compaction
        groups.values().any(|group| group.len() >= self.config.min_merge_files)
    }

    /// Selects files for compaction from each size group and creates compaction tasks, sorted by priority.
    fn select_files_for_compaction(&self, files: &[FileMetadata]) -> Vec<CompactionTask> {
        let mut tasks = Vec::new();
        let groups = self.group_files_by_size(files);

        for (size_bucket, group_files) in groups {
            if group_files.len() >= self.config.min_merge_files {
                let files_to_merge = group_files.into_iter().take(self.config.max_merge_files).collect::<Vec<_>>();

                let estimated_size = files_to_merge.iter().map(|f| f.size).sum();
                let priority = self.calculate_priority(&files_to_merge, size_bucket);

                tasks.push(CompactionTask {
                    id: size_bucket + (files_to_merge.len() as u64 * 1000), // Deterministic ID for consistency
                    strategy_type: CompactionStrategyType::SizeTiered,
                    input_files: files_to_merge,
                    estimated_output_size: estimated_size,
                    priority,
                    created_at: std::time::SystemTime::now(),
                });
            }
        }

        // Sort by priority (highest first)
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }

    /// Returns the type of this strategy (SizeTiered).
    fn strategy_type(&self) -> CompactionStrategyType {
        CompactionStrategyType::SizeTiered
    }
}

impl SizeTieredStrategy {
    /// Calculates the priority of a compaction task based on file count, size, and age.
    fn calculate_priority(&self, files: &[FileMetadata], size_bucket: u64) -> u8 {
        // Higher priority for:
        // 1. More files in the group
        // 2. Smaller files (compact frequently accessed small files first)
        // 3. Older files

        let file_count_score = (files.len() * 10).min(100) as u8;
        let size_score = (100u64.saturating_sub(size_bucket / (1024 * 1024))).min(100) as u8; // Inverse of size in MB

        let avg_age = files.iter().map(|f| f.created_at.elapsed().unwrap_or_default().as_secs()).sum::<u64>() / files.len() as u64;
        let age_score = (avg_age / 3600).min(100) as u8; // Age in hours, capped at 100

        ((file_count_score as u16 + size_score as u16 + age_score as u16) / 3).min(255) as u8
    }
}

/// Implements the leveled compaction strategy, grouping files by logical levels.
pub struct LeveledStrategy {
    config: CompactionConfig,
}

impl LeveledStrategy {
    /// Creates a new LeveledStrategy with the given configuration.
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Determines the logical level of a file based on its size and configuration.
    fn get_file_level(&self, file: &FileMetadata) -> usize {
        let base_size = self.config.max_file_size as f64 / self.config.level_multiplier as f64;
        let level = if (file.size as f64) <= base_size {
            0
        } else {
            let size_ratio = file.size as f64 / base_size;
            size_ratio.log(self.config.level_multiplier as f64).ceil() as usize
        };
        level.min(self.config.max_levels - 1)
    }

    /// Groups files by their calculated level.
    fn group_files_by_level(&self, files: &[FileMetadata]) -> HashMap<usize, Vec<FileMetadata>> {
        let mut levels = HashMap::new();

        for file in files {
            let level = self.get_file_level(file);
            levels.entry(level).or_insert_with(Vec::new).push(file.clone());
        }

        levels
    }
}

impl CompactionStrategy for LeveledStrategy {
    /// Returns true if any level exceeds its allowed file count, indicating compaction is needed.
    fn should_compact(&self, files: &[FileMetadata]) -> bool {
        let levels = self.group_files_by_level(files);

        // Check if any level exceeds the capacity
        for (level, level_files) in levels {
            let max_files_for_level = self.config.level_multiplier.pow(level as u32);
            if level_files.len() > max_files_for_level {
                return true;
            }
        }

        false
    }

    /// Selects files for compaction from overfull levels and creates compaction tasks, sorted by priority.
    fn select_files_for_compaction(&self, files: &[FileMetadata]) -> Vec<CompactionTask> {
        let mut tasks = Vec::new();
        let levels = self.group_files_by_level(files);

        for (level, level_files) in levels {
            let max_files_for_level = self.config.level_multiplier.pow(level as u32);

            if level_files.len() > max_files_for_level {
                // Select oldest files for compaction
                let mut files_to_compact = level_files;
                files_to_compact.sort_by(|a, b| a.created_at.cmp(&b.created_at));

                let excess_files = files_to_compact.len() - max_files_for_level;
                let files_to_merge = files_to_compact.into_iter().take(excess_files.min(self.config.max_merge_files)).collect::<Vec<_>>();

                if files_to_merge.len() >= self.config.min_merge_files {
                    let estimated_size = files_to_merge.iter().map(|f| f.size).sum();
                    let priority = 255 - (level as u8 * 30).min(200); // Higher level = lower priority

                    tasks.push(CompactionTask {
                        id: (level as u64 * 10000) + files_to_merge.len() as u64, // Deterministic ID based on level
                        strategy_type: CompactionStrategyType::Leveled,
                        input_files: files_to_merge,
                        estimated_output_size: estimated_size,
                        priority,
                        created_at: std::time::SystemTime::now(),
                    });
                }
            }
        }

        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }

    /// Returns the type of this strategy (Leveled).
    fn strategy_type(&self) -> CompactionStrategyType {
        CompactionStrategyType::Leveled
    }
}

/// Implements the time-window compaction strategy, grouping files by creation time windows.
pub struct TimeWindowStrategy {
    config: CompactionConfig,
}

impl TimeWindowStrategy {
    /// Creates a new TimeWindowStrategy with the given configuration.
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Groups files by their creation time window.
    fn group_files_by_time_window(&self, files: &[FileMetadata]) -> HashMap<u64, Vec<FileMetadata>> {
        let mut windows = HashMap::new();
        let window_size_secs = self.config.time_window_size.as_secs();

        for file in files {
            let file_time = file.created_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

            let window_id = file_time / window_size_secs;
            windows.entry(window_id).or_insert_with(Vec::new).push(file.clone());
        }

        windows
    }
}

impl CompactionStrategy for TimeWindowStrategy {
    /// Returns true if any time window has enough files to trigger compaction.
    fn should_compact(&self, files: &[FileMetadata]) -> bool {
        let windows = self.group_files_by_time_window(files);

        // Check if any time window has enough files
        windows.values().any(|window_files| window_files.len() >= self.config.min_merge_files)
    }

    /// Selects files for compaction from each time window and creates compaction tasks, sorted by priority.
    fn select_files_for_compaction(&self, files: &[FileMetadata]) -> Vec<CompactionTask> {
        let mut tasks = Vec::new();
        let windows = self.group_files_by_time_window(files);

        for (window_id, window_files) in windows {
            if window_files.len() >= self.config.min_merge_files {
                let files_to_merge = window_files.into_iter().take(self.config.max_merge_files).collect::<Vec<_>>();

                let estimated_size = files_to_merge.iter().map(|f| f.size).sum();

                // Priority based on window age (older windows have higher priority)
                // Use the oldest file's creation time as reference to avoid timing issues
                let reference_time = files_to_merge
                    .iter()
                    .map(|f| f.created_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
                    .max()
                    .unwrap_or_default();
                let window_start = window_id * self.config.time_window_size.as_secs();
                let age_hours = (reference_time.saturating_sub(window_start)) / 3600;
                let priority = age_hours.min(255) as u8; // Older windows get higher priority values

                tasks.push(CompactionTask {
                    id: window_id * 1000 + files_to_merge.len() as u64, // Deterministic ID based on window
                    strategy_type: CompactionStrategyType::TimeWindow,
                    input_files: files_to_merge,
                    estimated_output_size: estimated_size,
                    priority,
                    created_at: std::time::SystemTime::now(),
                });
            }
        }

        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }

    /// Returns the type of this strategy (TimeWindow).
    fn strategy_type(&self) -> CompactionStrategyType {
        CompactionStrategyType::TimeWindow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    // Fixed base time for deterministic tests
    const TEST_BASE_TIME: u64 = 1_000_000; // Fixed timestamp

    fn create_test_file(id: u64, size: u64, age_secs: u64) -> FileMetadata {
        create_test_file_with_base_time(id, size, age_secs, SystemTime::UNIX_EPOCH + Duration::from_secs(TEST_BASE_TIME))
    }

    fn create_test_file_with_base_time(id: u64, size: u64, age_secs: u64, base_time: SystemTime) -> FileMetadata {
        FileMetadata {
            id,
            file_type: FileType::Data,
            version: 1,
            size,
            created_at: base_time - Duration::from_secs(age_secs),
            path: format!("test_{}.dat", id).into(),
        }
    }

    #[test]
    fn test_compaction_config_default() {
        let config = CompactionConfig::default();
        assert_eq!(config.strategy_type, CompactionStrategyType::SizeTiered);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
        assert_eq!(config.min_merge_files, 2);
        assert_eq!(config.max_merge_files, 10);
    }

    #[test]
    fn test_size_tiered_strategy() {
        let config = CompactionConfig::default();
        let strategy = SizeTieredStrategy::new(config);

        let files = vec![
            create_test_file(1, 1024, 100),
            create_test_file(2, 1024, 200),
            create_test_file(3, 1024, 300),
            create_test_file(4, 2048, 400),
        ];

        assert!(strategy.should_compact(&files));

        let tasks = strategy.select_files_for_compaction(&files);
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].strategy_type, CompactionStrategyType::SizeTiered);
    }

    #[test]
    fn test_size_tiered_grouping() {
        let config = CompactionConfig::default();
        let strategy = SizeTieredStrategy::new(config);

        let files = vec![create_test_file(1, 1024, 0), create_test_file(2, 1500, 0), create_test_file(3, 2048, 0), create_test_file(4, 4096, 0)];

        let groups = strategy.group_files_by_size(&files);

        // Files should be grouped by power-of-2 size buckets
        assert!(groups.len() > 0);

        // 1024 and 1500 should be in the same bucket (1024)
        // 2048 should be in bucket 2048
        // 4096 should be in bucket 4096
        assert!(groups.contains_key(&1024));
        assert!(groups.contains_key(&2048));
        assert!(groups.contains_key(&4096));
    }

    #[test]
    fn test_leveled_strategy() {
        let config = CompactionConfig {
            level_multiplier: 2,
            max_levels: 3,
            ..Default::default()
        };
        let strategy = LeveledStrategy::new(config);

        // Create files that would overflow level 0 (more than 2^0 = 1 file)
        let files = vec![create_test_file(1, 1024, 100), create_test_file(2, 1024, 200), create_test_file(3, 1024, 300)];

        assert!(strategy.should_compact(&files));
    }

    #[test]
    fn test_leveled_file_level_calculation() {
        let config = CompactionConfig {
            max_file_size: 100 * 1024 * 1024, // 100MB
            level_multiplier: 10,
            max_levels: 5,
            ..Default::default()
        };
        let strategy = LeveledStrategy::new(config);

        // Test different file sizes and their expected levels
        let small_file = create_test_file(1, 5 * 1024 * 1024, 0); // 5MB - should be level 0
        let medium_file = create_test_file(2, 15 * 1024 * 1024, 0); // 15MB - should be level 1
        let large_file = create_test_file(3, 150 * 1024 * 1024, 0); // 150MB - should be level 2

        assert_eq!(strategy.get_file_level(&small_file), 0);
        assert_eq!(strategy.get_file_level(&medium_file), 1);
        assert_eq!(strategy.get_file_level(&large_file), 2);
    }

    #[test]
    fn test_leveled_group_files_by_level() {
        let config = CompactionConfig {
            max_file_size: 100 * 1024 * 1024,
            level_multiplier: 10,
            ..Default::default()
        };
        let strategy = LeveledStrategy::new(config);

        let files = vec![
            create_test_file(1, 5 * 1024 * 1024, 0),   // Level 0
            create_test_file(2, 5 * 1024 * 1024, 0),   // Level 0
            create_test_file(3, 15 * 1024 * 1024, 0),  // Level 1
            create_test_file(4, 150 * 1024 * 1024, 0), // Level 2
        ];

        let groups = strategy.group_files_by_level(&files);

        assert_eq!(groups.get(&0).unwrap().len(), 2); // 2 files in level 0
        assert_eq!(groups.get(&1).unwrap().len(), 1); // 1 file in level 1
        assert_eq!(groups.get(&2).unwrap().len(), 1); // 1 file in level 2
    }

    #[test]
    fn test_leveled_compaction_task_creation() {
        let config = CompactionConfig {
            level_multiplier: 2, // Small multiplier for easier testing
            max_levels: 3,
            min_merge_files: 2,
            ..Default::default()
        };
        let strategy = LeveledStrategy::new(config);

        // Create enough files to trigger compaction in level 0
        // Level 0 can have 2^0 = 1 file max, so 3 files should trigger compaction
        let files = vec![
            create_test_file(1, 1024, 300), // Oldest
            create_test_file(2, 1024, 200),
            create_test_file(3, 1024, 100), // Newest
        ];

        let tasks = strategy.select_files_for_compaction(&files);
        assert_eq!(tasks.len(), 1);

        let task = &tasks[0];
        assert_eq!(task.strategy_type, CompactionStrategyType::Leveled);
        assert_eq!(task.input_files.len(), 2); // Should compact 2 oldest files
        assert_eq!(task.input_files[0].id, 1); // Oldest file first
        assert_eq!(task.input_files[1].id, 2); // Second oldest
    }

    #[test]
    fn test_time_window_strategy() {
        let config = CompactionConfig {
            time_window_size: Duration::from_secs(3600), // 1 hour windows
            min_merge_files: 2,
            ..Default::default()
        };
        let strategy = TimeWindowStrategy::new(config);

        // Use fixed base time to avoid timing issues
        let base_time = SystemTime::UNIX_EPOCH + Duration::from_secs(TEST_BASE_TIME);
        let files = vec![
            create_test_file_with_base_time(1, 1024, 100, base_time),  // Same window
            create_test_file_with_base_time(2, 1024, 200, base_time),  // Same window
            create_test_file_with_base_time(3, 1024, 7200, base_time), // Different window (2 hours ago)
        ];

        assert!(strategy.should_compact(&files));

        let tasks = strategy.select_files_for_compaction(&files);
        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].strategy_type, CompactionStrategyType::TimeWindow);
    }

    #[test]
    fn test_time_window_grouping() {
        let config = CompactionConfig {
            time_window_size: Duration::from_secs(3600), // 1 hour
            ..Default::default()
        };
        let strategy = TimeWindowStrategy::new(config);

        // Use a fixed base time to avoid timing race conditions
        let base_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1000000);

        let files = vec![
            FileMetadata {
                id: 1,
                file_type: FileType::Data,
                version: 1,
                size: 1024,
                created_at: base_time - Duration::from_secs(100), // Window 1
                path: "test_1.dat".into(),
            },
            FileMetadata {
                id: 2,
                file_type: FileType::Data,
                version: 1,
                size: 1024,
                created_at: base_time - Duration::from_secs(200), // Window 1
                path: "test_2.dat".into(),
            },
            FileMetadata {
                id: 3,
                file_type: FileType::Data,
                version: 1,
                size: 1024,
                created_at: base_time - Duration::from_secs(4000), // Window 2 (different hour)
                path: "test_3.dat".into(),
            },
            FileMetadata {
                id: 4,
                file_type: FileType::Data,
                version: 1,
                size: 1024,
                created_at: base_time - Duration::from_secs(4100), // Window 2
                path: "test_4.dat".into(),
            },
        ];

        let windows = strategy.group_files_by_time_window(&files);
        assert_eq!(windows.len(), 2); // Should have 2 time windows

        // Each window should have 2 files
        for (_, window_files) in windows {
            assert_eq!(window_files.len(), 2);
        }
    }

    #[test]
    fn test_time_window_priority_calculation() {
        let config = CompactionConfig {
            time_window_size: Duration::from_secs(3600),
            min_merge_files: 2,
            ..Default::default()
        };
        let strategy = TimeWindowStrategy::new(config);

        // Use fixed base time for deterministic results
        let base_time = SystemTime::UNIX_EPOCH + Duration::from_secs(TEST_BASE_TIME);

        // Create files with different ages - older files should have higher priority
        let old_files = vec![
            create_test_file_with_base_time(1, 1024, 7200, base_time), // 2 hours old
            create_test_file_with_base_time(2, 1024, 7300, base_time), // 2 hours old
        ];

        let new_files = vec![
            create_test_file_with_base_time(3, 1024, 100, base_time), // Recent
            create_test_file_with_base_time(4, 1024, 200, base_time), // Recent
        ];

        let mut all_files = old_files;
        all_files.extend(new_files);

        let tasks = strategy.select_files_for_compaction(&all_files);
        assert_eq!(tasks.len(), 2);

        // Older window should have higher priority (higher numeric value)
        assert!(tasks[0].priority >= tasks[1].priority);
    }

    #[test]
    fn test_size_tiered_priority_calculation() {
        let config = CompactionConfig::default();
        let strategy = SizeTieredStrategy::new(config);

        // Test priority calculation with different scenarios
        let many_small_old_files = vec![
            create_test_file(1, 1024, 7200),
            create_test_file(2, 1024, 7200),
            create_test_file(3, 1024, 7200),
            create_test_file(4, 1024, 7200),
        ];

        let few_large_new_files = vec![create_test_file(5, 50 * 1024 * 1024, 100), create_test_file(6, 50 * 1024 * 1024, 100)];

        let priority1 = strategy.calculate_priority(&many_small_old_files, 1024);
        let priority2 = strategy.calculate_priority(&few_large_new_files, 50 * 1024 * 1024);

        // Many small old files should have higher priority than few large new files
        assert!(priority1 > priority2);
    }

    #[test]
    fn test_compaction_task_sorting_by_priority() {
        let config = CompactionConfig::default();
        let strategy = SizeTieredStrategy::new(config);

        // Create files that will generate multiple tasks with different priorities
        let files = vec![
            // High priority group (many small files)
            create_test_file(1, 1024, 3600),
            create_test_file(2, 1024, 3600),
            create_test_file(3, 1024, 3600),
            // Lower priority group (fewer larger files)
            create_test_file(4, 10 * 1024 * 1024, 100),
            create_test_file(5, 10 * 1024 * 1024, 100),
        ];

        let tasks = strategy.select_files_for_compaction(&files);

        // Tasks should be sorted by priority (highest first)
        for i in 1..tasks.len() {
            assert!(tasks[i - 1].priority >= tasks[i].priority);
        }
    }

    #[test]
    fn test_empty_files_handling() {
        let config = CompactionConfig::default();
        let size_strategy = SizeTieredStrategy::new(config.clone());
        let leveled_strategy = LeveledStrategy::new(config.clone());
        let time_strategy = TimeWindowStrategy::new(config);

        let empty_files: Vec<FileMetadata> = vec![];

        // All strategies should handle empty file lists gracefully
        assert!(!size_strategy.should_compact(&empty_files));
        assert!(!leveled_strategy.should_compact(&empty_files));
        assert!(!time_strategy.should_compact(&empty_files));

        assert!(size_strategy.select_files_for_compaction(&empty_files).is_empty());
        assert!(leveled_strategy.select_files_for_compaction(&empty_files).is_empty());
        assert!(time_strategy.select_files_for_compaction(&empty_files).is_empty());
    }

    #[test]
    fn test_insufficient_files_for_compaction() {
        let config = CompactionConfig {
            min_merge_files: 3,
            ..Default::default()
        };

        let size_strategy = SizeTieredStrategy::new(config.clone());
        let leveled_strategy = LeveledStrategy::new(config.clone());
        let time_strategy = TimeWindowStrategy::new(config);

        // Only 2 files, but minimum is 3
        let files = vec![create_test_file(1, 1024, 100), create_test_file(2, 1024, 200)];

        assert!(!size_strategy.should_compact(&files));
        assert!(!time_strategy.should_compact(&files));

        assert!(size_strategy.select_files_for_compaction(&files).is_empty());
        assert!(time_strategy.select_files_for_compaction(&files).is_empty());
    }

    #[test]
    fn test_max_merge_files_limit() {
        let config = CompactionConfig {
            min_merge_files: 2,
            max_merge_files: 3,
            ..Default::default()
        };
        let strategy = SizeTieredStrategy::new(config);

        // Create 5 files of similar size
        let files = vec![
            create_test_file(1, 1024, 500),
            create_test_file(2, 1024, 400),
            create_test_file(3, 1024, 300),
            create_test_file(4, 1024, 200),
            create_test_file(5, 1024, 100),
        ];

        let tasks = strategy.select_files_for_compaction(&files);
        assert!(!tasks.is_empty());

        // Should respect max_merge_files limit
        for task in tasks {
            assert!(task.input_files.len() <= 3);
            assert!(task.input_files.len() >= 2);
        }
    }

    #[test]
    fn test_estimated_output_size_calculation() {
        let config = CompactionConfig::default();
        let strategy = SizeTieredStrategy::new(config);

        // Aynı bucket'a düşecek dosya boyutları
        let files = vec![create_test_file(1, 1024, 100), create_test_file(2, 1024, 200), create_test_file(3, 1024, 300)];

        let tasks = strategy.select_files_for_compaction(&files);
        assert!(!tasks.is_empty());
        let task = &tasks[0];
        let expected_size: u64 = task.input_files.iter().map(|f| f.size).sum();
        assert_eq!(task.estimated_output_size, expected_size);
    }

    #[test]
    fn test_strategy_type_consistency() {
        let config = CompactionConfig::default();

        let size_strategy = SizeTieredStrategy::new(config.clone());
        let leveled_strategy = LeveledStrategy::new(config.clone());
        let time_strategy = TimeWindowStrategy::new(config);

        assert_eq!(size_strategy.strategy_type(), CompactionStrategyType::SizeTiered);
        assert_eq!(leveled_strategy.strategy_type(), CompactionStrategyType::Leveled);
        assert_eq!(time_strategy.strategy_type(), CompactionStrategyType::TimeWindow);

        // Test that tasks created by each strategy have the correct type
        let files = vec![create_test_file(1, 1024, 100), create_test_file(2, 1024, 200), create_test_file(3, 1024, 300)];

        if let Some(task) = size_strategy.select_files_for_compaction(&files).first() {
            assert_eq!(task.strategy_type, CompactionStrategyType::SizeTiered);
        }

        if let Some(task) = time_strategy.select_files_for_compaction(&files).first() {
            assert_eq!(task.strategy_type, CompactionStrategyType::TimeWindow);
        }
    }

    #[test]
    fn test_compaction_task_creation_time() {
        let config = CompactionConfig::default();
        let strategy = SizeTieredStrategy::new(config);

        let files = vec![create_test_file(1, 1024, 100), create_test_file(2, 1024, 200)];

        let before = SystemTime::now();
        let tasks = strategy.select_files_for_compaction(&files);
        let after = SystemTime::now();

        if let Some(task) = tasks.first() {
            assert!(task.created_at >= before);
            assert!(task.created_at <= after);
        }
    }

    #[test]
    fn test_zero_size_files() {
        let config = CompactionConfig::default();
        let strategy = SizeTieredStrategy::new(config);

        let files = vec![
            create_test_file(1, 0, 100), // Zero size file
            create_test_file(2, 0, 200), // Zero size file
            create_test_file(3, 1024, 300),
        ];

        // Should handle zero-size files without panicking
        let groups = strategy.group_files_by_size(&files);
        assert!(groups.contains_key(&0)); // Zero-size files should be in bucket 0

        let tasks = strategy.select_files_for_compaction(&files);
        // Should create tasks even with zero-size files
        assert!(!tasks.is_empty());
    }

    #[test]
    fn test_leveled_strategy_max_levels_respect() {
        let config = CompactionConfig {
            max_levels: 2, // Only 2 levels allowed
            level_multiplier: 10,
            max_file_size: 100 * 1024 * 1024,
            ..Default::default()
        };
        let strategy = LeveledStrategy::new(config);

        // Create a very large file that would normally be at a high level
        let huge_file = create_test_file(1, 1000 * 1024 * 1024, 0); // 1GB

        // Should be capped at max_levels - 1
        assert_eq!(strategy.get_file_level(&huge_file), 1); // Level 1 (max_levels - 1)
    }
}
