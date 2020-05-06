#[cfg(feature = "logs")]
extern crate rayon_logs as rayon;

mod blocked;
mod join_policy;
pub use blocked::Blocked;
mod adaptive;
pub use adaptive::work;
mod algorithms;
pub use algorithms::iter_sort::iter_par_sort;
pub use algorithms::manual_merge::adaptive_slice_merge;
pub use algorithms::slice_merge_sort::slice_par_sort;
mod composed;
mod even_levels;
mod join_context_policy;
mod macro_blocks;
mod map;
mod merge;
pub mod prelude;
mod range;
mod rayon_policy;
mod sequential;
mod slice;
pub(crate) mod small_channel;
pub(crate) mod traits;
pub mod utils;
mod wrap;
mod zip;
// TODO: change crate name
#[macro_use]
mod private;

pub(crate) use private_try::Try;

/// We hide the `Try` trait in a private module, as it's only meant to be a
/// stable clone of the standard library's `Try` trait, as yet unstable.
/// this snippet is taken directly from rayon.
mod private_try {
    /// Clone of `std::ops::Try`.
    ///
    /// Implementing this trait is not permitted outside of `rayon`.
    pub trait Try {
        private_decl! {}

        type Ok;
        type Error;
        fn into_result(self) -> Result<Self::Ok, Self::Error>;
        fn from_ok(v: Self::Ok) -> Self;
        fn from_error(v: Self::Error) -> Self;
    }

    impl<T> Try for Option<T> {
        private_impl! {}

        type Ok = T;
        type Error = ();

        fn into_result(self) -> Result<T, ()> {
            self.ok_or(())
        }
        fn from_ok(v: T) -> Self {
            Some(v)
        }
        fn from_error(_: ()) -> Self {
            None
        }
    }

    impl<T, E> Try for Result<T, E> {
        private_impl! {}

        type Ok = T;
        type Error = E;

        fn into_result(self) -> Result<T, E> {
            self
        }
        fn from_ok(v: T) -> Self {
            Ok(v)
        }
        fn from_error(v: E) -> Self {
            Err(v)
        }
    }
}

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
        assert!(!(0u64..100u64)
            .wrap_iter()
            .map(|_| true)
            .even_levels()
            .reduce(
                || true,
                |left, right| {
                    assert_eq!(left, right);
                    !left
                }
            ));
    }
    #[test]
    fn join_policy_test() {
        //TODO this test should have a lower and an upper bound on the base size
        const JP_SIZE: u32 = 3;
        const PROBLEM_SIZE: u64 = 1000;
        (0u64..PROBLEM_SIZE)
            .wrap_iter()
            .map(|chunk| {
                assert_eq!(
                    chunk.end - chunk.start,
                    PROBLEM_SIZE / 2u32.pow(JP_SIZE) as u64
                );
                chunk
            })
            .join_policy(JP_SIZE)
            .reduce(|| (0..1), |left, _| left);
    }
}
