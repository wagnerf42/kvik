//! we do a manual 'all' implementation using try_fold
#[cfg(feature = "logs")]
fn main() {
    use kvik::prelude::*;
    use rand::prelude::*;
    unimplemented!("more thinking is needed")
    //    let pool = rayon_logs::ThreadPoolBuilder::new()
    //        .build()
    //        .expect("failed creating pool");
    //    let mut input = vec![true; 10_000_000];
    //    *input.choose_mut(&mut rand::thread_rng()).unwrap() = false;
    //    let (_, log) = pool.logging_install(|| {
    //        let r = input
    //            .par_iter()
    //            .map(|e| if *e { Ok(()) } else { Err(()) })
    //            .try_fold(|| (), |_, _| Ok(()))
    //            .adaptive()
    //            .micro_block_sizes(10_000, 100_000)
    //            .try_reduce(|| (), |_, _| Ok(()));
    //        assert_eq!(r, Err(()))
    //    });
    //    log.save_svg("try_fold.svg")
    //        .expect("failed saving execution trace");
}
#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
