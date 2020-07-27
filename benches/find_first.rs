#[macro_use]
extern crate criterion;
#[macro_use]
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate rayon_try_fold;

use rand::prelude::*;
use rayon_try_fold::prelude::*;

use criterion::{Criterion, ParameterizedBenchmark};
use std::time::Duration;

fn random_vec(size: usize) -> Vec<usize> {
    let mut input: Vec<_> = (0..size).collect();
    let mut rng = rand::thread_rng();
    input.shuffle(&mut rng);
    input
}

fn ffirst_bench(c: &mut Criterion) {
    let sizes: Vec<usize> = vec![100_000, 1_000_000, 10_000_000];
    let num_threads: Vec<_> = (1..33).map(|elem| elem * 2_usize).collect();
    c.bench(
        "random input",
        ParameterizedBenchmark::new(
            "ffirst with doubling blocks",
            |b, (nthreads, input_size)| {
                b.iter_with_setup(
                    || {
                        (
                            random_vec(*input_size),
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
                                .by_blocks(std::iter::successors(Some(16usize), |s| Some(
                                    s.saturating_mul(2)
                                )))
                                .find_first(|elem| **elem == *input_size / 2)
                                .is_some());
                        });
                    },
                )
            },
            iproduct!(num_threads.clone(), sizes.clone()),
        )
        .with_function("ffirst with 1.5 blocks", |b, (nthreads, input_size)| {
            b.iter_with_setup(
                || {
                    (
                        random_vec(*input_size),
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
                            .by_blocks(std::iter::successors(Some(16usize), |s| Some(
                                s.saturating_add(s / 2)
                            )))
                            .find_first(|elem| **elem == *input_size / 2)
                            .is_some());
                    });
                },
            )
        })
        .with_function("ffirst without blocks", |b, (nthreads, input_size)| {
            b.iter_with_setup(
                || {
                    (
                        random_vec(*input_size),
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
                            .find_first(|elem| **elem == *input_size / 2)
                            .is_some());
                    });
                },
            )
        })
        .with_function("ffirst rayon", |b, (nthreads, input_size)| {
            b.iter_with_setup(
                || {
                    (
                        random_vec(*input_size),
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
                            |elem| **elem == *input_size / 2
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
