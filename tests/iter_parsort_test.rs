use rand::random;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use rayon_try_fold::iter_par_sort;

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
fn test_iter_sort() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    let mut rng = rand::thread_rng();
    for size in (2..10)
        .chain(100..110)
        .chain(10_000..10_010)
        .chain(std::iter::once(1_000_000))
    {
        let mut input = (0..size).collect::<Vec<_>>();
        input.shuffle(&mut rng);
        tp.install(|| {
            iter_par_sort(&mut input);
        });
        assert!(input.par_windows(2).all(|w| w[0] <= w[1]));
    }
}

#[test]
fn test_stability_all_equal() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    for len in (2..10).chain(100..110).chain(10_000..10_010) {
        let mut v: Vec<_> = (0u64..len)
            .map(|index| OpaqueTuple {
                first: 2,
                second: index,
            })
            .collect();
        tp.install(|| {
            iter_par_sort(&mut v);
        });
        &v.windows(2).for_each(|slice_of_opaque_tuples| {
            assert!(slice_of_opaque_tuples[0].second < slice_of_opaque_tuples[1].second);
        });
    }
}
#[test]
fn test_stability_random_equal() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    for len in (2..10).chain(100..110).chain(10_000..10_010) {
        let mut v: Vec<_> = (0u64..len)
            .map(|index| OpaqueTuple {
                first: random::<u64>() % 3,
                second: index,
            })
            .collect();
        tp.install(|| {
            iter_par_sort(&mut v);
        });
        &v.windows(2).for_each(|slice_of_opaque_tuples| {
            assert!(
                slice_of_opaque_tuples[0].first < slice_of_opaque_tuples[1].first
                    || (slice_of_opaque_tuples[0].first == slice_of_opaque_tuples[1].first
                        && slice_of_opaque_tuples[0].second < slice_of_opaque_tuples[1].second)
            );
        });
    }
}
