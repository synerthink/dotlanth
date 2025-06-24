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

use super::strategy::{CompactionConfig, CompactionStrategy, CompactionStrategyType, CompactionTask};
use super::strategy::{LeveledStrategy, SizeTieredStrategy, TimeWindowStrategy};
use crate::fs::FileMetadata;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;

/// Compaction manager configuration
#[derive(Debug, Clone)]
pub struct CompactionManagerConfig {
    /// Maximum number of concurrent compaction threads
    pub max_concurrent_compactions: usize,
    /// How often to check for compaction opportunities (in seconds)
    pub check_interval_secs: u64,
    /// Maximum CPU usage percentage (0.0 - 1.0)
    pub max_cpu_usage: f64,
    /// Maximum memory usage for compaction operations (in bytes)
    pub max_memory_usage: u64,
    /// Compaction aggressiveness level (0-10, higher = more aggressive)
    pub aggressiveness_level: u8,
    /// Whether to enable background compaction
    pub enable_background_compaction: bool,
    /// Minimum time between compactions for the same file group
    pub min_compaction_interval: Duration,
    /// Maximum time a compaction task can run before being cancelled
    pub max_compaction_duration: Duration,
    /// I/O bandwidth limit for compaction operations (bytes per second)
    pub io_bandwidth_limit: u64,
}

impl Default for CompactionManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_compactions: 2,
            check_interval_secs: 30,
            max_cpu_usage: 0.3,                  // 30% CPU usage
            max_memory_usage: 512 * 1024 * 1024, // 512MB
            aggressiveness_level: 5,
            enable_background_compaction: true,
            min_compaction_interval: Duration::from_secs(300),  // 5 minutes
            max_compaction_duration: Duration::from_secs(3600), // 1 hour
            io_bandwidth_limit: 50 * 1024 * 1024,               // 50MB/s
        }
    }
}

/// Compaction statistics
#[derive(Debug, Clone, Default)]
pub struct CompactionStats {
    pub total_compactions: u64,
    pub successful_compactions: u64,
    pub failed_compactions: u64,
    pub bytes_compacted: u64,
    pub space_reclaimed: u64,
    pub total_compaction_time: Duration,
    pub average_compaction_time: Duration,
    pub last_compaction_time: Option<SystemTime>,
    pub active_compactions: u64,
}

/// Compaction execution result
#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub task_id: u64,
    pub success: bool,
    pub input_files: Vec<FileMetadata>,
    pub output_files: Vec<FileMetadata>,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub space_reclaimed: u64,
    pub duration: Duration,
    pub error_message: Option<String>,
}

/// Resource monitor for tracking system resources
#[derive(Debug)]
pub struct ResourceMonitor {
    cpu_usage: Arc<AtomicU64>, // Stored as percentage * 100
    memory_usage: Arc<AtomicU64>,
    io_usage: Arc<AtomicU64>,
    last_update: Arc<Mutex<Instant>>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            cpu_usage: Arc::new(AtomicU64::new(0)),
            memory_usage: Arc::new(AtomicU64::new(0)),
            io_usage: Arc::new(AtomicU64::new(0)),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn update_cpu_usage(&self, usage: f64) {
        self.cpu_usage.store((usage * 100.0) as u64, Ordering::Relaxed);
        *self.last_update.lock().unwrap() = Instant::now();
    }

    pub fn update_memory_usage(&self, usage: u64) {
        self.memory_usage.store(usage, Ordering::Relaxed);
    }

    pub fn update_io_usage(&self, usage: u64) {
        self.io_usage.store(usage, Ordering::Relaxed);
    }

    pub fn get_cpu_usage(&self) -> f64 {
        self.cpu_usage.load(Ordering::Relaxed) as f64 / 100.0
    }

    pub fn get_memory_usage(&self) -> u64 {
        self.memory_usage.load(Ordering::Relaxed)
    }

    pub fn get_io_usage(&self) -> u64 {
        self.io_usage.load(Ordering::Relaxed)
    }

    pub fn can_proceed(&self, config: &CompactionManagerConfig) -> bool {
        let cpu_ok = self.get_cpu_usage() < config.max_cpu_usage;
        let memory_ok = self.get_memory_usage() < config.max_memory_usage;

        cpu_ok && memory_ok
    }
}

/// Background compaction manager
pub struct CompactionManager {
    config: CompactionManagerConfig,
    strategies: HashMap<CompactionStrategyType, Box<dyn CompactionStrategy + Send + Sync>>,
    active_tasks: Arc<RwLock<HashMap<u64, CompactionTask>>>,
    completed_tasks: Arc<RwLock<VecDeque<CompactionResult>>>,
    stats: Arc<RwLock<CompactionStats>>,
    resource_monitor: Arc<ResourceMonitor>,
    task_queue: Arc<Mutex<VecDeque<CompactionTask>>>,
    shutdown_signal: Arc<AtomicBool>,
    worker_handles: Vec<JoinHandle<()>>,
    scheduler_handle: Option<JoinHandle<()>>,
    task_sender: mpsc::Sender<CompactionTask>,
    task_receiver: Arc<Mutex<mpsc::Receiver<CompactionTask>>>,
    last_compaction_times: Arc<RwLock<HashMap<String, SystemTime>>>,
}

impl CompactionManager {
    /// Creates a new CompactionManager with the given configuration and compaction strategies.
    /// Initializes all background structures and resource monitors.
    pub fn new(config: CompactionManagerConfig, compaction_configs: HashMap<CompactionStrategyType, CompactionConfig>) -> Self {
        let mut strategies: HashMap<CompactionStrategyType, Box<dyn CompactionStrategy + Send + Sync>> = HashMap::new();

        // Initialize strategies based on provided configs
        for (strategy_type, strategy_config) in compaction_configs {
            let strategy: Box<dyn CompactionStrategy + Send + Sync> = match strategy_type {
                CompactionStrategyType::SizeTiered => Box::new(SizeTieredStrategy::new(strategy_config)),
                CompactionStrategyType::Leveled => Box::new(LeveledStrategy::new(strategy_config)),
                CompactionStrategyType::TimeWindow => Box::new(TimeWindowStrategy::new(strategy_config)),
                CompactionStrategyType::Custom => {
                    // For custom strategies, we'd need a factory pattern
                    // For now, default to size-tiered
                    Box::new(SizeTieredStrategy::new(strategy_config))
                }
            };
            strategies.insert(strategy_type, strategy);
        }

        let (task_sender, task_receiver) = mpsc::channel(1000);

        Self {
            config,
            strategies,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            completed_tasks: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(CompactionStats::default())),
            resource_monitor: Arc::new(ResourceMonitor::new()),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            worker_handles: Vec::new(),
            scheduler_handle: None,
            task_sender,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
            last_compaction_times: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Starts the background compaction manager, including worker and scheduler threads.
    /// Returns an error if background compaction is disabled or thread creation fails.
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enable_background_compaction {
            return Ok(());
        }

        // Start worker threads
        for i in 0..self.config.max_concurrent_compactions {
            let worker_handle = self.start_worker_thread(i)?;
            self.worker_handles.push(worker_handle);
        }

        // Start scheduler thread
        let scheduler_handle = self.start_scheduler_thread()?;
        self.scheduler_handle = Some(scheduler_handle);

        Ok(())
    }

    /// Stops the compaction manager, signaling all threads to shut down and waiting for their completion.
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Signal shutdown
        self.shutdown_signal.store(true, Ordering::Relaxed);

        // Wait for scheduler to finish
        if let Some(handle) = self.scheduler_handle.take() {
            handle.join().map_err(|_| "Failed to join scheduler thread")?;
        }

        // Wait for all workers to finish
        for handle in self.worker_handles.drain(..) {
            handle.join().map_err(|_| "Failed to join worker thread")?;
        }

        Ok(())
    }

    /// Checks the provided files and schedules compaction tasks if needed, according to all enabled strategies.
    /// Returns the number of tasks scheduled.
    pub fn check_and_schedule_compaction(&self, files: &[FileMetadata]) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let mut total_scheduled = 0;

        for (strategy_type, strategy) in &self.strategies {
            if strategy.should_compact(files) {
                let tasks = strategy.select_files_for_compaction(files);

                for task in tasks {
                    if self.should_schedule_task(&task)? {
                        self.schedule_task(task)?;
                        total_scheduled += 1;
                    }
                }
            }
        }

        Ok(total_scheduled)
    }

    /// Schedules a compaction task for execution by worker threads.
    /// Returns an error if resource limits are exceeded or the task cannot be queued.
    pub fn schedule_task(&self, task: CompactionTask) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check if we should throttle based on resource usage
        if !self.resource_monitor.can_proceed(&self.config) {
            return Err("Resource limits exceeded, compaction throttled".into());
        }

        // Add to active tasks
        self.active_tasks.write().unwrap().insert(task.id, task.clone());

        // Send to worker threads
        self.task_sender.try_send(task).map_err(|e| format!("Failed to schedule task: {}", e))?;

        Ok(())
    }

    /// Executes a compaction task synchronously, updating statistics and task tracking.
    /// Returns the result of the compaction operation.
    pub fn execute_compaction(&self, task: CompactionTask) -> CompactionResult {
        let start_time = Instant::now();
        let mut stats = self.stats.write().unwrap();
        stats.total_compactions += 1;
        stats.active_compactions += 1;
        drop(stats);

        // Simulate compaction execution (in real implementation, this would do actual file operations)
        let result = self.perform_compaction_operation(&task);

        // Update statistics
        let duration = start_time.elapsed();
        let mut stats = self.stats.write().unwrap();
        stats.active_compactions -= 1;
        stats.total_compaction_time += duration;
        stats.average_compaction_time = Duration::from_secs(stats.total_compaction_time.as_secs() / stats.total_compactions);
        stats.last_compaction_time = Some(SystemTime::now());

        if result.success {
            stats.successful_compactions += 1;
            stats.bytes_compacted += result.bytes_read;
            stats.space_reclaimed += result.space_reclaimed;
        } else {
            stats.failed_compactions += 1;
        }

        // Remove from active tasks and add to completed tasks
        self.active_tasks.write().unwrap().remove(&task.id);

        let mut completed = self.completed_tasks.write().unwrap();
        completed.push_back(result.clone());

        // Keep only last 1000 completed tasks
        if completed.len() > 1000 {
            completed.pop_front();
        }

        // Update last compaction time for this file group
        let group_key = self.get_file_group_key(&task.input_files);
        self.last_compaction_times.write().unwrap().insert(group_key, SystemTime::now());

        result
    }

    /// Returns a snapshot of current compaction statistics.
    pub fn get_stats(&self) -> CompactionStats {
        self.stats.read().unwrap().clone()
    }

    /// Returns a list of currently active compaction tasks.
    pub fn get_active_tasks(&self) -> Vec<CompactionTask> {
        self.active_tasks.read().unwrap().values().cloned().collect()
    }

    /// Returns the most recent completed compaction tasks, up to the specified limit.
    pub fn get_recent_completed_tasks(&self, limit: usize) -> Vec<CompactionResult> {
        let completed = self.completed_tasks.read().unwrap();
        completed.iter().rev().take(limit).cloned().collect()
    }

    /// Updates the resource monitor with the latest CPU, memory, and I/O usage values.
    pub fn update_resource_usage(&self, cpu_usage: f64, memory_usage: u64, io_usage: u64) {
        self.resource_monitor.update_cpu_usage(cpu_usage);
        self.resource_monitor.update_memory_usage(memory_usage);
        self.resource_monitor.update_io_usage(io_usage);
    }

    /// Returns true if compaction is currently throttled due to resource limits.
    pub fn is_throttled(&self) -> bool {
        !self.resource_monitor.can_proceed(&self.config)
    }

    /// Forces a compaction check and scheduling for the provided files.
    pub fn trigger_compaction_check(&self, files: &[FileMetadata]) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        self.check_and_schedule_compaction(files)
    }

    /// Starts a worker thread that processes compaction tasks from the queue until shutdown is signaled.
    fn start_worker_thread(&self, worker_id: usize) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        let shutdown_signal = Arc::clone(&self.shutdown_signal);
        let task_receiver = Arc::clone(&self.task_receiver);
        let manager_clone = self.create_manager_handle();

        let handle = thread::Builder::new().name(format!("compaction-worker-{}", worker_id)).spawn(move || {
            while !shutdown_signal.load(Ordering::Relaxed) {
                if let Ok(mut receiver) = task_receiver.try_lock() {
                    match receiver.try_recv() {
                        Ok(task) => {
                            let result = manager_clone.execute_compaction(task);
                            // Log result or send to monitoring system
                            if !result.success {
                                eprintln!("Compaction task {} failed: {:?}", result.task_id, result.error_message);
                            }
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {
                            // No tasks available, sleep briefly
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            break;
                        }
                    }
                } else {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        })?;

        Ok(handle)
    }

    /// Starts the scheduler thread, which periodically checks for compaction opportunities.
    fn start_scheduler_thread(&self) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        let shutdown_signal = Arc::clone(&self.shutdown_signal);
        let check_interval = Duration::from_secs(self.config.check_interval_secs);

        let handle = thread::Builder::new().name("compaction-scheduler".to_string()).spawn(move || {
            while !shutdown_signal.load(Ordering::Relaxed) {
                // In a real implementation, this would:
                // 1. Query the file manager for current files
                // 2. Check if compaction is needed
                // 3. Schedule tasks accordingly

                // For now, we just sleep
                thread::sleep(check_interval);
            }
        })?;

        Ok(handle)
    }

    /// Determines if a compaction task should be scheduled, based on resource limits, recent compactions, and aggressiveness.
    fn should_schedule_task(&self, task: &CompactionTask) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Check resource limits
        if !self.resource_monitor.can_proceed(&self.config) {
            return Ok(false);
        }

        // Check if we recently compacted this file group
        let group_key = self.get_file_group_key(&task.input_files);
        if let Some(last_time) = self.last_compaction_times.read().unwrap().get(&group_key) {
            if last_time.elapsed().unwrap_or_default() < self.config.min_compaction_interval {
                return Ok(false);
            }
        }

        // Adjust based on aggressiveness level
        let aggressiveness_factor = self.config.aggressiveness_level as f64 / 10.0;
        let priority_threshold = 255.0 * (1.0 - aggressiveness_factor);

        Ok(task.priority as f64 >= priority_threshold)
    }

    /// Simulates the actual compaction operation: merges input files, creates output, and estimates space reclaimed.
    /// In a real implementation, this would perform file I/O and data merging.
    fn perform_compaction_operation(&self, task: &CompactionTask) -> CompactionResult {
        // This is a simplified simulation of compaction
        // In a real implementation, this would:
        // 1. Read input files
        // 2. Merge/compact the data
        // 3. Write output files
        // 4. Update file metadata
        // 5. Clean up old files

        let start_time = Instant::now();

        // Simulate I/O operations with bandwidth limiting
        let total_bytes = task.input_files.iter().map(|f| f.size).sum::<u64>();
        let estimated_duration = Duration::from_millis(((total_bytes as f64 / self.config.io_bandwidth_limit as f64) * 1000.0) as u64);

        // Simulate work
        thread::sleep(estimated_duration.min(Duration::from_millis(100)));

        let duration = start_time.elapsed();
        let success = duration < self.config.max_compaction_duration;

        let space_reclaimed = if success {
            // Estimate space reclaimed (typically 10-30% for compaction)
            (total_bytes as f64 * 0.2) as u64
        } else {
            0
        };

        CompactionResult {
            task_id: task.id,
            success,
            input_files: task.input_files.clone(),
            output_files: if success {
                // Simulate creating fewer, larger output files
                vec![FileMetadata {
                    id: task.id,
                    file_type: crate::fs::FileType::Data,
                    version: 1,
                    size: total_bytes - space_reclaimed,
                    created_at: SystemTime::now(),
                    path: format!("compacted_{}.dat", task.id).into(),
                }]
            } else {
                vec![]
            },
            bytes_read: total_bytes,
            bytes_written: if success { total_bytes - space_reclaimed } else { 0 },
            space_reclaimed,
            duration,
            error_message: if success { None } else { Some("Compaction timeout".to_string()) },
        }
    }

    /// Generates a unique key for a group of files, used to track compaction intervals and avoid redundant work.
    fn get_file_group_key(&self, files: &[FileMetadata]) -> String {
        // Create a key to identify this group of files
        let mut ids: Vec<u64> = files.iter().map(|f| f.id).collect();
        ids.sort();
        ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join("-")
    }

    /// Creates a lightweight handle to the compaction manager for use in worker threads.
    fn create_manager_handle(&self) -> CompactionManagerHandle {
        CompactionManagerHandle {
            active_tasks: Arc::clone(&self.active_tasks),
            completed_tasks: Arc::clone(&self.completed_tasks),
            stats: Arc::clone(&self.stats),
            resource_monitor: Arc::clone(&self.resource_monitor),
            config: self.config.clone(),
            last_compaction_times: Arc::clone(&self.last_compaction_times),
        }
    }
}

/// A lightweight handle to the compaction manager for use in worker threads
struct CompactionManagerHandle {
    active_tasks: Arc<RwLock<HashMap<u64, CompactionTask>>>,
    completed_tasks: Arc<RwLock<VecDeque<CompactionResult>>>,
    stats: Arc<RwLock<CompactionStats>>,
    resource_monitor: Arc<ResourceMonitor>,
    config: CompactionManagerConfig,
    last_compaction_times: Arc<RwLock<HashMap<String, SystemTime>>>,
}

impl CompactionManagerHandle {
    /// Executes a compaction task and updates statistics and task tracking (worker thread context).
    fn execute_compaction(&self, task: CompactionTask) -> CompactionResult {
        let start_time = Instant::now();
        let mut stats = self.stats.write().unwrap();
        stats.total_compactions += 1;
        stats.active_compactions += 1;
        drop(stats);

        // Perform the actual compaction work
        let result = self.perform_compaction_operation(&task);

        // Update statistics
        let duration = start_time.elapsed();
        let mut stats = self.stats.write().unwrap();
        stats.active_compactions -= 1;
        stats.total_compaction_time += duration;
        stats.average_compaction_time = Duration::from_secs(stats.total_compaction_time.as_secs() / stats.total_compactions);
        stats.last_compaction_time = Some(SystemTime::now());

        if result.success {
            stats.successful_compactions += 1;
            stats.bytes_compacted += result.bytes_read;
            stats.space_reclaimed += result.space_reclaimed;
        } else {
            stats.failed_compactions += 1;
        }

        // Remove from active tasks and add to completed tasks
        self.active_tasks.write().unwrap().remove(&task.id);

        let mut completed = self.completed_tasks.write().unwrap();
        completed.push_back(result.clone());

        if completed.len() > 1000 {
            completed.pop_front();
        }

        // Update last compaction time
        let group_key = self.get_file_group_key(&task.input_files);
        self.last_compaction_times.write().unwrap().insert(group_key, SystemTime::now());

        result
    }

    /// Simulates the compaction operation for the handle (worker thread context).
    fn perform_compaction_operation(&self, task: &CompactionTask) -> CompactionResult {
        let start_time = Instant::now();

        let total_bytes = task.input_files.iter().map(|f| f.size).sum::<u64>();
        let estimated_duration = Duration::from_millis(((total_bytes as f64 / self.config.io_bandwidth_limit as f64) * 1000.0) as u64);

        thread::sleep(estimated_duration.min(Duration::from_millis(100)));

        let duration = start_time.elapsed();
        let success = duration < self.config.max_compaction_duration;

        let space_reclaimed = if success { (total_bytes as f64 * 0.2) as u64 } else { 0 };

        CompactionResult {
            task_id: task.id,
            success,
            input_files: task.input_files.clone(),
            output_files: if success {
                vec![FileMetadata {
                    id: task.id,
                    file_type: crate::fs::FileType::Data,
                    version: 1,
                    size: total_bytes - space_reclaimed,
                    created_at: SystemTime::now(),
                    path: format!("compacted_{}.dat", task.id).into(),
                }]
            } else {
                vec![]
            },
            bytes_read: total_bytes,
            bytes_written: if success { total_bytes - space_reclaimed } else { 0 },
            space_reclaimed,
            duration,
            error_message: if success { None } else { Some("Compaction timeout".to_string()) },
        }
    }

    /// Generates a unique key for a group of files (worker thread context).
    fn get_file_group_key(&self, files: &[FileMetadata]) -> String {
        let mut ids: Vec<u64> = files.iter().map(|f| f.id).collect();
        ids.sort();
        ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join("-")
    }
}

/// Ensures all background threads are stopped and resources are cleaned up when the manager is dropped.
impl Drop for CompactionManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FileType;
    use std::thread;
    use std::time::{Duration, SystemTime};

    fn create_test_config() -> CompactionManagerConfig {
        CompactionManagerConfig {
            max_concurrent_compactions: 1,
            check_interval_secs: 1,
            max_cpu_usage: 0.8,
            max_memory_usage: 1024 * 1024 * 1024,
            aggressiveness_level: 5,
            enable_background_compaction: false, // Disable for testing
            min_compaction_interval: Duration::from_secs(1),
            max_compaction_duration: Duration::from_secs(10),
            io_bandwidth_limit: 100 * 1024 * 1024,
        }
    }

    fn create_test_file(id: u64, size: u64) -> FileMetadata {
        FileMetadata {
            id,
            file_type: FileType::Data,
            version: 1,
            size,
            created_at: SystemTime::now(),
            path: format!("test_{}.dat", id).into(),
        }
    }

    fn create_test_task(id: u64, files: Vec<FileMetadata>) -> CompactionTask {
        CompactionTask {
            id,
            strategy_type: CompactionStrategyType::SizeTiered,
            input_files: files,
            estimated_output_size: 1024,
            priority: 128,
            created_at: SystemTime::now(),
        }
    }

    #[test]
    fn test_compaction_manager_creation() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);
        assert_eq!(manager.strategies.len(), 1);
        assert!(manager.strategies.contains_key(&CompactionStrategyType::SizeTiered));
    }

    #[test]
    fn test_resource_monitor() {
        let monitor = ResourceMonitor::new();

        monitor.update_cpu_usage(0.5);
        monitor.update_memory_usage(1024);
        monitor.update_io_usage(2048);

        assert_eq!(monitor.get_cpu_usage(), 0.5);
        assert_eq!(monitor.get_memory_usage(), 1024);
        assert_eq!(monitor.get_io_usage(), 2048);

        let config = create_test_config();
        assert!(monitor.can_proceed(&config));

        monitor.update_cpu_usage(0.9); // Exceed limit
        assert!(!monitor.can_proceed(&config));
    }

    #[test]
    fn test_task_scheduling() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        let files = vec![create_test_file(1, 1024)];
        let task = create_test_task(1, files);

        // Should be able to schedule task
        assert!(manager.schedule_task(task).is_ok());

        // Task should be in active tasks
        assert_eq!(manager.get_active_tasks().len(), 1);
    }

    #[test]
    fn test_compaction_execution() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        let files = vec![create_test_file(1, 1024), create_test_file(2, 2048)];
        let task = create_test_task(1, files);

        let result = manager.execute_compaction(task);

        assert_eq!(result.task_id, 1);
        assert!(result.success);
        assert_eq!(result.bytes_read, 3072); // 1024 + 2048
        assert!(result.space_reclaimed > 0);

        // Check statistics were updated
        let stats = manager.get_stats();
        assert_eq!(stats.total_compactions, 1);
        assert_eq!(stats.successful_compactions, 1);
        assert!(stats.bytes_compacted > 0);
    }

    #[test]
    fn test_compaction_stats() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        let initial_stats = manager.get_stats();
        assert_eq!(initial_stats.total_compactions, 0);

        // Execute a compaction
        let files = vec![create_test_file(1, 1024)];
        let task = create_test_task(1, files);
        let _result = manager.execute_compaction(task);

        let updated_stats = manager.get_stats();
        assert_eq!(updated_stats.total_compactions, 1);
        assert_eq!(updated_stats.successful_compactions, 1);
        assert!(updated_stats.last_compaction_time.is_some());
    }

    #[test]
    fn test_throttling() {
        let mut config = create_test_config();
        config.max_cpu_usage = 0.3; // Lower threshold for testing

        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        // Initially should not be throttled
        assert!(!manager.is_throttled());

        // Update resource usage to exceed CPU limit
        manager.update_resource_usage(0.5, 0, 0);
        assert!(manager.is_throttled());

        // Try to schedule a task when throttled
        let files = vec![create_test_file(1, 1024)];
        let task = create_test_task(1, files);

        let result = manager.schedule_task(task);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Resource limits exceeded"));
    }

    #[test]
    fn test_multiple_strategies() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());
        strategies.insert(CompactionStrategyType::Leveled, CompactionConfig::default());
        strategies.insert(CompactionStrategyType::TimeWindow, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);
        assert_eq!(manager.strategies.len(), 3);
        assert!(manager.strategies.contains_key(&CompactionStrategyType::SizeTiered));
        assert!(manager.strategies.contains_key(&CompactionStrategyType::Leveled));
        assert!(manager.strategies.contains_key(&CompactionStrategyType::TimeWindow));
    }

    #[test]
    fn test_compaction_result_tracking() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        // Execute multiple compactions
        for i in 1..=5 {
            let files = vec![create_test_file(i, 1024 * i)];
            let task = create_test_task(i, files);
            let _result = manager.execute_compaction(task);
        }

        // Check completed tasks tracking
        let completed_tasks = manager.get_recent_completed_tasks(3);
        assert_eq!(completed_tasks.len(), 3);

        // Should be in reverse order (most recent first)
        assert_eq!(completed_tasks[0].task_id, 5);
        assert_eq!(completed_tasks[1].task_id, 4);
        assert_eq!(completed_tasks[2].task_id, 3);

        // Check all completed tasks
        let all_completed = manager.get_recent_completed_tasks(10);
        assert_eq!(all_completed.len(), 5);
    }

    #[test]
    fn test_config_defaults() {
        let config = CompactionManagerConfig::default();

        assert_eq!(config.max_concurrent_compactions, 2);
        assert_eq!(config.check_interval_secs, 30);
        assert_eq!(config.max_cpu_usage, 0.3);
        assert_eq!(config.max_memory_usage, 512 * 1024 * 1024);
        assert_eq!(config.aggressiveness_level, 5);
        assert!(config.enable_background_compaction);
        assert_eq!(config.min_compaction_interval, Duration::from_secs(300));
        assert_eq!(config.max_compaction_duration, Duration::from_secs(3600));
        assert_eq!(config.io_bandwidth_limit, 50 * 1024 * 1024);
    }

    #[test]
    fn test_file_group_key_generation() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        let files1 = vec![create_test_file(1, 1024), create_test_file(3, 2048)];
        let files2 = vec![create_test_file(3, 2048), create_test_file(1, 1024)]; // Same files, different order

        let key1 = manager.get_file_group_key(&files1);
        let key2 = manager.get_file_group_key(&files2);

        // Keys should be the same regardless of order
        assert_eq!(key1, key2);
        assert_eq!(key1, "1-3");
    }

    #[test]
    fn test_resource_monitor_updates() {
        let monitor = ResourceMonitor::new();

        // Test initial values
        assert_eq!(monitor.get_cpu_usage(), 0.0);
        assert_eq!(monitor.get_memory_usage(), 0);
        assert_eq!(monitor.get_io_usage(), 0);

        // Test updates
        monitor.update_cpu_usage(0.75);
        monitor.update_memory_usage(1024 * 1024);
        monitor.update_io_usage(512);

        assert_eq!(monitor.get_cpu_usage(), 0.75);
        assert_eq!(monitor.get_memory_usage(), 1024 * 1024);
        assert_eq!(monitor.get_io_usage(), 512);

        // Test edge cases
        monitor.update_cpu_usage(0.0);
        assert_eq!(monitor.get_cpu_usage(), 0.0);

        monitor.update_cpu_usage(1.0);
        assert_eq!(monitor.get_cpu_usage(), 1.0);
    }

    #[test]
    fn test_compaction_timeout() {
        let mut config = create_test_config();
        config.max_compaction_duration = Duration::from_millis(1); // Very short timeout
        config.io_bandwidth_limit = 1; // Very slow bandwidth to force timeout

        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        let files = vec![create_test_file(1, 1024 * 1024)]; // Large file
        let task = create_test_task(1, files);

        let result = manager.execute_compaction(task);

        // Should fail due to timeout
        assert!(!result.success);
        assert!(result.error_message.is_some());
        assert_eq!(result.error_message.unwrap(), "Compaction timeout");
        assert_eq!(result.space_reclaimed, 0);
        assert_eq!(result.bytes_written, 0);
    }

    #[test]
    fn test_aggressiveness_level_priority_filtering() {
        let mut config = create_test_config();
        config.aggressiveness_level = 2; // Low aggressiveness, high priority threshold

        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        // Create tasks with different priorities
        let files = vec![create_test_file(1, 1024)];

        let mut low_priority_task = create_test_task(1, files.clone());
        low_priority_task.priority = 50; // Low priority

        let mut high_priority_task = create_test_task(2, files);
        high_priority_task.priority = 250; // High priority

        // Low priority task should be rejected due to low aggressiveness
        assert!(!manager.should_schedule_task(&low_priority_task).unwrap());

        // High priority task should be accepted
        assert!(manager.should_schedule_task(&high_priority_task).unwrap());
    }

    #[test]
    fn test_min_compaction_interval() {
        let mut config = create_test_config();
        config.min_compaction_interval = Duration::from_millis(100);

        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        let files = vec![create_test_file(1, 1024)];
        let task = create_test_task(1, files);

        // First execution should work
        let result1 = manager.execute_compaction(task.clone());
        assert!(result1.success);

        // Immediate second execution should be rejected due to min interval
        assert!(!manager.should_schedule_task(&task).unwrap());

        // Wait for interval to pass
        thread::sleep(Duration::from_millis(150));

        // Now should be allowed
        assert!(manager.should_schedule_task(&task).unwrap());
    }

    #[test]
    fn test_stats_calculation() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        // Execute successful compaction
        let files1 = vec![create_test_file(1, 1000)];
        let task1 = create_test_task(1, files1);
        let result1 = manager.execute_compaction(task1);
        assert!(result1.success);

        // Execute another successful compaction
        let files2 = vec![create_test_file(2, 2000)];
        let task2 = create_test_task(2, files2);
        let result2 = manager.execute_compaction(task2);
        assert!(result2.success);

        let stats = manager.get_stats();
        assert_eq!(stats.total_compactions, 2);
        assert_eq!(stats.successful_compactions, 2);
        assert_eq!(stats.failed_compactions, 0);
        assert_eq!(stats.bytes_compacted, 3000); // 1000 + 2000
        assert!(stats.space_reclaimed > 0);
        assert!(stats.total_compaction_time > Duration::from_nanos(0));
        assert!(stats.last_compaction_time.is_some());
    }

    #[test]
    fn test_empty_file_compaction() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        // Create task with empty file
        let files = vec![create_test_file(1, 0)];
        let task = create_test_task(1, files);

        let result = manager.execute_compaction(task);

        assert!(result.success);
        assert_eq!(result.bytes_read, 0);
        assert_eq!(result.space_reclaimed, 0);
        assert_eq!(result.bytes_written, 0);
    }

    #[test]
    fn test_large_file_compaction() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        // Create task with large files
        let files = vec![
            create_test_file(1, 1024 * 1024),     // 1MB
            create_test_file(2, 2 * 1024 * 1024), // 2MB
            create_test_file(3, 3 * 1024 * 1024), // 3MB
        ];
        let task = create_test_task(1, files);

        let result = manager.execute_compaction(task);

        assert!(result.success);
        assert_eq!(result.bytes_read, 6 * 1024 * 1024); // 6MB total
        assert!(result.space_reclaimed > 0);
        assert_eq!(result.output_files.len(), 1); // Should be compacted into single file
    }

    #[test]
    fn test_concurrent_resource_updates() {
        let monitor = Arc::new(ResourceMonitor::new());
        let mut handles = vec![];

        // Spawn multiple threads updating resources
        for i in 0..10 {
            let monitor_clone = Arc::clone(&monitor);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let cpu = (i as f64 + j as f64) / 1000.0;
                    let memory = (i + j) * 1024;
                    let io = (i + j) * 512;

                    monitor_clone.update_cpu_usage(cpu);
                    monitor_clone.update_memory_usage(memory as u64);
                    monitor_clone.update_io_usage(io as u64);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final values are reasonable
        let final_cpu = monitor.get_cpu_usage();
        let final_memory = monitor.get_memory_usage();
        let final_io = monitor.get_io_usage();

        assert!(final_cpu >= 0.0 && final_cpu <= 1.0);
        assert!(final_memory > 0);
        assert!(final_io > 0);
    }

    #[test]
    fn test_completed_tasks_limit() {
        let config = create_test_config();
        let mut strategies = HashMap::new();
        strategies.insert(CompactionStrategyType::SizeTiered, CompactionConfig::default());

        let manager = CompactionManager::new(config, strategies);

        // Execute more than 1000 compactions to test limit
        for i in 1..=1005 {
            let files = vec![create_test_file(i, 1024)];
            let task = create_test_task(i, files);
            let _result = manager.execute_compaction(task);
        }

        // Should only keep last 1000
        let completed = manager.completed_tasks.read().unwrap();
        assert_eq!(completed.len(), 1000);

        // Verify it's the most recent ones
        let oldest_task_id = completed.front().unwrap().task_id;
        let newest_task_id = completed.back().unwrap().task_id;
        assert_eq!(oldest_task_id, 6); // Should start from task 6 (1005 - 1000 + 1)
        assert_eq!(newest_task_id, 1005);
    }
}
