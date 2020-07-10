use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon_try_fold::{iter_sort_jc_jc, slice_par_sort};

const PROBLEM_SIZE: u32 = 1_000_000;
fn main() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    let mut input = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
    input.shuffle(&mut thread_rng());
    let solution = (0..PROBLEM_SIZE).collect::<Vec<u32>>();
    #[cfg(not(feature = "logs"))]
    {
        tp.install(|| {
            iter_sort_jc_jc(&mut input);
        });
    }
    #[cfg(feature = "logs")]
    {
        let tp = rayon_logs::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("Thread pool build failed");
        let log = tp
            .logging_install(|| {
                iter_sort_jc_jc(&mut input);
            })
            .1;
        log.save_svg("iter_par_sort.svg")
            .expect("saving log failed");
    }
    assert_eq!(input, solution);
}
