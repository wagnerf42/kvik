mod map;
pub mod prelude;
mod range;
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
}
