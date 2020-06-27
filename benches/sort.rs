#[macro_use]
extern crate criterion;
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate rayon_try_fold;

use rand::prelude::*;
use rayon_try_fold::slice_par_sort;
use std::time::Duration;

use criterion::{Criterion, ParameterizedBenchmark};
use itertools::iproduct;

const PROBLEM_SIZE: u32 = 100_000_000;

fn sort_benchmarks(c: &mut Criterion) {
    let threads: Vec<usize> = vec![10, 16, 30, 50, 54, 56, 60, 64];
    let caps: Vec<isize> = vec![1, 2, 4, 8, 16, 32, 64];
    c.bench(
        "fusion task cap",
        ParameterizedBenchmark::new(
            "slice sort",
            |b, (num_threads, num_tasks)| {
                b.iter_with_setup(
                    || {
                        let tp = rayon::ThreadPoolBuilder::new()
                            .num_threads(*num_threads)
                            .build()
                            .expect("Couldn't build thread pool");
                        let mut input = (0..PROBLEM_SIZE).collect::<Vec<_>>();
                        let mut rng = rand::thread_rng();
                        input.shuffle(&mut rng);
                        (tp, input)
                    },
                    |(tp, mut input)| {
                        tp.install(|| {
                            slice_par_sort(&mut input, *num_tasks);
                            input
                        });
                    },
                )
            },
            iproduct!(threads.clone(), caps.clone()),
        ),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = sort_benchmarks
}
criterion_main!(benches);
