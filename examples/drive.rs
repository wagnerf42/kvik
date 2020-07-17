//! test for drive and consumers

#[cfg(feature = "logs")]
fn main() {
    use rayon_logs::ThreadPoolBuilder;
    use rayon_try_fold::prelude::*;
    let pool = ThreadPoolBuilder::new()
        .build()
        .expect("failed building pool");
    let (_, log) = pool.logging_install(|| {
        assert_eq!(
            (0u64..10_000)
                .into_par_iter()
                .map(|x| x + 1)
                .adaptive()
                .filter(|&x| x < 1_000)
                .rayon(3)
                .log("driving")
                .reduce(|| 0, |a, b| a + b),
            999 * 500
        )
    });
    log.save_svg("drive.svg").expect("failed saving svg");
    let (_, log) = pool.logging_install(|| {
        assert_eq!(
            (0u64..100)
                .into_par_iter()
                .rayon(2)
                .flat_map(|e| 0..e)
                .adaptive()
                .filter(|&x| x % 2 == 1)
                .reduce(|| 0, |a, b| a + b),
            80850
        )
    });
    log.save_svg("flat_map.svg").expect("failed saving svg");
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
