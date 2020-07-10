#[cfg(feature = "logs")]
use rayon_logs::ThreadPoolBuilder;
use rayon_try_fold::prelude::*;

const PROBLEM_SIZE: u64 = 1_000_000;
fn main() {
    #[cfg(feature = "logs")]
    {
        let inp: Vec<_> = (0u64..PROBLEM_SIZE).collect();
        let tp = ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("Thread pool builder failed");
        let (sum, log) = tp.logging_install(|| {
            inp.par_iter()
                .join_context_policy(1)
                .force_depth(1)
                .bound_depth(6)
                .map(|r| *r)
                .reduce(|| 0, |left, right| left + right)
        });
        log.save_svg("join_context_policy.svg");
        assert_eq!(sum, 499999500000u64);
    }
    #[cfg(not(feature = "logs"))]
    {
        println!("You have to see it to believe it");
    }
}
