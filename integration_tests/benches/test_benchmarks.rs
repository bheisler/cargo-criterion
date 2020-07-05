//! This benchmark defines some test benchmarks that exercise various parts of cargo-criterion.

use criterion::{
    criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion, PlotConfiguration,
    SamplingMode, Throughput,
};
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

const SIZE: usize = 1024;

fn throughput_tests(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    group.throughput(Throughput::Bytes(SIZE as u64));
    group.bench_function("Bytes", |bencher| {
        bencher.iter(|| (0..SIZE).map(|i| i as u8).collect::<Vec<_>>())
    });

    group.throughput(Throughput::Elements(SIZE as u64));
    group.bench_function("Elem", |bencher| {
        bencher.iter(|| (0..SIZE).map(|i| i as u64).collect::<Vec<_>>())
    });

    group.finish();
}

fn log_scale_tests(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

    let mut group = c.benchmark_group("log_scale");
    group.plot_config(plot_config);

    for time in &[1, 100, 10000] {
        group.bench_with_input(
            BenchmarkId::new("sleep (micros)", time),
            time,
            |bencher, input| bencher.iter(|| sleep(Duration::from_micros(*input))),
        );
    }
    group.finish()
}

fn linear_scale_tests(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Linear);

    let mut group = c.benchmark_group("linear_scale");
    group.plot_config(plot_config);

    for time in &[1, 2, 3] {
        group.bench_with_input(
            BenchmarkId::new("sleep (millis)", time),
            time,
            |bencher, input| bencher.iter(|| sleep(Duration::from_millis(*input))),
        );
    }
    group.finish()
}

// These benchmarks are used for testing cargo-criterion, so to make the tests faster we configure
// them to run quickly. This is not recommended for real benchmarks.
criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_millis(500))
        .nresamples(2000);
    targets = special_characters, sampling_mode_tests, throughput_tests, log_scale_tests, linear_scale_tests,
}
criterion_main!(benches);
