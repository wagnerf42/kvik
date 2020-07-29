#[macro_use]
extern crate criterion;
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate kvik;

use rand::prelude::*;
use rayon::prelude::*;
use kvik::{iter_par_sort, slice_par_sort};
use std::time::Duration;

use criterion::{Criterion, ParameterizedBenchmark};

const NUM_THREADS: usize = 16;

fn sort_benchmarks(c: &mut Criterion) {
    //let sizes: Vec<u32> = vec![100_000];
    let sizes: Vec<u32> = vec![100_000, 1_000_000, 10_000_000, 100_000_000];
    c.bench(
        "random input",
        ParameterizedBenchmark::new(
            "sequential sort",
            |b, input_size| {
                b.iter_with_setup(
                    || {
                        let tp = rayon::ThreadPoolBuilder::new()
                            .num_threads(1)
                            .build()
                            .expect("Couldn't build thread pool");
                        let mut input = (0..*input_size).collect::<Vec<_>>();
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
            },
            sizes.clone(),
        )
        .with_function("rayon sort", |b, input_size| {
            b.iter_with_setup(
                || {
                    let tp = rayon::ThreadPoolBuilder::new()
                        .num_threads(NUM_THREADS)
                        .build()
                        .expect("Couldn't build thread pool");
                    let mut input = (0..*input_size).collect::<Vec<_>>();
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
        })
        .with_function("slice par sort", |b, input_size| {
            b.iter_with_setup(
                || {
                    let tp = rayon::ThreadPoolBuilder::new()
                        .num_threads(NUM_THREADS)
                        .build()
                        .expect("Couldn't build thread pool");
                    let mut input = (0..*input_size).collect::<Vec<_>>();
                    let mut rng = rand::thread_rng();
                    input.shuffle(&mut rng);
                    (tp, input)
                },
                |(tp, mut input)| {
                    tp.install(|| {
                        slice_par_sort(&mut input);
                        input
                    });
                },
            )
        })
        .with_function("iter par sort", |b, input_size| {
            b.iter_with_setup(
                || {
                    let tp = rayon::ThreadPoolBuilder::new()
                        .num_threads(NUM_THREADS)
                        .build()
                        .expect("Couldn't build thread pool");
                    let mut input = (0..*input_size).collect::<Vec<_>>();
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
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = sort_benchmarks
}
criterion_main!(benches);
