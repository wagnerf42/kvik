// This uses rayon logs to take a closer look into the very manual slice sort
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon_try_fold::slice_par_sort;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        panic!("please enter problem_size num_threads task_cap as the three command line args for this");
    }
    let problem_size: u32 = args[1].parse().unwrap();
    let num_threads: usize = args[2].parse().unwrap();
    let fusion_task_cap: isize = args[3].parse().unwrap();

    #[cfg(feature = "logs")]
    {
        let thread_pool = rayon_logs::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .expect("No thread pool for you");
        thread_pool
            .compare()
            .attach_algorithm_nodisplay_with_setup(
                "slice par sort",
                || {
                    let mut input = (0..problem_size).collect::<Vec<u32>>();
                    input.shuffle(&mut thread_rng());
                    input
                },
                |mut v| {
                    slice_par_sort(&mut v, fusion_task_cap);
                    v
                },
            )
            .generate_logs(format!(
                "jccap_{}_{}_{}.html",
                problem_size, num_threads, fusion_task_cap
            ))
            .expect("No logs for you");
    }
    #[cfg(not(feature = "logs"))]
    {
        println!("~Hello~ I can't see anything world!");
    }
}
