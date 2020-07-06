//! test for drive and consumers

use rayon_try_fold::prelude::*;

fn main() {
    assert_eq!(
        (0u64..10)
            .into_par_iter()
            .map(|x| x + 1)
            .test_reduce(|| 0, |a, b| a + b),
        55
    )
}
