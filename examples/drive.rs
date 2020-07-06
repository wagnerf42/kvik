//! test for drive and consumers

use rayon_try_fold::prelude::*;

#[cfg(feature = "logs")]
fn main() {
    use rayon_logs::ThreadPoolBuilder;
    let pool = ThreadPoolBuilder::new()
        .build()
        .expect("failed building pool");
    let (_, log) = pool.logging_install(|| {
        assert_eq!(
            (0u64..10_000)
                .into_par_iter()
                .map(|x| x + 1)
                .filter(|&x| x < 1_000)
                .rayon(3)
                .log("driving")
                .test_reduce(|| 0, |a, b| a + b),
            999 * 500
        )
    });
    log.save_svg("drive.svg").expect("failed saving svg");
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
