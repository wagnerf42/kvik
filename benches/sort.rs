#[macro_use]
extern crate criterion;
#[macro_use]
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate rayon_try_fold;

use rand::prelude::*;
use rayon_try_fold::slice_par_sort;
use std::time::Duration;

use criterion::{Criterion, ParameterizedBenchmark};

fn sort_benchmarks(c: &mut Criterion) {
    //let sizes: Vec<u32> = vec![100_000];
    let sizes: Vec<u32> = vec![100_000_000];
    let lower_limits: Vec<usize> = vec![512, 1024, 2048];
    let upper_fractional_limits: Vec<usize> = vec![10, 100, 1000];
    let threads: Vec<usize> = vec![16, 54];
    c.bench(
        "random input",
        ParameterizedBenchmark::new(
            "slice par sort",
            |b, (input_size, lower, upper, threads)| {
                b.iter_with_setup(
                    || {
                        let tp = rayon::ThreadPoolBuilder::new()
                            .num_threads(*threads)
                            .build()
                            .expect("Couldn't build thread pool");
                        let mut input = (0..*input_size).collect::<Vec<_>>();
                        let mut rng = rand::thread_rng();
                        input.shuffle(&mut rng);
                        (tp, input, *lower, *input_size as usize / upper)
                    },
                    |(tp, mut input, lower, upper)| {
                        tp.install(|| {
                            slice_par_sort(&mut input, lower, upper);
                        });
                    },
                )
            },
            iproduct!(sizes, lower_limits, upper_fractional_limits, threads),
        ),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = sort_benchmarks
}
criterion_main!(benches);
