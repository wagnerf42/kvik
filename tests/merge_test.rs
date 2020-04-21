use rand::prelude::*;
use rand::random;
use rayon::prelude::*;
use rayon_try_fold::adaptive_slice_merge;

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
        adaptive_slice_merge(left, right, output.as_mut_slice());
        assert!(output.par_windows(2).all(|w| w[0] <= w[1]));
    }
}
#[test]
fn test_merge_random_equal() {
    let mut rng = rand::thread_rng();
    for size in (5..10)
        .chain(100..110)
        .chain(10_000..10_010)
        .chain(std::iter::once(1_000_000))
    {
        let mut input = (0..size / 5).cycle().take(size).collect::<Vec<_>>();
        input.shuffle(&mut rng);
        let mid = input.len() / 2;
        let (left, right) = input.split_at_mut(mid);
        left.par_sort();
        right.par_sort();
        let mut output = (0..size).map(|_| 0).collect::<Vec<_>>();
        adaptive_slice_merge(left, right, output.as_mut_slice());
        assert!(output.par_windows(2).all(|w| w[0] <= w[1]));
    }
}
#[test]
fn test_merge_all_equal() {
    for size in (5..10).chain(100..110) {
        let mut input = std::iter::repeat(42).take(size).collect::<Vec<_>>();
        let mid = input.len() / 2;
        let (left, right) = input.split_at_mut(mid);
        let mut output = (0..size).map(|_| 0).collect::<Vec<_>>();
        adaptive_slice_merge(left, right, output.as_mut_slice());
        assert!(output.par_windows(2).all(|w| w[0] <= w[1]));
    }
}
#[test]
fn test_stability_all_equal() {
    for len in (2..10)
        .chain(100..110)
        .chain(10_000..10_010)
        .chain(std::iter::once(1_000_000))
    {
        let mut input: Vec<_> = (0u64..len)
            .map(|index| OpaqueTuple {
                first: 42,
                second: index,
            })
            .collect();
        let mid = input.len() / 2;
        let (left, right) = input.split_at_mut(mid);
        let mut output = std::iter::repeat(OpaqueTuple {
            first: 0,
            second: 0,
        })
        .take(len as usize)
        .collect::<Vec<_>>();
        adaptive_slice_merge(left, right, output.as_mut_slice());
        output.par_windows(2).for_each(|slice_of_opaque_tuples| {
            assert!(slice_of_opaque_tuples[0].second < slice_of_opaque_tuples[1].second);
        });
    }
}
#[test]
fn test_stability_random_equal() {
    for len in (2..10)
        .chain(100..110)
        .chain(10_000..10_010)
        .chain(std::iter::once(1_000_000))
    {
        let mut input: Vec<_> = (0u64..len)
            .map(|index| OpaqueTuple {
                first: random::<u64>() % 3,
                second: index,
            })
            .collect();
        let mid = input.len() / 2;
        let (left, right) = input.split_at_mut(mid);
        left.par_sort();
        right.par_sort();
        let mut output = std::iter::repeat(OpaqueTuple {
            first: 0,
            second: 0,
        })
        .take(len as usize)
        .collect::<Vec<_>>();
        adaptive_slice_merge(left, right, output.as_mut_slice());
        assert!(!output
            .par_iter()
            .all(|elem| elem.first == 0 && elem.second == 0));
        output.par_windows(2).for_each(|slice_of_opaque_tuples| {
            assert!(
                slice_of_opaque_tuples[0].first < slice_of_opaque_tuples[1].first
                    || (slice_of_opaque_tuples[0].first == slice_of_opaque_tuples[1].first
                        && slice_of_opaque_tuples[0].second < slice_of_opaque_tuples[1].second)
            );
        });
    }
}
