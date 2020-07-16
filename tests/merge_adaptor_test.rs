use rand::prelude::*;
use rayon::prelude::ParallelSliceMut;
use rayon_try_fold::prelude::*;
use rayon_try_fold::Merger;

#[derive(Copy, Clone, Debug)]
struct OpaqueTuple {
    first: u64,
    second: u64,
}
unsafe impl Send for OpaqueTuple {}
unsafe impl Sync for OpaqueTuple {}
impl PartialEq for OpaqueTuple {
    fn eq(&self, other: &Self) -> bool {
        self.first == other.first
    }
}
impl Eq for OpaqueTuple {}
impl PartialOrd for OpaqueTuple {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.first.cmp(&other.first))
    }
}
impl Ord for OpaqueTuple {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.first.cmp(&other.first)
    }
}

#[test]
fn test_merge_all_unique() {
    let mut rng = rand::thread_rng();
    for size in (2..10)
        .chain(100..110)
        .chain(10_000..10_010)
        .chain(std::iter::once(1_000_000))
    {
        let mut input = (0..size).collect::<Vec<_>>();
        input.shuffle(&mut rng);
        let mid = input.len() / 2;
        let (left, right) = input.split_at_mut(mid);
        left.par_sort();
        right.par_sort();
        let mut output = (0..size).map(|_| 0).collect::<Vec<_>>();
        let merger = Merger::new(left, right, &mut output);
        merger.into_par_iter().bound_depth(4).for_each(|_| ());
        //adaptive_slice_merge(left, right, output.as_mut_slice());
        assert!(output.windows(2).all(|w| w[0] <= w[1]));
    }
}
