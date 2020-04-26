#[macro_use]
extern crate criterion;
extern crate itertools;
extern crate rand;
extern crate rayon;
extern crate rayon_try_fold;

use itertools::Itertools;
use rand::prelude::*;
use rayon::prelude::*;

use criterion::{Criterion, ParameterizedBenchmark};

struct SliceIter<'a, T: 'a> {
    slice: &'a [T],
    index: usize,
}

impl<'a, T: 'a> SliceIter<'a, T> {
    fn new(slice: &'a [T]) -> Self {
        SliceIter { slice, index: 0 }
    }
    fn completed(&self) -> bool {
        self.slice.len() <= self.index
    }
    fn peek(&self) -> &T {
        unsafe { self.slice.get_unchecked(self.index) }
    }
}

impl<'a, T: 'a> Iterator for SliceIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.completed() {
            None
        } else {
            let index = self.index;
            self.index += 1;
            Some(unsafe { self.slice.get_unchecked(index) })
        }
    }
}

struct MergeIter<'a, T: 'a> {
    left: SliceIter<'a, T>,
    right: SliceIter<'a, T>,
}

impl<'a, T: 'a> MergeIter<'a, T> {
    fn new(left: SliceIter<'a, T>, right: SliceIter<'a, T>) -> Self {
        MergeIter { left, right }
    }
}

impl<'a, T: 'a + Ord> Iterator for MergeIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left.completed() {
            self.right.next()
        } else if self.right.completed() {
            self.left.next()
        } else if self.left.peek() <= self.right.peek() {
            self.left.next()
        } else {
            self.right.next()
        }
    }
}

fn safe_manual_merge(left: &[u32], right: &[u32], output: &mut [u32]) {
    let mut left_index = 0;
    let mut right_index = 0;
    for o in output {
        if left_index >= left.len() {
            *o = right[right_index];
            right_index += 1;
        } else if right_index >= right.len() {
            *o = left[left_index];
            left_index += 1;
        } else if left[left_index] <= right[right_index] {
            *o = left[left_index];
            left_index += 1;
        } else {
            *o = right[right_index];
            right_index += 1;
        };
    }
}

fn unsafe_manual_merge(left: &[u32], right: &[u32], output: &mut [u32]) {
    let mut left_index = 0;
    let mut right_index = 0;
    for o in output {
        unsafe {
            if left_index >= left.len() {
                *o = *right.get_unchecked(right_index);
                right_index += 1;
            } else if right_index >= right.len() {
                *o = *left.get_unchecked(left_index);
                left_index += 1;
            } else if left.get_unchecked(left_index) <= right.get_unchecked(right_index) {
                *o = *left.get_unchecked(left_index);
                left_index += 1;
            } else {
                *o = *right.get_unchecked(right_index);
                right_index += 1;
            };
        }
    }
}

fn manual_slice_iter(left: &[u32], right: &[u32], output: &mut [u32]) {
    let mut left_iter = SliceIter::new(left);
    let mut right_iter = SliceIter::new(right);
    for o in output {
        if left_iter.completed() {
            *o = *right_iter.next().unwrap();
        } else if right_iter.completed() {
            *o = *left_iter.next().unwrap();
        } else if left_iter.peek() <= right_iter.peek() {
            *o = *left_iter.next().unwrap(); // TODO: remove unwrap ?
        } else {
            *o = *right_iter.next().unwrap();
        };
    }
}

fn manual_merge_iter(left: &[u32], right: &[u32], output: &mut [u32]) {
    let left_iter = SliceIter::new(left);
    let right_iter = SliceIter::new(right);
    let merge_iter = MergeIter::new(left_iter, right_iter);
    output.iter_mut().zip(merge_iter).for_each(|(o, i)| *o = *i);
}

//TODO: this will be very bad if one block ends up being small
// we should fall back to another algorithm in this case
fn safe_very_manual_merge(left: &[u32], right: &[u32], mut output: &mut [u32]) {
    let mut left_index = 0;
    let mut right_index = 0;
    loop {
        let remaining_left_size = left.len() - left_index;
        let remaining_right_size = right.len() - right_index;
        let block_size = std::cmp::min(remaining_left_size, remaining_right_size);
        if block_size == 0 {
            break;
        }
        output[..block_size].iter_mut().for_each(|o| {
            if left[left_index] <= right[right_index] {
                *o = left[left_index];
                left_index += 1;
            } else {
                *o = right[right_index];
                right_index += 1;
            }
        });
        output = &mut output[block_size..];
    }
    if left_index != left.len() {
        output.copy_from_slice(&left[left_index..])
    } else {
        output.copy_from_slice(&right[right_index..])
    }
}

struct Merger<'a, T> {
    a: &'a [T],
    b: &'a [T],
    a_index: usize,
    b_index: usize,
    out: &'a mut [T],
    out_index: usize,
}
impl<'a, T: Copy + std::cmp::Ord> Merger<'a, T> {
    #[inline]
    fn copy_from_a(&mut self, amount: usize) {
        //PRECONDITION amount <= a.len() - a_index
        //PRECONDITION amount <= out.len() - out_index
        self.out[self.out_index..self.out_index + amount]
            .copy_from_slice(&self.a[self.a_index..self.a_index + amount]);
        self.out_index += amount;
        self.a_index += amount;
    }
    #[inline]
    fn copy_from_b(&mut self, amount: usize) {
        //PRECONDITION amount <= b.len() - b_index
        //PRECONDITION amount <= out.len() - out_index
        self.out[self.out_index..self.out_index + amount]
            .copy_from_slice(&self.b[self.b_index..self.b_index + amount]);
        self.out_index += amount;
        self.b_index += amount;
    }

    pub fn manual_merge(&mut self, limit: usize) {
        let out_len = self.out.len();
        let to_do = std::cmp::min(out_len - self.out_index, limit);
        if self.a_index >= self.a.len() {
            self.copy_from_b(to_do);
            return;
        }
        if self.b_index >= self.b.len() {
            self.copy_from_a(to_do);
            return;
        }
        let a_remaining = self.a.len() - self.a_index;
        if self.a[self.a_index + std::cmp::min(to_do, a_remaining - 1)] <= self.b[self.b_index] {
            self.copy_from_a(std::cmp::min(to_do, a_remaining));
            if to_do > a_remaining {
                self.copy_from_b(to_do - a_remaining);
            }
            return;
        }
        let b_remaining = self.b.len() - self.b_index;
        if self.b[self.b_index + std::cmp::min(to_do, b_remaining - 1)] < self.a[self.a_index] {
            let b_remaining = self.b.len() - self.b_index;
            self.copy_from_b(std::cmp::min(to_do, b_remaining));
            if to_do > b_remaining {
                self.copy_from_a(to_do - b_remaining);
            }
            return;
        }

        let mut left_index = self.a_index;
        let mut right_index = self.b_index;
        let left_len = self.a.len();
        let right_len = self.b.len();
        let left = self.a;
        let right = self.b;
        let out = &mut self.out[self.out_index..self.out_index + to_do];

        for o in out {
            unsafe {
                if left_index >= left_len {
                    *o = *right.get_unchecked(right_index);
                    right_index += 1;
                } else if right_index >= right_len {
                    *o = *left.get_unchecked(left_index);
                    left_index += 1;
                } else if left.get_unchecked(left_index) <= right.get_unchecked(right_index) {
                    *o = *left.get_unchecked(left_index);
                    left_index += 1;
                } else {
                    *o = *right.get_unchecked(right_index);
                    right_index += 1;
                }
            }
        }
        self.a_index = left_index;
        self.b_index = right_index;
        self.out_index += to_do;
    }
}
fn trivial_input(input_size: u32) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let left = (0..input_size / 2).collect::<Vec<_>>();
    let right = (input_size / 2..input_size).collect::<Vec<_>>();
    let output = vec![0u32; input_size as usize];
    (left, right, output)
}

fn interleaved_input(input_size: u32) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let (left, right): (Vec<_>, Vec<_>) = (0..input_size).tuples().unzip();
    let output = vec![0u32; input_size as usize];
    (left, right, output)
}

fn random_cuts(input_size: u32) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let mut orig = (0..input_size).collect::<Vec<_>>();
    let mut rng = thread_rng();
    orig.shuffle(&mut rng);
    let cutting_point: usize = rng.gen::<usize>() % (input_size as usize);
    let (left, right) = orig.split_at_mut(cutting_point);
    left.par_sort();
    right.par_sort();
    let output = vec![0u32; input_size as usize];
    (left.to_vec(), right.to_vec(), output)
}

fn random_equal_input(input_size: u32) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let mut rng = thread_rng();
    let mut orig = (0..input_size).collect::<Vec<_>>();
    // Suck it, Newton's method!
    let root_n = match orig.binary_search_by(|x| (x * x).cmp(&input_size)) {
        Ok(root) => root as u32,
        Err(ceil_root) => ceil_root.saturating_sub(1) as u32,
    };

    orig.iter_mut().for_each(|num| {
        *num = *num % root_n;
    });
    orig.shuffle(&mut rng);
    let (left, right) = orig.split_at_mut(input_size as usize / 2);
    left.par_sort();
    right.par_sort();
    let output = vec![0u32; input_size as usize];
    (left.to_vec(), right.to_vec(), output)
}

fn all_equal_input(input_size: u32) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let mut orig = std::iter::repeat(42)
        .take(input_size as usize)
        .collect::<Vec<_>>();
    let (left, right) = orig.split_at_mut(input_size as usize / 2);
    left.par_sort();
    right.par_sort();
    let output = vec![0u32; input_size as usize];
    (left.to_vec(), right.to_vec(), output)
}

fn merge_benchmarks(c: &mut Criterion) {
    let sizes: Vec<u32> = vec![100_000, 300_000, 900_000, 2_700_000, 8_100_000, 24_300_000];
    //let sizes: Vec<u32> = vec![100_000]; //, 300_000, 900_000, 2_700_000, 8_100_000, 24_300_000];
    c.bench(
        "merge interleaved input",
        ParameterizedBenchmark::new(
            "itertool merge",
            |b, input_size| {
                b.iter_with_setup(
                    || interleaved_input(*input_size),
                    |(left, right, mut output)| {
                        left.iter()
                            .merge(right.iter())
                            .zip(output.iter_mut())
                            .for_each(|(i, o)| *o = *i);
                        (left, right, output)
                    },
                )
            },
            sizes.clone(),
        )
        .with_function("manual slice iter", |b, input_size| {
            b.iter_with_setup(
                || interleaved_input(*input_size),
                |(left, right, mut output)| {
                    manual_slice_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("manual merge iter", |b, input_size| {
            b.iter_with_setup(
                || interleaved_input(*input_size),
                |(left, right, mut output)| {
                    manual_merge_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("unsafe manual merge", |b, input_size| {
            b.iter_with_setup(
                || interleaved_input(*input_size),
                |(left, right, mut output)| {
                    unsafe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe very manual merge", |b, input_size| {
            b.iter_with_setup(
                || interleaved_input(*input_size),
                |(left, right, mut output)| {
                    safe_very_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe manual merge", |b, input_size| {
            b.iter_with_setup(
                || interleaved_input(*input_size),
                |(left, right, mut output)| {
                    safe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("artisanal merge", |b, input_size| {
            b.iter_with_setup(
                || interleaved_input(*input_size),
                |(left, right, mut output)| {
                    Merger {
                        a: &left,
                        a_index: 0,
                        b: &right,
                        b_index: 0,
                        out: &mut output,
                        out_index: 0,
                    }
                    .manual_merge(std::usize::MAX);
                    (left, right, output)
                },
            )
        }),
    );
    c.bench(
        "random cuts",
        ParameterizedBenchmark::new(
            "itertool merge",
            |b, input_size| {
                b.iter_with_setup(
                    || random_cuts(*input_size),
                    |(left, right, mut output)| {
                        left.iter()
                            .merge(right.iter())
                            .zip(output.iter_mut())
                            .for_each(|(i, o)| *o = *i);
                        (left, right, output)
                    },
                )
            },
            sizes.clone(),
        )
        .with_function("manual slice iter", |b, input_size| {
            b.iter_with_setup(
                || random_cuts(*input_size),
                |(left, right, mut output)| {
                    manual_slice_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("manual merge iter", |b, input_size| {
            b.iter_with_setup(
                || random_cuts(*input_size),
                |(left, right, mut output)| {
                    manual_merge_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("unsafe manual merge", |b, input_size| {
            b.iter_with_setup(
                || random_cuts(*input_size),
                |(left, right, mut output)| {
                    unsafe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe very manual merge", |b, input_size| {
            b.iter_with_setup(
                || random_cuts(*input_size),
                |(left, right, mut output)| {
                    safe_very_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe manual merge", |b, input_size| {
            b.iter_with_setup(
                || random_cuts(*input_size),
                |(left, right, mut output)| {
                    safe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("artisanal merge", |b, input_size| {
            b.iter_with_setup(
                || random_cuts(*input_size),
                |(left, right, mut output)| {
                    Merger {
                        a: &left,
                        a_index: 0,
                        b: &right,
                        b_index: 0,
                        out: &mut output,
                        out_index: 0,
                    }
                    .manual_merge(std::usize::MAX);
                    (left, right, output)
                },
            )
        }),
    );
    c.bench(
        "square root repetition",
        ParameterizedBenchmark::new(
            "itertool merge",
            |b, input_size| {
                b.iter_with_setup(
                    || random_equal_input(*input_size),
                    |(left, right, mut output)| {
                        left.iter()
                            .merge(right.iter())
                            .zip(output.iter_mut())
                            .for_each(|(i, o)| *o = *i);
                        (left, right, output)
                    },
                )
            },
            sizes.clone(),
        )
        .with_function("manual slice iter", |b, input_size| {
            b.iter_with_setup(
                || random_equal_input(*input_size),
                |(left, right, mut output)| {
                    manual_slice_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("manual merge iter", |b, input_size| {
            b.iter_with_setup(
                || random_equal_input(*input_size),
                |(left, right, mut output)| {
                    manual_merge_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("unsafe manual merge", |b, input_size| {
            b.iter_with_setup(
                || random_equal_input(*input_size),
                |(left, right, mut output)| {
                    unsafe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe very manual merge", |b, input_size| {
            b.iter_with_setup(
                || random_equal_input(*input_size),
                |(left, right, mut output)| {
                    safe_very_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe manual merge", |b, input_size| {
            b.iter_with_setup(
                || random_equal_input(*input_size),
                |(left, right, mut output)| {
                    safe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("artisanal merge", |b, input_size| {
            b.iter_with_setup(
                || random_equal_input(*input_size),
                |(left, right, mut output)| {
                    Merger {
                        a: &left,
                        a_index: 0,
                        b: &right,
                        b_index: 0,
                        out: &mut output,
                        out_index: 0,
                    }
                    .manual_merge(std::usize::MAX);
                    (left, right, output)
                },
            )
        }),
    );
    c.bench(
        "all equal input",
        ParameterizedBenchmark::new(
            "itertool merge",
            |b, input_size| {
                b.iter_with_setup(
                    || all_equal_input(*input_size),
                    |(left, right, mut output)| {
                        left.iter()
                            .merge(right.iter())
                            .zip(output.iter_mut())
                            .for_each(|(i, o)| *o = *i);
                        (left, right, output)
                    },
                )
            },
            sizes.clone(),
        )
        .with_function("manual slice iter", |b, input_size| {
            b.iter_with_setup(
                || all_equal_input(*input_size),
                |(left, right, mut output)| {
                    manual_slice_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("manual merge iter", |b, input_size| {
            b.iter_with_setup(
                || all_equal_input(*input_size),
                |(left, right, mut output)| {
                    manual_merge_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("unsafe manual merge", |b, input_size| {
            b.iter_with_setup(
                || all_equal_input(*input_size),
                |(left, right, mut output)| {
                    unsafe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe very manual merge", |b, input_size| {
            b.iter_with_setup(
                || all_equal_input(*input_size),
                |(left, right, mut output)| {
                    safe_very_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe manual merge", |b, input_size| {
            b.iter_with_setup(
                || all_equal_input(*input_size),
                |(left, right, mut output)| {
                    safe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("artisanal merge", |b, input_size| {
            b.iter_with_setup(
                || all_equal_input(*input_size),
                |(left, right, mut output)| {
                    Merger {
                        a: &left,
                        a_index: 0,
                        b: &right,
                        b_index: 0,
                        out: &mut output,
                        out_index: 0,
                    }
                    .manual_merge(std::usize::MAX);
                    (left, right, output)
                },
            )
        }),
    );
    c.bench(
        "trivial input",
        ParameterizedBenchmark::new(
            "itertool merge",
            |b, input_size| {
                b.iter_with_setup(
                    || trivial_input(*input_size),
                    |(left, right, mut output)| {
                        left.iter()
                            .merge(right.iter())
                            .zip(output.iter_mut())
                            .for_each(|(i, o)| *o = *i);
                        (left, right, output)
                    },
                )
            },
            sizes.clone(),
        )
        .with_function("manual slice iter", |b, input_size| {
            b.iter_with_setup(
                || trivial_input(*input_size),
                |(left, right, mut output)| {
                    manual_slice_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("manual merge iter", |b, input_size| {
            b.iter_with_setup(
                || trivial_input(*input_size),
                |(left, right, mut output)| {
                    manual_merge_iter(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("unsafe manual merge", |b, input_size| {
            b.iter_with_setup(
                || trivial_input(*input_size),
                |(left, right, mut output)| {
                    unsafe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe very manual merge", |b, input_size| {
            b.iter_with_setup(
                || trivial_input(*input_size),
                |(left, right, mut output)| {
                    safe_very_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("safe manual merge", |b, input_size| {
            b.iter_with_setup(
                || trivial_input(*input_size),
                |(left, right, mut output)| {
                    safe_manual_merge(&left, &right, &mut output);
                    (left, right, output)
                },
            )
        })
        .with_function("artisanal merge", |b, input_size| {
            b.iter_with_setup(
                || trivial_input(*input_size),
                |(left, right, mut output)| {
                    Merger {
                        a: &left,
                        a_index: 0,
                        b: &right,
                        b_index: 0,
                        out: &mut output,
                        out_index: 0,
                    }
                    .manual_merge(std::usize::MAX);
                    (left, right, output)
                },
            )
        }),
    );
}

criterion_group! {
    name = benches;
            config = Criterion::default().sample_size(50).nresamples(100);
                targets = merge_benchmarks
}
criterion_main!(benches);
