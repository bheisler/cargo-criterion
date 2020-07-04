//! This benchmark defines some test benchmarks that exercise various parts of cargo-criterion.

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
use std::thread::sleep;
use std::time::Duration;

fn special_characters(c: &mut Criterion) {
    let mut group = c.benchmark_group("\"*group/\"");
    group.bench_function("\"*benchmark/\" '", |b| b.iter(|| 1 + 1));
    group.finish();
}

fn sampling_mode_tests(c: &mut Criterion) {
    let mut group = c.benchmark_group("sampling_mode");

    group.sampling_mode(SamplingMode::Auto);
    group.bench_function("Auto (short)", |bencher| {
        bencher.iter(|| sleep(Duration::from_millis(0)))
    });
    group.bench_function("Auto (long)", |bencher| {
        bencher.iter(|| sleep(Duration::from_millis(10)))
    });

    group.sampling_mode(SamplingMode::Linear);
    group.bench_function("Linear", |bencher| {
        bencher.iter(|| sleep(Duration::from_millis(0)))
    });

    group.sampling_mode(SamplingMode::Flat);
    group.bench_function("Flat", |bencher| {
        bencher.iter(|| sleep(Duration::from_millis(10)))
    });

    group.finish();
}

const SIZE: usize = 1024 * 1024;

fn throughput_tests(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    group.throughput(Throughput::Bytes(SIZE as u64));
    group.bench_function("Bytes", |bencher| {
        bencher.iter(|| (0..SIZE).map(|i| i as u8).collect::<Vec<_>>())
    });

    group.throughput(Throughput::Elements(SIZE as u64));
    group.bench_function("Bytes", |bencher| {
        bencher.iter(|| (0..SIZE).map(|i| i as u64).collect::<Vec<_>>())
    });

    group.finish();
}

criterion_group!(
    benches,
    special_characters,
    sampling_mode_tests,
    throughput_tests
);
criterion_main!(benches);
