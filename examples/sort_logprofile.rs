// This uses rayon logs to take a closer look into the very manual slice sort
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        panic!("please enter problem_size num_threads as the two command line args for this");
    }

    #[cfg(feature = "logs")]
    {
        use rand::prelude::*;
        use kvik::slice_par_sort;
        let problem_size: u32 = args[1].parse().unwrap();
        let num_threads: usize = args[2].parse().unwrap();
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
                    input.shuffle(&mut rand::thread_rng());
                    input
                },
                |mut v| {
                    slice_par_sort(&mut v);
                    v
                },
            )
            .generate_logs(format!("jccap_{}_{}.html", problem_size, num_threads))
            .expect("No logs for you");
    }
    #[cfg(not(feature = "logs"))]
    {
        println!("~Hello~ I can't see anything world!");
    }
}
