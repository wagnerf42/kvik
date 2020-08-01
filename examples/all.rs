#[cfg(feature = "logs")]
fn main() {
    use kvik::prelude::*;
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .build()
        .expect("failed creating pool");
    let (_, log) = pool.logging_install(|| {
        // let's search if the range contains 4_999_999
        let s = (0..10_000_000u64)
            .into_par_iter()
            //.by_blocks(std::iter::successors(Some(2usize), |l| {
            //    Some(l.saturating_mul(2))
            //}))
            .adaptive()
            .all(|e| e != 4_999_999);
        assert!(!s)
    });
    log.save_svg("all.svg")
        .expect("failed saving execution trace");
}
#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
