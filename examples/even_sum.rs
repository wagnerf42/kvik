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
    use rayon::prelude::*;

    vec.par_iter()
        .map(|v| ((v.par_iter().sum::<u64>() + 1) % 2) as usize)
        .sum()
}

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

fn rayon_outer(vec: &Vec<Vec<u64>>) -> usize {
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
        .reduce(|| 0, |a, b| a + b)
}

#[cfg(feature = "logs")]
fn main() {
    let threads = 4;
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("Failed to create thread pool");

    let vec = problematic_vec(1000, 1000, 10000000);
    let expected = seq(&vec);

    /*
    let (_, log) = pool.logging_install(|| {
        let res = composed(&vec);
        assert_eq!(expected, res);
    });
    log.save_svg("even_sum.svg").expect("Failed to save svg");
     */

    pool
	.compare()
	.runs_number(10)
	.attach_algorithm_with_setup("without counter", || {
	    let vec = problematic_vec(1000, 1000, 1000000);
	    let expected = seq(&vec);
	    (vec, expected)
	}, |(v, e)| {
	    let res = composed(&v);
	    assert_eq!(e, res);
	})
	.attach_algorithm_with_setup("with counter", || {
	    let vec = problematic_vec(1000, 1000, 1000000);
	    let expected = seq(&vec);
	    (vec, expected)
	}, |(v, e)| {
	    let res = composed_counter(&v);
	    assert_eq!(e, res);
	})
	
	.generate_logs("log.html").expect("failed to generate logs");
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
