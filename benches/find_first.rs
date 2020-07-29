#[macro_use]
extern crate criterion;
extern crate kvik;
extern crate rand;
extern crate rayon;

use kvik::prelude::*;
use rand::prelude::*;

use criterion::{Criterion, ParameterizedBenchmark};
use std::time::Duration;

const INPUT_SIZE: usize = 10_000_000;

fn random_vec(size: usize) -> Vec<usize> {
    let mut input: Vec<_> = (0..size).collect();
    let mut rng = rand::thread_rng();
    input.shuffle(&mut rng);
    input
}

// return rayon's initial counter for a given number of threads
fn log(t: usize) -> usize {
    (t as f64).log2().ceil() as usize + 1
}

fn ffirst_bench(c: &mut Criterion) {
    let num_threads: Vec<_> = (1..33).map(|elem| elem * 2_usize).collect();
    c.bench(
        "random input",
        ParameterizedBenchmark::new(
            "ffirst with doubling blocks",
            |b, nthreads| {
                b.iter_with_setup(
                    || {
                        (
                            random_vec(INPUT_SIZE),
                            rayon::ThreadPoolBuilder::new()
                                .num_threads(*nthreads)
                                .build()
                                .expect("Couldn't build thread pool"),
                        )
                    },
                    |(input, tp)| {
                        tp.install(|| {
                            assert!(input
                                .par_iter()
                                .filter(|&e| *e == INPUT_SIZE / 2)
                                .next()
                                .by_blocks(std::iter::successors(Some(*nthreads), |s| Some(
                                    s.saturating_mul(2)
                                )))
                                .rayon(log(*nthreads))
                                .reduce_with(|a, _| a)
                                .is_some());
                        });
                    },
                )
            },
            num_threads.clone(),
        )
        .with_function("ffirst with 1.5 blocks", |b, nthreads| {
            b.iter_with_setup(
                || {
                    (
                        random_vec(INPUT_SIZE),
                        rayon::ThreadPoolBuilder::new()
                            .num_threads(*nthreads)
                            .build()
                            .expect("Couldn't build thread pool"),
                    )
                },
                |(input, tp)| {
                    tp.install(|| {
                        assert!(input
                            .par_iter()
                            .filter(|&e| *e == INPUT_SIZE / 2)
                            .next()
                            .by_blocks(std::iter::successors(Some(*nthreads), |s| Some(
                                s.saturating_add(s / 2)
                            )))
                            .rayon(log(*nthreads))
                            .reduce_with(|a, _| a)
                            .is_some());
                    });
                },
            )
        })
        .with_function("ffirst without blocks", |b, nthreads| {
            b.iter_with_setup(
                || {
                    (
                        random_vec(INPUT_SIZE),
                        rayon::ThreadPoolBuilder::new()
                            .num_threads(*nthreads)
                            .build()
                            .expect("Couldn't build thread pool"),
                    )
                },
                |(input, tp)| {
                    tp.install(|| {
                        assert!(input
                            .par_iter()
                            .rayon(log(*nthreads))
                            .find_first(|elem| **elem == INPUT_SIZE / 2)
                            .is_some());
                    });
                },
            )
        })
        .with_function("ffirst rayon", |b, nthreads| {
            b.iter_with_setup(
                || {
                    (
                        random_vec(INPUT_SIZE),
                        rayon::ThreadPoolBuilder::new()
                            .num_threads(*nthreads)
                            .build()
                            .expect("Couldn't build thread pool"),
                    )
                },
                |(input, tp)| {
                    tp.install(|| {
                        assert!(rayon::iter::ParallelIterator::find_first(
                            rayon::iter::IntoParallelRefIterator::par_iter(&input),
                            |elem| **elem == INPUT_SIZE / 2
                        )
                        .is_some());
                    });
                },
            )
        }),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = ffirst_bench
}
criterion_main!(benches);
