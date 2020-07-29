#[cfg(feature = "logs")]
extern crate rayon_logs as rayon;
use rand::seq::SliceRandom;
use rand::thread_rng;
use kvik::iter_par_sort;

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
            iter_par_sort(&mut input);
        });
    }
    #[cfg(feature = "logs")]
    {
        let log = tp
            .logging_install(|| {
                iter_par_sort(&mut input);
            })
            .1;
        log.save_svg("iter_par_sort.svg")
            .expect("saving log failed");
    }
    assert_eq!(input, solution);
}
