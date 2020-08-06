use crate::adaptive_slice_merge;
use crate::prelude::*;
#[cfg(feature = "logs")]
use rayon_logs::subgraph;
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
    (input, buffer.as_mut_slice())
        .wrap_iter()
        .map(|s| {
            #[cfg(feature = "logs")]
            {
                subgraph("sequential sort", s.0.len(), || {
                    let left_slice = s.0;
                    let right_slice = s.1;
                    left_slice.sort();
                    (left_slice, right_slice)
                })
            }

            #[cfg(not(feature = "logs"))]
            {
                let left_slice = s.0;
                let right_slice = s.1;
                left_slice.sort();
                (left_slice, right_slice)
            }
        })
        .join_context_policy(
            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                - (rayon::current_num_threads() as f32).log2().floor()) as u32,
        )
        .depjoin()
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            #[cfg(feature = "logs")]
            {
                subgraph("parallel fusion", new_output.len(), || {
                    adaptive_slice_merge(left_input, right_input, new_output)
                        .into_par_iter()
                        .micro_block_sizes(1024, 10_000)
                        .for_each(|_| ());
                });
            }
            #[cfg(not(feature = "logs"))]
            {
                adaptive_slice_merge(left_input, right_input, new_output)
                    .into_par_iter()
                    .micro_block_sizes(1024, 10_000)
                    .for_each(|_| ());
            }
            (new_output, fuse_slices(left_input, right_input))
        });
}
