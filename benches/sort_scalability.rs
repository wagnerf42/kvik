#[macro_use]
extern crate criterion;
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate rayon_try_fold;

use rand::prelude::*;
use rayon::prelude::*;
use rayon_try_fold::{iter_par_sort, slice_par_sort};
use std::time::Duration;

use criterion::{Benchmark, Criterion, ParameterizedBenchmark};

const PROBLEM_SIZE: u32 = 100_000_000;

fn sort_benchmarks(c: &mut Criterion) {
    //let num_threads: Vec<usize> = vec![1];
    let num_threads: Vec<usize> = vec![
        1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40, 42, 44, 46,
        48, 50, 52, 54, 56, 58, 60, 64,
    ];
    c.bench(
        "sort baseline",
        Benchmark::new("sequential sort", |b| {
            b.iter_with_setup(
                || {
                    let tp = rayon::ThreadPoolBuilder::new()
                        .num_threads(1)
                        .build()
                        .expect("Couldn't build thread pool");
                    let mut input = (0..PROBLEM_SIZE).collect::<Vec<_>>();
                    let mut rng = rand::thread_rng();
                    input.shuffle(&mut rng);
                    (tp, input)
                },
                |(tp, mut input)| {
                    tp.install(|| {
                        input.sort();
                        input
                    });
                },
            )
        }),
    );
    c.bench(
        "sort scalability",
        ParameterizedBenchmark::new(
            "rayon sort",
            |b, nt| {
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
                            input.par_sort();
                            input
                        });
                    },
                )
            },
            num_threads.clone(),
        )
        .with_function("slice par sort", |b, nt| {
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
                        slice_par_sort(&mut input, 1, usize::MAX);
                        input
                    });
                },
            )
        })
        .with_function("iter par sort", |b, nt| {
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
                        iter_par_sort(&mut input);
                        input
                    });
                },
            )
        }),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(10).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = sort_benchmarks
}
criterion_main!(benches);
