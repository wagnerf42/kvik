#[macro_use]
extern crate criterion;
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate kvik;

// TODO: we should show the whole distribution of times and not the average time

use rand::prelude::*;
use kvik::prelude::*;

use criterion::{Criterion, ParameterizedBenchmark};
use std::time::Duration;

fn random_vec(size: usize) -> Vec<bool> {
    let mut input: Vec<bool> = std::iter::repeat(true).take(size).collect();
    let mut rng = rand::thread_rng();
    *input.choose_mut(&mut rng).unwrap() = false;
    input
}

fn all_benchmarks(c: &mut Criterion) {
    let sizes: Vec<usize> = vec![100_000, 1_000_000, 10_000_000];
    c.bench(
        "random input",
        ParameterizedBenchmark::new(
            "all with doubling blocks",
            |b, input_size| {
                b.iter_with_setup(
                    || random_vec(*input_size),
                    |input| {
                        assert!(!input
                            .par_iter()
                            .by_blocks(std::iter::successors(Some(16usize), |s| Some(
                                s.saturating_mul(2)
                            )))
                            .all(|x| *x))
                    },
                )
            },
            sizes.clone(),
        )
        .with_function("all with 1.5 blocks", |b, input_size| {
            b.iter_with_setup(
                || random_vec(*input_size),
                |input| {
                    assert!(!input
                        .par_iter()
                        .by_blocks(std::iter::successors(Some(16usize), |s| Some(
                            s.saturating_add(s / 2)
                        )))
                        .all(|x| *x))
                },
            )
        })
        .with_function("all without blocks", |b, input_size| {
            b.iter_with_setup(
                || random_vec(*input_size),
                |input| assert!(!input.par_iter().all(|x| *x)),
            )
        }),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = all_benchmarks
}
criterion_main!(benches);
