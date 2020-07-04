//! This is a Bencher benchmark, used to verify that cargo-criterion can at least run non-criterion
//! benchmarks, even if the more fancy features aren't available.

#[macro_use]
extern crate bencher;

use bencher::Bencher;

fn bencher_test(bench: &mut Bencher) {
    bench.iter(|| (0..1000).fold(0, |x, y| x + y))
}

benchmark_group!(benches, bencher_test);
benchmark_main!(benches);
