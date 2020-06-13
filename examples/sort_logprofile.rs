// This uses rayon logs to take a closer look into the very manual slice sort
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon_try_fold::slice_par_sort;

const PROBLEM_SIZE: u32 = 1_000_000;
const NUM_THREADS: usize = 8;
fn main() {
    #[cfg(feature = "logs")]
    {
        let thread_pool = rayon_logs::ThreadPoolBuilder::new()
            .num_threads(NUM_THREADS)
            .build()
            .expect("No thread pool for you");
        thread_pool
            .compare()
            .attach_algorithm_nodisplay_with_setup(
                "slice par sort",
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
            .generate_logs("sort_prof.html")
            .expect("No logs for you");
    }
    #[cfg(not(feature = "logs"))]
    {
        println!("~Hello~ I can't see anything world!");
    }
}
