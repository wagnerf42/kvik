#[cfg(feature = "logs")]
extern crate rayon_logs as rayon;

mod blocked;
mod join_policy;
pub use blocked::Blocked;
mod adaptive;
pub use adaptive::work;
mod algorithms;
pub use algorithms::manual_merge::adaptive_slice_merge;
mod even_levels;
mod map;
pub mod prelude;
mod range;
mod rayon_policy;
mod sequential;
mod slice;
pub(crate) mod small_channel;
pub(crate) mod traits;
pub mod utils;
mod wrap;
// TODO: change crate name

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    #[test]
    fn reduce_range() {
        let s = (0u64..10).into_par_iter().reduce(|| 0, |a, b| a + b);
        assert_eq!(s, 45)
    }
    #[test]
    fn reduce_mapped_range() {
        let s = (0u64..10)
            .into_par_iter()
            .map(|i| i + 1)
            .reduce(|| 0, |a, b| a + b);
        assert_eq!(s, 55)
    }
    #[test]
    fn slice_sum_reduce() {
        let a = [1, 3, 2, 4];
        let ten = a.par_iter().map(|r| *r).reduce(|| 0, |a, b| a + b);
        assert_eq!(10, ten);
    }
    #[test]
    fn even_levels_test() {
        //This does not return 1, ie, it does not end at an even level
        (0u64..10u64)
            .into_par_iter()
            .map(|_| 1u64)
            .even_levels()
            .reduce(
                || 0u64,
                |left, right| {
                    if left != right {
                        assert!(left == 0u64 || right == 0u64);
                        if std::cmp::max(left, right) == 2u64 {
                            1u64
                        } else {
                            2u64
                        }
                    } else {
                        if left == 1u64 || left == 0u64 {
                            2u64
                        } else {
                            1u64
                        }
                    }
                },
            );
    }
    #[test]
    fn join_policy_test() {
        const JP_SIZE: usize = 10;
        (0u64..1000u64)
            .wrap_iter()
            .map(|chunk| {
                assert!(chunk.end - chunk.start >= JP_SIZE as u64);
                chunk
            })
            .join_policy(JP_SIZE)
            .reduce(|| (0..1), |left, _| left);
    }
}
