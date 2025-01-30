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
