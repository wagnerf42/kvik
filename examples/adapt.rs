#[cfg(feature = "logs")]
fn main() {
    use rayon_try_fold::prelude::*;
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .build()
        .expect("failed creating pool");
    let (_, log) = pool.logging_install(|| {
        let s = (0..100_000u64)
            .into_par_iter()
            .map(|e| e % 2)
            .adaptive()
            .reduce(|| 0, |a, b| a + b);
        assert_eq!(s, 50_000);
    });
    log.save_svg("adapt.svg")
        .expect("failed saving execution trace");
}
#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
