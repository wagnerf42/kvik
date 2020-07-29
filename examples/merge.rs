use rand::prelude::*;
#[cfg(not(feature = "logs"))]
use rayon::prelude::*;
#[cfg(feature = "logs")]
use rayon_logs::{prelude::*, ThreadPoolBuilder};

const SIZE: u32 = 10_000_000;

fn main() {
    #[cfg(not(feature = "logs"))]
    {
        use kvik::adaptive_slice_merge;
        let mut rng = rand::thread_rng();
        let mut input = (0..SIZE / 5)
            .cycle()
            .take(SIZE as usize)
            .collect::<Vec<_>>();
        input.shuffle(&mut rng);
        let mid = input.len() / 2;
        let (left, right) = input.split_at_mut(mid);
        left.par_sort();
        right.par_sort();
        let mut output = (0..SIZE).map(|_| 0).collect::<Vec<_>>();
        adaptive_slice_merge(left, right, output.as_mut_slice());
        assert!(output.windows(2).all(|w| w[0] <= w[1]));
    }
    #[cfg(feature = "logs")]
    {
        use kvik::prelude::ParallelIterator;
        use kvik::{prelude::IntoParallelIterator, Merger};
        let mut rng = rand::thread_rng();
        let mut input = (0..SIZE).collect::<Vec<_>>();
        input.shuffle(&mut rng);
        let mid = input.len() / 2;
        let (left, right) = input.split_at_mut(mid);
        left.par_sort();
        right.par_sort();
        let mut output = (0..SIZE).map(|_| 0).collect::<Vec<_>>();
        let merger = Merger::new(left, right, &mut output);
        let tp = ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("Couldn't make thread pool");
        tp.logging_install(|| {
            merger.into_par_iter().bound_depth(2).for_each(|_| ());
        })
        .1
        .save_svg("merge_adaptor.svg")
        .unwrap();
        //adaptive_slice_merge(left, right, output.as_mut_slice());
        assert!(output.windows(2).all(|w| w[0] <= w[1]));
    }
}
