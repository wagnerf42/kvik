use rand::prelude::*;
use rayon::prelude::*;
use rayon_try_fold::adaptive_slice_merge;

const SIZE: u64 = 1_000_000;

fn main() {
    let mut rng = rand::thread_rng();
    let mut input = (0..SIZE / 5)
        .cycle()
        .take(SIZE as usize)
        .collect::<Vec<_>>();
    input.shuffle(&mut rng);
    let mid = input.len() / 2;
    let (left, right) = input.split_at_mut(mid);
    left.par_sort();
    right.par_sort();
    let mut output = (0..SIZE).map(|_| 0).collect::<Vec<_>>();
    adaptive_slice_merge(left, right, output.as_mut_slice());
    assert!(output.windows(2).all(|w| w[0] <= w[1]));
}
