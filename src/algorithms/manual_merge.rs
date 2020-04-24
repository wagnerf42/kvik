use crate::utils::slice_utils::{subslice_without_first_value, subslice_without_last_value};
use crate::work;

struct Merger<'a, T> {
    a: &'a [T],
    b: &'a [T],
    a_index: usize,
    b_index: usize,
    out: &'a mut [T],
    out_index: usize,
}

impl<'a, T: Copy + std::cmp::Ord> Merger<'a, T> {
    fn copy_from_a(&mut self, amount: usize) {
        //PRECONDITION amount <= a.len() - a_index
        //PRECONDITION amount <= out.len() - out_index
        self.out[self.out_index..self.out_index + amount]
            .copy_from_slice(&self.a[self.a_index..self.a_index + amount]);
        self.out_index += amount;
        self.a_index += amount;
    }
    fn copy_from_b(&mut self, amount: usize) {
        //PRECONDITION amount <= b.len() - b_index
        //PRECONDITION amount <= out.len() - out_index
        self.out[self.out_index..self.out_index + amount]
            .copy_from_slice(&self.b[self.b_index..self.b_index + amount]);
        self.out_index += amount;
        self.b_index += amount;
    }

    fn check_triviality(&self) -> bool {
        !(self.a.len() - self.a_index < 2
            || self.b.len() - self.b_index < 2
            || self.a[self.a.len() - 1] <= self.b[self.b_index]
            || self.a[self.a_index] >= self.b[self.b.len() - 1]
            || self.a[self.a_index] == self.a[self.a.len() - 1]
            || self.b[self.b_index] == self.b[self.b.len() - 1])
    }

    fn manual_merge(&mut self, limit: usize) {
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
        let mut out_index = self.out_index;
        let left_len = self.a.len();
        let right_len = self.b.len();
        let left = self.a;
        let right = self.b;
        let out = &mut self.out[..];

        for _ in 0..to_do {
            if left_index >= left_len {
                out[out_index] = right[right_index];
                out_index += 1;
                right_index += 1;
            } else if right_index >= right_len {
                out[out_index] = left[left_index];
                out_index += 1;
                left_index += 1;
            } else if left[left_index] <= right[right_index] {
                out[out_index] = left[left_index];
                left_index += 1;
                out_index += 1;
            } else {
                out[out_index] = right[right_index];
                right_index += 1;
                out_index += 1;
            }
        }
        self.a_index = left_index;
        self.b_index = right_index;
        self.out_index = out_index;
    }
    fn divide(self) -> (Self, Self) {
        //PRECONDITION not a trivial merge, as per triviality check.
        let out_remaining_len = self.out.len() - self.out_index;
        let a_remaining_len = self.a.len() - self.a_index;
        let b_remaining_len = self.b.len() - self.b_index;
        let mut swap = false;
        let ((longer_slice, longer_slice_index), (shorter_slice, shorter_slice_index)) =
            if a_remaining_len >= b_remaining_len {
                ((self.a, self.a_index), (self.b, self.b_index))
            } else {
                swap = true;
                ((self.b, self.b_index), (self.a, self.a_index))
            };
        let longer_slice_remaining_len = longer_slice.len() - longer_slice_index;
        let shorter_slice_remaining_len = shorter_slice.len() - shorter_slice_index;
        // 0. Partition longer slice into less, equal, greater.
        // 1. Search for the equal element in the other guy, get less, equal and greater.
        // 2. Copy the equal stuff directly into the output. left equal followed by right equal for
        //    stability.
        // 3. Return (less, less), (greater, greater)
        let pivot = longer_slice[(longer_slice_index + longer_slice.len()) / 2 - 1];

        let longer_left_slice = subslice_without_last_value(
            &longer_slice[longer_slice_index..(longer_slice_index + longer_slice.len()) / 2],
        );
        let longer_right_slice = subslice_without_first_value(
            &longer_slice[(longer_slice_index + longer_slice.len()) / 2 - 1..],
        );

        let longer_equal_slice = if longer_slice_remaining_len
            - (longer_left_slice.len() + longer_right_slice.len())
            > 0
        {
            &longer_slice[longer_slice_index + longer_left_slice.len()
                ..(longer_slice.len() - longer_right_slice.len())]
        } else {
            &[]
        };

        let shorter_right_bound = shorter_slice[shorter_slice_index..]
            .binary_search_by(|x| {
                if x == &pivot {
                    std::cmp::Ordering::Less
                } else if x < &pivot {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
            .unwrap_err();
        let shorter_right_slice = if shorter_slice_index + shorter_right_bound < shorter_slice.len()
        {
            &shorter_slice[shorter_slice_index + shorter_right_bound..]
        } else {
            &[]
        };
        //Do another binary search for the left bound, subslice_without_last_value is anyway not as
        //fast as subslice_without_first_value
        let shorter_left_bound = shorter_slice
            [shorter_slice_index..shorter_slice_index + shorter_right_bound]
            .binary_search_by(|x| {
                if x == &pivot {
                    std::cmp::Ordering::Greater
                } else if x < &pivot {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
            .unwrap_err();
        let shorter_left_slice =
            &shorter_slice[shorter_slice_index..shorter_slice_index + shorter_left_bound];

        let shorter_equal_slice = if shorter_slice_remaining_len
            - (shorter_left_slice.len() + shorter_right_slice.len())
            > 0
        {
            &shorter_slice[shorter_slice_index + shorter_left_slice.len()
                ..shorter_slice.len() - shorter_right_slice.len()]
        } else {
            &[]
        };

        // Copy the equal part.
        let base = longer_left_slice.len() + shorter_left_slice.len();
        let (_done, left_out) = self.out.split_at_mut(self.out_index);
        let (left_out, remaining) = left_out.split_at_mut(base);
        let (equal, right_out) =
            remaining.split_at_mut(longer_equal_slice.len() + shorter_equal_slice.len());

        debug_assert!(
            longer_left_slice.len() + longer_equal_slice.len() + longer_right_slice.len()
                == longer_slice_remaining_len
        );
        debug_assert!(
            shorter_left_slice.len() + shorter_equal_slice.len() + shorter_right_slice.len()
                == shorter_slice_remaining_len
        );
        debug_assert!(left_out.len() + equal.len() + right_out.len() == out_remaining_len);
        //debug_assert!(
        //    std::cmp::max(
        //        longer_left_slice[longer_left_slice.len().saturating_sub(1)],
        //        shorter_left_slice[shorter_left_slice.len().saturating_sub(1)]
        //    ) < std::cmp::min(longer_right_slice[0], shorter_right_slice[0])
        //);
        if swap {
            equal[..shorter_equal_slice.len()].copy_from_slice(shorter_equal_slice);
            equal[shorter_equal_slice.len()..].copy_from_slice(longer_equal_slice);
            (
                Merger {
                    a: shorter_left_slice,
                    a_index: 0,
                    b: longer_left_slice,
                    b_index: 0,
                    out: left_out,
                    out_index: 0,
                },
                Merger {
                    a: shorter_right_slice,
                    a_index: 0,
                    b: longer_right_slice,
                    b_index: 0,
                    out: right_out,
                    out_index: 0,
                },
            )
        } else {
            equal[..longer_equal_slice.len()].copy_from_slice(longer_equal_slice);
            equal[longer_equal_slice.len()..].copy_from_slice(shorter_equal_slice);
            (
                Merger {
                    a: longer_left_slice,
                    a_index: 0,
                    b: shorter_left_slice,
                    b_index: 0,
                    out: left_out,
                    out_index: 0,
                },
                Merger {
                    a: longer_right_slice,
                    a_index: 0,
                    b: shorter_right_slice,
                    b_index: 0,
                    out: right_out,
                    out_index: 0,
                },
            )
        }
    }
}

pub fn adaptive_slice_merge<T: Copy + Ord + Send + Sync>(
    left: &mut [T],
    right: &mut [T],
    output: &mut [T],
) {
    let merger = Merger {
        a: left,
        b: right,
        a_index: 0,
        b_index: 0,
        out: output,
        out_index: 0,
    };
    work(
        merger,
        |m| m.out_index == m.out.len(),
        |m| m.divide(),
        |m, s| m.manual_merge(s),
        |m| m.check_triviality(),
    );
}
