use itertools::Itertools;
use kvik::prelude::*;

const SIZE: usize = 100_000_000;
const TARGET: usize = 40_000_000;

#[cfg(feature = "logs")]
fn main() {
    let input: Vec<_> = (0..SIZE).collect();
    let start = std::time::Instant::now();
    let res = input.iter().find(|&e| *e == TARGET);
    println!("took {:?}", start.elapsed());
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let (_, log) = pool.logging_install(|| {
        assert_eq!(
                input
                    .par_iter()
                    .filter(|&e| *e == TARGET)
                    .next()
                    .log("filter")
                    .by_blocks(std::iter::successors(Some(200_000), |old| Some(2 * old)))
                    .rayon(2)
                    .reduce_with(|a, _| a),
                res
            );
    });
    log.save_svg("find_example.svg").unwrap();
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
