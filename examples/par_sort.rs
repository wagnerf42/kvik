use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::ThreadPoolBuilder;
use rayon_try_fold::slice_par_sort;

fn main() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    let mut input = (0..1_000_000).collect::<Vec<u32>>();
    input.shuffle(&mut thread_rng());
    let solution = (0..1_000_000).collect::<Vec<u32>>();
    tp.install(|| {
        slice_par_sort(&mut input);
    });
    assert_eq!(input, solution);
}
