mod map;
pub mod prelude;
mod range;
mod sequential;
mod slice;
pub(crate) mod traits;

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
        let s = &a;
        //TODO: have a .par_iter method
        let ten = s.into_par_iter().map(|r| *r).reduce(|| 0, |a, b| a + b);
        assert_eq!(10, ten);
    }
}
