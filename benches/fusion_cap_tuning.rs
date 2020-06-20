#[macro_use]
extern crate criterion;
#[macro_use]
extern crate itertools;

use rand::prelude::*;
use rayon_try_fold::slice_par_sort;
use std::time::Duration;

use criterion::{Criterion, ParameterizedBenchmark};

fn sort_benchmarks(c: &mut Criterion) {
    let sizes: Vec<u32> = vec![100_000_000];
    let bounds: Vec<usize> = vec![10_000, 100_000, 1_000_000];
    let threads: Vec<usize> = vec![10, 16, 30, 50, 54, 60, 64];
    c.bench(
        "fusion cap tuning",
        ParameterizedBenchmark::new(
            "slice par sort",
            |b, (input_size, num_threads, bound)| {
                b.iter_with_setup(
                    || {
                        let tp = rayon::ThreadPoolBuilder::new()
                            .num_threads(*num_threads)
                            .build()
                            .expect("Couldn't build thread pool");
                        let mut input = (0..*input_size).collect::<Vec<_>>();
                        let mut rng = rand::thread_rng();
                        input.shuffle(&mut rng);
                        (tp, input)
                    },
                    |(tp, mut input)| {
                        tp.install(|| {
                            slice_par_sort(&mut input, *bound);
                            input
                        });
                    },
                )
            },
            iproduct!(sizes.clone(), threads.clone(), bounds.clone()),
        ),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = sort_benchmarks
}
criterion_main!(benches);
