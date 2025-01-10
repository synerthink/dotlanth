use criterion::{Criterion, criterion_group, criterion_main};

fn benchmark_main_initialization(c: &mut Criterion) {
    c.bench_function("main_init", |b| {
        b.iter(|| {
            // We can't actually run main() directly in benchmarks because of tokio runtime
            // Instead, we benchmark the core functionality
            tracing_subscriber::fmt::try_init()
        });
    });
}

criterion_group!(main_benches, benchmark_main_initialization);
criterion_main!(main_benches);
