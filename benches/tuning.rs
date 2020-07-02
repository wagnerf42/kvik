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

const PROBLEM_SIZE: u32 = 100_000_000;

fn sort_benchmarks(c: &mut Criterion) {
    let num_threads: Vec<usize> = vec![54, 56, 58, 60, 64];
    let upper_bounds: Vec<u32> = vec![4, 6, 8, 10, 12];
    c.bench(
        "tuning bounds",
        ParameterizedBenchmark::new(
            "slice sort",
            |b, (nt, u)| {
                b.iter_with_setup(
                    || {
                        let tp = rayon::ThreadPoolBuilder::new()
                            .num_threads(*nt)
                            .build()
                            .expect("Couldn't build thread pool");
                        let mut input = (0..PROBLEM_SIZE).collect::<Vec<_>>();
                        let mut rng = rand::thread_rng();
                        input.shuffle(&mut rng);
                        (tp, input)
                    },
                    |(tp, mut input)| {
                        tp.install(|| {
                            slice_par_sort(&mut input, *u);
                            input
                        });
                    },
                )
            },
            iproduct!(num_threads, upper_bounds),
        ),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = sort_benchmarks
}
criterion_main!(benches);
