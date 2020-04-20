use rand::prelude::*;
use rayon_try_fold::prelude::*;
use rayon_try_fold::work;

const SIZE: u64 = 10_000_000;

struct Merger<'a> {
    a: &'a [u64],
    b: &'a [u64],
    a_index: usize,
    b_index: usize,
    out: &'a mut [u64],
    out_index: usize,
}

fn main() {
    let mut rng = rand::thread_rng();
    let mut input = (0..SIZE).collect::<Vec<_>>();
    let mid = input.len() / 2;
    let (left, right) = input.split_at_mut(mid);
    left.shuffle(&mut rng);
    right.shuffle(&mut rng);
    let mut output = (0..SIZE).map(|_| 0).collect::<Vec<_>>();
    let merger = Merger {
        a: left,
        b: right,
        a_index: 0,
        b_index: 0,
        out: output.as_mut_slice(),
        out_index: 0,
    };
    work(
        merger,
        |m| m.out_index == m.out.len(),
        |m| unimplemented!("divide"),
        |m, s| unimplemented!("work by s"),
    );
    assert!(output.into_iter().eq(0..SIZE))
}
