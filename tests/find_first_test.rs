use kvik::prelude::*;
#[derive(Copy, Clone, Debug)]
struct OpaqueTuple {
    first: u64,
    second: u64,
}
unsafe impl Send for OpaqueTuple {}
unsafe impl Sync for OpaqueTuple {}

#[test]
fn test_exists() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    for size in (2..10)
        .chain(100..110)
        .chain(10_000..10_010)
        .chain(std::iter::once(1_000_000))
    {
        let input = (0..size).collect::<Vec<_>>();
        tp.install(|| {
            assert!(input
                .into_par_iter()
                .find_first(|elem| **elem == 1)
                .is_some());
        });
        tp.install(|| {
            assert!(input
                .into_par_iter()
                .by_blocks((1..).map(|elem| 2_usize.pow(elem)))
                .find_first(|elem| **elem == 1)
                .is_some());
        });
    }
}
#[test]
fn test_dne() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    for size in 1..7 {
        let input = (0..size).collect::<Vec<_>>();
        tp.install(|| {
            assert!(input
                .into_par_iter()
                .find_first(|elem| **elem == 7)
                .is_none());
        });
    }
}
#[test]
fn test_first() {
    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("Thread pool build failed");
    for len in (3..10)
        .chain(100..110)
        .chain(10_000..10_010)
        .chain(std::iter::once(1_000_000))
    {
        let v: Vec<_> = (0u64..len)
            .map(|index| OpaqueTuple {
                first: index % 3,
                second: index,
            })
            .collect();
        tp.install(|| {
            assert_eq!(
                v.into_par_iter()
                    .by_blocks((1..).map(|elem| 2_usize.pow(elem)))
                    .find_first(|elem| elem.first == 0)
                    .unwrap()
                    .first,
                0
            );
            assert_eq!(
                v.into_par_iter()
                    .by_blocks((1..).map(|elem| 2_usize.pow(elem)))
                    .find_first(|elem| elem.first == 1)
                    .unwrap()
                    .first,
                1
            );
            assert_eq!(
                v.into_par_iter()
                    .by_blocks((1..).map(|elem| 2_usize.pow(elem)))
                    .find_first(|elem| elem.first == 2)
                    .unwrap()
                    .first,
                2
            );
        });
    }
}
