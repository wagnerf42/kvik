use crate::adaptive_slice_merge;
use crate::prelude::*;
fn fuse_slices<'a: 'c, 'b: 'c, 'c, T: 'a + 'b>(s1: &'a mut [T], s2: &'b mut [T]) -> &'c mut [T] {
    let ptr1 = s1.as_mut_ptr();
    unsafe {
        assert_eq!(ptr1.add(s1.len()) as *const T, s2.as_ptr(),);
        std::slice::from_raw_parts_mut(ptr1, s1.len() + s2.len())
    }
}

/// This is a stable parallel merge sort for slices
pub fn slice_par_sort<T: Copy + Ord + Send + Sync>(input: &mut [T]) {
    let input_len = input.len();
    let mut buffer: Vec<T> = Vec::with_capacity(input_len);
    unsafe {
        buffer.set_len(input_len);
    }
    (input.iter_mut(), buffer.iter_mut())
        .wrap_iter()
        .map(|s| {
            let left_slice = s.0.into_slice();
            let right_slice = s.1.into_slice();
            left_slice.sort();
            (left_slice, right_slice)
        })
        .join_policy(input_len / rayon::current_num_threads())
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            adaptive_slice_merge(left_input, right_input, new_output);
            (new_output, fuse_slices(left_input, right_input))
        });
}