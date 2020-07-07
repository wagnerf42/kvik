use rand::Rng;
use std::iter::{once, repeat, repeat_with};

fn random_vec(outer_size: usize, inner_size: usize) -> Vec<Vec<u64>> {
    let mut rng = rand::thread_rng();
    repeat_with(|| {
        repeat_with(|| rng.gen::<u64>() % 10)
            .take(inner_size)
            .collect()
    })
    .take(outer_size)
    .collect()
}

fn problematic_vec(outer_size: usize, small_size: usize, big_size: usize) -> Vec<Vec<u64>> {
    let mut rng = rand::thread_rng();
    repeat(small_size)
        .take(outer_size - 1)
        .chain(once(big_size))
        .map(|size| repeat_with(|| rng.gen::<u64>() % 10).take(size).collect())
        .collect()
}

fn seq(vec: &Vec<Vec<u64>>) -> usize {
    vec.iter()
        .map(|v| ((v.iter().sum::<u64>() + 1) % 2) as usize)
        .sum()
}

#[cfg(feature = "logs")]
fn composed(vec: &Vec<Vec<u64>>) -> usize {
    use rayon_try_fold::prelude::*;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    vec.par_iter()
        .map(|v: &Vec<u64>| {
            let sum = v
                .par_iter()
                .fold(|| 0u64, |a, b| a + b)
                .rayon(limit)
                .composed()
                .reduce(|| 0, |a, b| a + b);
            ((sum + 1) % 2) as usize
        })
        .fold(|| 0, |a, b| a + b)
        .rayon(limit)
        .composed()
        .log("outer")
        .reduce(|| 0, |a, b| a + b)
}

#[cfg(feature = "logs")]
fn composed_counter(vec: &Vec<Vec<u64>>) -> usize {
    use rayon_try_fold::prelude::*;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    vec.par_iter()
        .map(|v: &Vec<u64>| {
            let sum = v
                .par_iter()
                .fold(|| 0u64, |a, b| a + b)
                .rayon(limit)
                .composed_counter(2 * threads)
                .reduce(|| 0, |a, b| a + b);
            ((sum + 1) % 2) as usize
        })
        .fold(|| 0, |a, b| a + b)
        .rayon(limit)
        .composed_counter(2 * threads)
        .log("outer")
        .reduce(|| 0, |a, b| a + b)
}

#[cfg(feature = "logs")]
fn composed_task(vec: &Vec<Vec<u64>>) -> usize {
    use rayon_try_fold::prelude::*;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    vec.par_iter()
        .map(|v: &Vec<u64>| {
            let sum = v
                .par_iter()
                .fold(|| 0u64, |a, b| a + b)
                .rayon(limit)
                .composed()
                .reduce(|| 0, |a, b| a + b);
            ((sum + 1) % 2) as usize
        })
        .fold(|| 0, |a, b| a + b)
        .rayon(limit)
        .composed_task()
        .log("outer")
        .reduce(|| 0, |a, b| a + b)
}

#[cfg(feature = "logs")]
fn rayon_both(vec: &Vec<Vec<u64>>) -> usize {
    use rayon_try_fold::prelude::*;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    vec.par_iter()
        .map(|v: &Vec<u64>| {
            let sum = v
                .par_iter()
                .fold(|| 0u64, |a, b| a + b)
                .rayon(limit)
                .reduce(|| 0, |a, b| a + b);
            ((sum + 1) % 2) as usize
        })
        .fold(|| 0, |a, b| a + b)
        .rayon(limit)
        .reduce(|| 0, |a, b| a + b)
}

#[cfg(feature = "logs")]
fn rayon_outer(vec: &Vec<Vec<u64>>) -> usize {
    use rayon_try_fold::prelude::*;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    vec.par_iter()
        .map(|v: &Vec<u64>| {
            let sum: u64 = v.iter().fold(0, |a, b| a + b);
            ((sum + 1) % 2) as usize
        })
        .fold(|| 0, |a, b| a + b)
        .rayon(limit)
        .log("outer")
        .reduce(|| 0, |a, b| a + b)
}

#[cfg(feature = "logs")]
fn rayon_outer_sum(vec: &Vec<Vec<u64>>) -> usize {
    use rayon_try_fold::prelude::*;

    let threads = rayon::current_num_threads();
    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    vec.par_iter()
        .map(|v: &Vec<u64>| {
            let sum: u64 = v.iter().sum();
            ((sum + 1) % 2) as usize
        })
        .fold(|| 0, |a, b| a + b)
        .rayon(limit)
        .log("outer")
        .reduce(|| 0, |a, b| a + b)
}
#[cfg(feature = "logs")]
fn composed_size(vec: &Vec<Vec<u64>>) -> usize {
    use rayon_try_fold::prelude::*;

    let threads = rayon::current_num_threads();

    let limit = (((threads as f64).log(2.0).ceil()) as usize) + 1;

    vec.par_iter()
        .map(|v: &Vec<u64>| {
            let sum = v
                .par_iter()
                .fold(|| 0u64, |a, b| a + b)
                .composed_size(limit)
                .reduce(|| 0, |a, b| a + b);
            ((sum + 1) % 2) as usize
        })
        .fold(|| 0, |a, b| a + b)
        .composed_size(limit)
        .log("outer")
        .reduce(|| 0, |a, b| a + b)
}

#[cfg(feature = "logs")]
fn main() {
    use std::time::Instant;

    let threads = 2;
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("Failed to create thread pool");

    /*
    let (_, log) = pool.logging_install(|| {
        let res = composed(&vec);
        assert_eq!(expected, res);
    });
    log.save_svg("even_sum.svg").expect("Failed to save svg");
     */

    let outer = 5;
    let inner = 100_000;

    pool.compare()
        .runs_number(20)
        .attach_algorithm_with_setup(
            "rayon_outer",
            || {
                let vec = random_vec(outer, inner);
                vec
            },
            |v| {
                rayon_outer(&v);
                v
            },
        )
        .attach_algorithm_with_setup(
            "composed_size",
            || {
                let vec = random_vec(outer, inner);
                vec
            },
            |v| {
                composed_size(&v);
                v
            },
        )
        .attach_algorithm_with_setup(
            "composed_task",
            || {
                let vec = random_vec(outer, inner);
                vec
            },
            |v| {
                composed_task(&v);
                v
            },
        )
        .generate_logs("log.html")
        .expect("failed to generate logs");
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
