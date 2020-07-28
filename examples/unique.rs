//! small example counting the number of different elements
//! in a slice.
//! I would have expected unique2 to be slower because
//! the `collect` in `unique` knows the slice's size but strangely
//! the fold is way faster.
//! something is weird because on my machine i have more speedup than cores.
//!
//! maybe it IS bad for the ones who know sizes because
//! they spread data in memory for nothing.
use itertools::Itertools;
use rayon_try_fold::prelude::*;
use std::collections::HashSet;

fn unique(input: &[u32]) -> Option<HashSet<&u32>> {
    input
        .wrap_iter()
        .rayon(2)
        .map(|s| s.iter().collect::<HashSet<_>>())
        .reduce_with(|mut a, b| {
            a.extend(&b);
            a
        })
}

fn unique2(input: &[u32]) -> Option<HashSet<&u32>> {
    input
        .into_par_iter()
        .fold(HashSet::new, |mut h, e| {
            h.insert(e);
            h
        })
        .rayon(2)
        .reduce_with(|mut a, b| {
            a.extend(&b);
            a
        })
}

const SIZE: usize = 10_000_000;

fn main() {
    let v: Vec<u32> = std::iter::repeat_with(|| rand::random::<u32>() % 10_000)
        .take(SIZE)
        .collect();
    let start = std::time::Instant::now();
    let count = v.iter().unique().count();
    println!("we took in seq: {:?} (count is {})", start.elapsed(), count);
    let start = std::time::Instant::now();
    assert_eq!(count, unique(&v).map(|h| h.len()).unwrap_or(0));
    println!("we took in par: {:?}", start.elapsed());
    let start = std::time::Instant::now();
    assert_eq!(count, unique2(&v).map(|h| h.len()).unwrap_or(0));
    println!("we took in par with fold: {:?}", start.elapsed());
}
