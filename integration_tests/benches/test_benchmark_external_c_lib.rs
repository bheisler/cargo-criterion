//! This benchmark binds to an external C library, so it's useful for verifying that cargo-criterion
//! handles library paths and similar correctly.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use tch::Tensor;

fn bench_norm(c: &mut Criterion) {
    let t = &Tensor::of_slice(&[0_f32, 0.1f32, 0.5f32, 0.9f32]);
    c.bench_function("norm", |b| b.iter(|| black_box(t).norm()));
}

// These benchmarks are used for testing cargo-criterion, so to make the tests faster we configure
// them to run quickly. This is not recommended for real benchmarks.
criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_millis(500))
        .nresamples(2000);
    targets = bench_norm
}

criterion_main!(benches);
