use itertools::Itertools;
use kvik::prelude::*;

const SIZE: usize = 100_000_000;

fn palindrome(s: &str) -> bool {
    let l = s.len();
    s.chars()
        .take(l / 2)
        .zip(s.chars().rev())
        .all(|(a, b)| a == b)
}

#[cfg(feature = "logs")]
fn main() {
    let input: Vec<String> = lipsum::lipsum(SIZE)
        .split_whitespace()
        .map(|w| w.to_string())
        .filter(|w| w.len() > 4)
        .collect();
    let start = std::time::Instant::now();
    let res = input.iter().find(|&w| palindrome(w));
    println!("took {:?}", start.elapsed());
    let pool = rayon_logs::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    pool.compare()
        .runs_number(5)
        .attach_algorithm("no blocks", || {
            assert_eq!(
                input
                    .par_iter()
                    .filter(|&w| palindrome(w))
                    .next()
                    .rayon(2)
                    .reduce_with(|a, _| a),
                res
            );
        })
        .attach_algorithm("blocks", || {
            assert_eq!(
                input
                    .par_iter()
                    .filter(|&w| palindrome(w))
                    .next()
                    .by_blocks(std::iter::successors(Some(2), |old| Some(2 * old)))
                    .rayon(2)
                    .reduce_with(|a, _| a),
                res
            );
        })
        .generate_logs("find.html")
        .unwrap();
}

#[cfg(not(feature = "logs"))]
fn main() {
    println!("you should run me with the logs feature");
}
