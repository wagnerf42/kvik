#[cfg(feature = "logs")]
extern crate rayon_logs as rayon;

mod blocked;
pub use blocked::Blocked;
mod adaptive;
pub use adaptive::work;
mod algorithms;
pub use algorithms::manual_merge::adaptive_slice_merge;
mod map;
pub mod prelude;
mod range;
mod rayon_policy;
mod sequential;
mod slice;
pub(crate) mod small_channel;
pub(crate) mod traits;
pub mod utils;
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
}
