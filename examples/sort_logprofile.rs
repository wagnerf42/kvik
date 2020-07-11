// This uses rayon logs to take a closer look into the very manual slice sort
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon_try_fold::{
    iter_sort_jc_adaptive, iter_sort_jc_jc, iter_sort_jc_rayon, slice_par_sort,
    slice_sort_jc_adaptive_jp, slice_sort_jc_jc,
};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        panic!("please enter problem_size num_threads as the two command line args for this");
    }
    let PROBLEM_SIZE: u32 = args[1].parse().unwrap();
    let NUM_THREADS: usize = args[2].parse().unwrap();

    #[cfg(feature = "logs")]
    {
        let thread_pool = rayon_logs::ThreadPoolBuilder::new()
            .num_threads(NUM_THREADS)
            .build()
            .expect("No thread pool for you");
        thread_pool
            .compare()
            .attach_algorithm_nodisplay_with_setup(
                "slice sort JC JC",
                || {
                    let mut input = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
                    input.shuffle(&mut thread_rng());
                    input
                },
                |mut v| {
                    slice_sort_jc_jc(&mut v);
                    v
                },
            )
            .attach_algorithm_nodisplay_with_setup(
                "slice sort JC adaptive",
                || {
                    let mut input = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
                    input.shuffle(&mut thread_rng());
                    input
                },
                |mut v| {
                    slice_par_sort(&mut v);
                    v
                },
            )
            .attach_algorithm_nodisplay_with_setup(
                "slice sort JC adaptive JP",
                || {
                    let mut input = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
                    input.shuffle(&mut thread_rng());
                    input
                },
                |mut v| {
                    slice_sort_jc_adaptive_jp(&mut v);
                    v
                },
            )
            .attach_algorithm_nodisplay_with_setup(
                "iter sort JC adaptive",
                || {
                    let mut input = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
                    input.shuffle(&mut thread_rng());
                    input
                },
                |mut v| {
                    iter_sort_jc_adaptive(&mut v);
                    v
                },
            )
            .attach_algorithm_nodisplay_with_setup(
                "iter sort JC JC",
                || {
                    let mut input = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
                    input.shuffle(&mut thread_rng());
                    input
                },
                |mut v| {
                    iter_sort_jc_jc(&mut v);
                    v
                },
            )
            .attach_algorithm_nodisplay_with_setup(
                "iter sort JC rayon",
                || {
                    let mut input = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
                    input.shuffle(&mut thread_rng());
                    input
                },
                |mut v| {
                    iter_sort_jc_rayon(&mut v);
                    v
                },
            )
            .generate_logs(format!(
                "sort_policy_comparison_{}_{}.html",
                PROBLEM_SIZE, NUM_THREADS
            ))
            .expect("No logs for you");
    }
    #[cfg(not(feature = "logs"))]
    {
        println!("~Hello~ I can't see anything world!");
    }
}
