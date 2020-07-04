//! This benchmark binds to an external C library, so it's useful for verifying that cargo-criterion
//! handles library paths and similar correctly.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tch::Tensor;

fn bench_norm(c: &mut Criterion) {
    let t = &Tensor::of_slice(&[0_f32, 0.1f32, 0.5f32, 0.9f32]);
    c.bench_function("norm", |b| b.iter(|| black_box(t).norm()));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_norm
}

criterion_main!(benches);
