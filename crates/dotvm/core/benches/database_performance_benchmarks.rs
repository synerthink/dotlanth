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

//! Performance benchmarks for database opcodes
//!
//! This module provides comprehensive benchmarks to verify that database
//! operations meet the performance requirements, especially the <1ms read latency.

use criterion::{BenchmarkId, Criterion, PlottingBackend, Throughput, black_box, criterion_group, criterion_main};
use dotdb_core::storage_engine::wal::WalConfig;
use dotdb_core::storage_engine::{BufferManager, TransactionManager, WriteAheadLog};
use dotdb_core::storage_engine::{FileFormat, lib::StorageConfig};
use dotvm_core::vm::database_executor::{DatabaseOpcodeExecutor, IndexOperation, IndexType, OrderBy, OrderDirection, QueryCondition, QueryOperator, QuerySpec, StreamSpec, TransactionOp};
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn create_benchmark_executor() -> DatabaseOpcodeExecutor {
    let storage_config = StorageConfig::default();
    let file_format = Arc::new(Mutex::new(FileFormat::new(storage_config.clone())));
    let buffer_manager = Arc::new(BufferManager::new(file_format, &storage_config));

    let test_dir = std::env::temp_dir().join("dotvm_benchmark_wal");
    std::fs::create_dir_all(&test_dir).ok();
    let mut wal_config = WalConfig::default();
    wal_config.directory = test_dir;

    let wal = Arc::new(WriteAheadLog::new(wal_config).unwrap());
    let transaction_manager = Arc::new(TransactionManager::new(buffer_manager, wal));

    DatabaseOpcodeExecutor::new(transaction_manager).expect("Failed to create benchmark executor")
}

/// Benchmark to specifically test the <1ms read requirement
fn bench_sub_millisecond_reads(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    // Pre-populate with test data
    for i in 0..1000 {
        let key = format!("benchmark_key_{}", i).into_bytes();
        let value = format!("benchmark_value_{}", i).into_bytes();
        let _ = executor.execute_db_write(1, key, value);
    }

    let mut group = c.benchmark_group("sub_millisecond_reads");
    group.significance_level(0.1).sample_size(100);
    group.measurement_time(Duration::from_secs(5));

    // Test different key patterns
    let test_keys = vec![
        ("existing_key", b"benchmark_key_500".to_vec()),
        ("non_existing_key", b"non_existent_key".to_vec()),
        ("small_key", b"k1".to_vec()),
        ("large_key", vec![42u8; 1024]),
    ];

    for (key_type, key) in test_keys {
        group.bench_with_input(BenchmarkId::new("read_latency", key_type), &key, |b, key| {
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();

                for _i in 0..iters {
                    let result = executor.execute_db_read(black_box(1), black_box(key.clone()));
                    black_box(result);
                }

                let total_duration = start.elapsed();
                let avg_duration = total_duration / iters as u32;

                // Check if average exceeds 1ms requirement
                if avg_duration > Duration::from_millis(1) {
                    eprintln!("⚠️  WARNING: Average read latency for {} ({:?}) exceeds 1ms requirement!", key_type, avg_duration);
                } else {
                    println!("✅ Average read latency for {}: {:?} (meets <1ms requirement)", key_type, avg_duration);
                }

                total_duration
            });
        });
    }

    group.finish();
}

/// Benchmark read throughput with different data sizes
fn bench_read_throughput(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    let mut group = c.benchmark_group("read_throughput");
    group.significance_level(0.1).sample_size(50);

    // Test different value sizes
    let value_sizes = vec![64, 256, 1024, 4096, 16384];

    for value_size in value_sizes {
        // Pre-populate with test data
        let key = format!("throughput_key_{}", value_size).into_bytes();
        let value = vec![42u8; value_size];
        let _ = executor.execute_db_write(1, key.clone(), value);

        group.throughput(Throughput::Bytes(value_size as u64));
        group.bench_with_input(BenchmarkId::new("value_size", value_size), &key, |b, key| {
            b.iter(|| {
                let result = executor.execute_db_read(black_box(1), black_box(key.clone()));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark write performance and throughput
fn bench_write_performance(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    let mut group = c.benchmark_group("write_performance");
    group.significance_level(0.1).sample_size(50);

    // Test different value sizes
    let value_sizes = vec![64, 256, 1024, 4096];

    for value_size in value_sizes {
        group.throughput(Throughput::Bytes(value_size as u64));
        group.bench_with_input(BenchmarkId::new("write_size", value_size), &value_size, |b, &size| {
            let mut counter = 0;
            b.iter(|| {
                counter += 1;
                let key = format!("write_key_{}_{}", counter, size).into_bytes();
                let value = vec![42u8; size];
                let result = executor.execute_db_write(black_box(1), black_box(key), black_box(value));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark query performance with different complexities
fn bench_query_performance(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    // Pre-populate with test data
    for i in 0..1000 {
        let key = format!("query_key_{}", i).into_bytes();
        let value = format!(r#"{{"id": {}, "name": "user_{}", "age": {}, "status": "active"}}"#, i, i, 20 + (i % 50)).into_bytes();
        let _ = executor.execute_db_write(1, key, value);
    }

    let mut group = c.benchmark_group("query_performance");
    group.significance_level(0.1).sample_size(20);

    // Test queries with different complexity levels
    let query_complexities = vec![("simple", 1), ("medium", 3), ("complex", 5)];

    for (name, condition_count) in query_complexities {
        let mut conditions = Vec::new();
        for i in 0..condition_count {
            conditions.push(QueryCondition {
                field: format!("field_{}", i),
                operator: QueryOperator::Equal,
                value: format!("value_{}", i).into_bytes(),
            });
        }

        let query_spec = QuerySpec {
            table_id: 1,
            conditions,
            projections: vec!["id".to_string(), "name".to_string()],
            limit: Some(100),
            offset: None,
            order_by: vec![OrderBy {
                field: "id".to_string(),
                direction: OrderDirection::Ascending,
            }],
        };

        group.bench_with_input(BenchmarkId::new("complexity", name), &query_spec, |b, query_spec| {
            b.iter(|| {
                let result = executor.execute_db_query(black_box(query_spec.clone()));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark transaction performance
fn bench_transaction_performance(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    let mut group = c.benchmark_group("transaction_performance");
    group.significance_level(0.1).sample_size(20);

    // Test transactions with different operation counts
    let operation_counts = vec![1, 5, 10, 20];

    for op_count in operation_counts {
        group.bench_with_input(BenchmarkId::new("operations", op_count), &op_count, |b, &op_count| {
            let mut counter = 0;
            b.iter(|| {
                counter += 1;
                let mut tx_ops = Vec::new();
                for i in 0..op_count {
                    tx_ops.push(TransactionOp::Write {
                        table_id: 1,
                        key: format!("tx_key_{}_{}", counter, i).into_bytes(),
                        value: format!("tx_value_{}_{}", counter, i).into_bytes(),
                    });
                }

                let result = executor.execute_db_transaction(black_box(tx_ops));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark concurrent read performance
fn bench_concurrent_reads(c: &mut Criterion) {
    let executor = Arc::new(create_benchmark_executor());

    // Pre-populate with test data
    for i in 0..1000 {
        let key = format!("concurrent_key_{}", i).into_bytes();
        let value = format!("concurrent_value_{}", i).into_bytes();
        let _ = executor.execute_db_write(1, key, value);
    }

    let mut group = c.benchmark_group("concurrent_reads");
    group.significance_level(0.1).sample_size(10);

    // Test different concurrency levels
    let thread_counts = vec![1, 2, 4, 8];

    for thread_count in thread_counts {
        group.bench_with_input(BenchmarkId::new("threads", thread_count), &thread_count, |b, &thread_count| {
            b.iter(|| {
                let mut handles = vec![];

                for i in 0..thread_count {
                    let executor_clone = Arc::clone(&executor);
                    let handle = std::thread::spawn(move || {
                        let key = format!("concurrent_key_{}", i % 100).into_bytes();
                        let result = executor_clone.execute_db_read(1, key);
                        black_box(result)
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark index operations
fn bench_index_operations(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    let mut group = c.benchmark_group("index_operations");
    group.significance_level(0.1).sample_size(10);

    // Test different index types
    let index_types = vec![
        ("btree", IndexType::BTree),
        ("hash", IndexType::Hash),
        ("composite", IndexType::Composite(vec!["field1".to_string(), "field2".to_string()])),
    ];

    for (name, index_type) in index_types {
        group.bench_with_input(BenchmarkId::new("index_type", name), &index_type, |b, index_type| {
            let mut counter = 0;
            b.iter(|| {
                counter += 1;
                let index_op = IndexOperation::Create {
                    table_id: 1,
                    field: format!("benchmark_field_{}", counter),
                    index_type: index_type.clone(),
                };

                let result = executor.execute_db_index(black_box(index_op));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark stream creation and management
fn bench_stream_operations(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    let mut group = c.benchmark_group("stream_operations");
    group.significance_level(0.1).sample_size(10);

    // Test different batch sizes
    let batch_sizes = vec![10, 50, 100, 500];

    for batch_size in batch_sizes {
        let stream_spec = StreamSpec {
            query: QuerySpec {
                table_id: 1,
                conditions: vec![],
                projections: vec!["*".to_string()],
                limit: None,
                offset: None,
                order_by: vec![],
            },
            batch_size,
            timeout_ms: Some(5000),
        };

        group.bench_with_input(BenchmarkId::new("batch_size", batch_size), &stream_spec, |b, stream_spec| {
            b.iter(|| {
                let result = executor.execute_db_stream(black_box(stream_spec.clone()));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Comprehensive latency analysis
fn bench_latency_analysis(c: &mut Criterion) {
    let executor = create_benchmark_executor();

    // Pre-populate with various data sizes
    for i in 0..100 {
        for size in [64, 256, 1024, 4096] {
            let key = format!("latency_key_{}_{}", i, size).into_bytes();
            let value = vec![42u8; size];
            let _ = executor.execute_db_write(1, key, value);
        }
    }

    let mut group = c.benchmark_group("latency_analysis");
    group.significance_level(0.1).sample_size(50);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("comprehensive_latency_test", |b| {
        b.iter_custom(|iters| {
            let mut latencies = Vec::new();

            for i in 0..iters {
                let key = format!("latency_key_{}_{}", i % 100, 256).into_bytes();

                let start = std::time::Instant::now();
                let result = executor.execute_db_read(black_box(1), black_box(key));
                let latency = start.elapsed();

                black_box(result);
                latencies.push(latency);
            }

            // Analyze latency distribution
            latencies.sort();
            let p50 = latencies[latencies.len() / 2];
            let p95 = latencies[latencies.len() * 95 / 100];
            let p99 = latencies[latencies.len() * 99 / 100];
            let max = latencies[latencies.len() - 1];

            println!("\n📊 Latency Analysis Results:");
            println!("   P50 (median): {:?}", p50);
            println!("   P95: {:?}", p95);
            println!("   P99: {:?}", p99);
            println!("   Max: {:?}", max);

            let exceeds_1ms = latencies.iter().filter(|&&l| l > Duration::from_millis(1)).count();
            let percentage = (exceeds_1ms as f64 / latencies.len() as f64) * 100.0;

            if percentage > 0.0 {
                println!("   ⚠️  {:.2}% of reads exceeded 1ms requirement", percentage);
            } else {
                println!("   ✅ All reads met <1ms requirement");
            }

            latencies.iter().sum::<Duration>()
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().plotting_backend(PlottingBackend::Plotters);
    targets = bench_sub_millisecond_reads,
    bench_read_throughput,
    bench_write_performance,
    bench_query_performance,
    bench_transaction_performance,
    bench_concurrent_reads,
    bench_index_operations,
    bench_stream_operations,
    bench_latency_analysis
}

criterion_main!(benches);
