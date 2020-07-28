#[macro_use]
extern crate criterion;
#[macro_use]
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate rayon_try_fold;

use rayon_try_fold::prelude::*;

use criterion::{Criterion, ParameterizedBenchmark};
use std::time::Duration;

fn random_vec(size: usize) -> Vec<i32> {
    std::iter::repeat_with(|| rand::random::<i32>() % 1_000)
        .take(size)
        .collect()
}

fn fuse_slices<'a: 'c, 'b: 'c, 'c, T: 'a + 'b>(s1: &'a [T], s2: &'b [T]) -> &'c [T] {
    let ptr1 = s1.as_ptr();
    unsafe {
        assert_eq!(ptr1.add(s1.len()) as *const T, s2.as_ptr(),);
        std::slice::from_raw_parts(ptr1, s1.len() + s2.len())
    }
}

fn iter_sum<'a, I: Iterator<Item = &'a i32>>(iter: I) -> i32 {
    iter.scan(0, |p, e| {
        *p += *e;
        Some(*p)
    })
    .max()
    .unwrap_or(0)
}

fn max_sum_seq(slice: &[i32]) -> i32 {
    if slice.len() <= 1 {
        slice.first().copied().unwrap_or(0)
    } else {
        let mid = slice.len() / 2;
        let (left, right) = slice.split_at(mid);
        let left_sum = max_sum_seq(left);
        let right_sum = max_sum_seq(right);
        let mid_sum = iter_sum(left.iter().rev()) + iter_sum(right.iter());
        left_sum.max(right_sum).max(mid_sum)
    }
}

fn par_iter_sum<'a, I: ParallelIterator<Item = &'a i32>>(iter: I) -> i32 {
    iter.fold(
        || (0, 0),
        |(current_sum, current_max), &e| {
            let new_sum = current_sum + e;
            (new_sum, new_sum.max(current_max))
        },
    )
    .reduce(
        || (0, 0),
        |(left_sum, left_max), (right_sum, right_max)| {
            ((left_sum + right_sum), left_max.max(left_sum + right_max))
        },
    )
    .1
}

fn max_sum_par(slice: &[i32]) -> i32 {
    slice
        .wrap_iter()
        .map(|s| (s, max_sum_seq(s)))
        .rayon((rayon::current_num_threads() as f64).log2().ceil() as usize + 1)
        .reduce_with(|(left, left_sum), (right, right_sum)| {
            let (left_mid, right_mid) = rayon::join(
                || par_iter_sum(left.par_iter().rev()),
                || par_iter_sum(right.par_iter()),
            );
            let mid_sum = left_mid + right_mid;
            (
                fuse_slices(left, right),
                left_sum.max(right_sum).max(mid_sum),
            )
        })
        .map(|(_, sum)| sum)
        .unwrap_or(0)
}

// almost sequential reduction
fn max_sum_par_seq(slice: &[i32]) -> i32 {
    slice
        .wrap_iter()
        .map(|s| (s, max_sum_seq(s)))
        .rayon((rayon::current_num_threads() as f64).log2().ceil() as usize + 1)
        .reduce_with(|(left, left_sum), (right, right_sum)| {
            let (left_mid, right_mid) =
                rayon::join(|| iter_sum(left.iter().rev()), || iter_sum(right.iter()));
            let mid_sum = left_mid + right_mid;
            (
                fuse_slices(left, right),
                left_sum.max(right_sum).max(mid_sum),
            )
        })
        .map(|(_, sum)| sum)
        .unwrap_or(0)
}

// par code, sequential reduction
fn max_sum_par_fullseq(slice: &[i32]) -> i32 {
    slice
        .wrap_iter()
        .map(|s| (s, max_sum_seq(s)))
        .rayon((rayon::current_num_threads() as f64).log2().ceil() as usize + 1)
        .reduce_with(|(left, left_sum), (right, right_sum)| {
            let mid_sum = iter_sum(left.iter().rev()) + iter_sum(right.iter());
            (
                fuse_slices(left, right),
                left_sum.max(right_sum).max(mid_sum),
            )
        })
        .map(|(_, sum)| sum)
        .unwrap_or(0)
}

fn maximum_subarray(c: &mut Criterion) {
    let sizes: Vec<usize> = vec![100_000, 1_000_000, 5_000_000];
    let num_threads: Vec<_> = (1..=2).collect();
    c.bench(
        "random input",
        ParameterizedBenchmark::new(
            "maximum subarray full parallel",
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
                        tp.install(|| max_sum_par(&input));
                    },
                )
            },
            iproduct!(num_threads.clone(), sizes.clone()),
        )
        .with_function(
            "maximum subarray par + half-seq merge",
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
                        tp.install(|| max_sum_par_seq(&input));
                    },
                )
            },
        )
        .with_function(
            "maximum subarray par + seq merge",
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
                        tp.install(|| max_sum_par_fullseq(&input));
                    },
                )
            },
        ),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(15).warm_up_time(Duration::from_secs(1)).nresamples(1000);
                targets = maximum_subarray
}
criterion_main!(benches);
