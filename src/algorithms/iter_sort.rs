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
pub fn iter_sort_jc_adaptive<T: Copy + Ord + Send + Sync>(input: &mut [T]) {
    let input_len = input.len();
    let mut buffer: Vec<T> = Vec::with_capacity(input_len);
    unsafe {
        buffer.set_len(input_len);
    }
    (input, buffer.as_mut_slice())
        .wrap_iter()
        .map(|(inp, out)| {
            #[cfg(feature = "logs")]
            {
                subgraph("sequential sort", inp.len(), || {
                    inp.sort();
                    (inp, out)
                })
            }

            #[cfg(not(feature = "logs"))]
            {
                inp.sort();
                (inp, out)
            }
        })
        .join_context_policy(std::cmp::min(
            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                - (rayon::current_num_threads() as f32).log2().floor()) as u32,
            5u32,
        ))
        .depjoin()
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            #[cfg(feature = "logs")]
            {
                subgraph("parallel fusion", new_output.len(), || {
                    left_input
                        .as_ref()
                        .into_par_iter()
                        .merge(right_input.as_ref())
                        .zip(&mut new_output[..])
                        .adaptive() // we still need it because zip cannot relay info
                        .for_each(|(inp, out)| {
                            *out = *inp;
                        });
                });
            }
            #[cfg(not(feature = "logs"))]
            {
                (left_input.as_ref())
                    .into_par_iter()
                    .merge(right_input.as_ref())
                    .zip(&mut new_output[..])
                    .adaptive()
                    .for_each(|(inp, out)| {
                        *out = *inp;
                    });
            }
            (new_output, fuse_slices(left_input, right_input))
        });
}

pub fn iter_sort_size_adaptive<T: Copy + Ord + Send + Sync>(input: &mut [T], num_threads: usize) {
    let input_len = input.len();
    let mut buffer: Vec<T> = Vec::with_capacity(input_len);
    unsafe {
        buffer.set_len(input_len);
    }
    (input, buffer.as_mut_slice())
        .wrap_iter()
        .map(|(inp, out)| {
            #[cfg(feature = "logs")]
            {
                subgraph("sequential sort", inp.len(), || {
                    inp.sort();
                    (inp, out)
                })
            }

            #[cfg(not(feature = "logs"))]
            {
                inp.sort();
                (inp, out)
            }
        })
        .size_limit(input_len / num_threads)
        .depjoin()
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            #[cfg(feature = "logs")]
            {
                subgraph("parallel fusion", new_output.len(), || {
                    left_input
                        .as_ref()
                        .into_par_iter()
                        .merge(right_input.as_ref())
                        .zip(&mut new_output[..])
                        .adaptive()
                        .for_each(|(inp, out)| {
                            *out = *inp;
                        });
                });
            }
            #[cfg(not(feature = "logs"))]
            {
                (left_input.as_ref())
                    .into_par_iter()
                    .merge(right_input.as_ref())
                    .zip(&mut new_output[..])
                    .adaptive()
                    .for_each(|(inp, out)| {
                        *out = *inp;
                    });
            }
            (new_output, fuse_slices(left_input, right_input))
        });
}

pub fn iter_sort_rayon_adaptive<T: Copy + Ord + Send + Sync>(input: &mut [T]) {
    let input_len = input.len();
    let mut buffer: Vec<T> = Vec::with_capacity(input_len);
    unsafe {
        buffer.set_len(input_len);
    }
    (input, buffer.as_mut_slice())
        .wrap_iter()
        .map(|(inp, out)| {
            #[cfg(feature = "logs")]
            {
                subgraph("sequential sort", inp.len(), || {
                    inp.sort();
                    (inp, out)
                })
            }

            #[cfg(not(feature = "logs"))]
            {
                inp.sort();
                (inp, out)
            }
        })
        .rayon(
            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                - (rayon::current_num_threads() as f32).log2().floor()) as usize,
        )
        .depjoin()
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            #[cfg(feature = "logs")]
            {
                subgraph("parallel fusion", new_output.len(), || {
                    left_input
                        .as_ref()
                        .into_par_iter()
                        .merge(right_input.as_ref())
                        .zip(&mut new_output[..])
                        .adaptive()
                        .for_each(|(inp, out)| {
                            *out = *inp;
                        });
                });
            }
            #[cfg(not(feature = "logs"))]
            {
                (left_input.as_ref())
                    .into_par_iter()
                    .merge(right_input.as_ref())
                    .zip(&mut new_output[..])
                    .adaptive()
                    .for_each(|(inp, out)| {
                        *out = *inp;
                    });
            }
            (new_output, fuse_slices(left_input, right_input))
        });
}

pub fn iter_sort_rayon_rayon<T: Copy + Ord + Send + Sync>(input: &mut [T]) {
    let input_len = input.len();
    let mut buffer: Vec<T> = Vec::with_capacity(input_len);
    unsafe {
        buffer.set_len(input_len);
    }
    (input, buffer.as_mut_slice())
        .wrap_iter()
        .map(|(inp, out)| {
            #[cfg(feature = "logs")]
            {
                subgraph("sequential sort", inp.len(), || {
                    inp.sort();
                    (inp, out)
                })
            }

            #[cfg(not(feature = "logs"))]
            {
                inp.sort();
                (inp, out)
            }
        })
        .rayon(
            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                - (rayon::current_num_threads() as f32).log2().floor()) as usize,
        )
        .depjoin()
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            #[cfg(feature = "logs")]
            {
                subgraph("parallel fusion", new_output.len(), || {
                    left_input
                        .as_ref()
                        .into_par_iter()
                        .merge(right_input.as_ref())
                        .zip(&mut new_output[..])
                        .rayon(
                            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                                - (rayon::current_num_threads() as f32).log2().floor())
                                as usize,
                        )
                        .for_each(|(inp, out)| {
                            *out = *inp;
                        });
                });
            }
            #[cfg(not(feature = "logs"))]
            {
                (left_input.as_ref())
                    .into_par_iter()
                    .merge(right_input.as_ref())
                    .zip(&mut new_output[..])
                    .rayon(
                        (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                            - (rayon::current_num_threads() as f32).log2().floor())
                            as usize,
                    )
                    .for_each(|(inp, out)| {
                        *out = *inp;
                    });
            }
            (new_output, fuse_slices(left_input, right_input))
        });
}
pub fn iter_sort_size_rayon<T: Copy + Ord + Send + Sync>(input: &mut [T], num_threads: usize) {
    let input_len = input.len();
    let mut buffer: Vec<T> = Vec::with_capacity(input_len);
    unsafe {
        buffer.set_len(input_len);
    }
    (input, buffer.as_mut_slice())
        .wrap_iter()
        .map(|(inp, out)| {
            #[cfg(feature = "logs")]
            {
                subgraph("sequential sort", inp.len(), || {
                    inp.sort();
                    (inp, out)
                })
            }

            #[cfg(not(feature = "logs"))]
            {
                inp.sort();
                (inp, out)
            }
        })
        .size_limit(input_len / num_threads)
        .depjoin()
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            #[cfg(feature = "logs")]
            {
                subgraph("parallel fusion", new_output.len(), || {
                    left_input
                        .as_ref()
                        .into_par_iter()
                        .merge(right_input.as_ref())
                        .zip(&mut new_output[..])
                        .rayon(
                            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                                - (rayon::current_num_threads() as f32).log2().floor())
                                as usize,
                        )
                        .for_each(|(inp, out)| {
                            *out = *inp;
                        });
                });
            }
            #[cfg(not(feature = "logs"))]
            {
                (left_input.as_ref())
                    .into_par_iter()
                    .merge(right_input.as_ref())
                    .zip(&mut new_output[..])
                    .rayon(
                        (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                            - (rayon::current_num_threads() as f32).log2().floor())
                            as usize,
                    )
                    .for_each(|(inp, out)| {
                        *out = *inp;
                    });
            }
            (new_output, fuse_slices(left_input, right_input))
        });
}
pub fn iter_sort_jc_rayon<T: Copy + Ord + Send + Sync>(input: &mut [T]) {
    let input_len = input.len();
    let mut buffer: Vec<T> = Vec::with_capacity(input_len);
    unsafe {
        buffer.set_len(input_len);
    }
    (input, buffer.as_mut_slice())
        .wrap_iter()
        .map(|(inp, out)| {
            #[cfg(feature = "logs")]
            {
                subgraph("sequential sort", inp.len(), || {
                    inp.sort();
                    (inp, out)
                })
            }

            #[cfg(not(feature = "logs"))]
            {
                inp.sort();
                (inp, out)
            }
        })
        .join_context_policy(std::cmp::min(
            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                - (rayon::current_num_threads() as f32).log2().floor()) as u32,
            5u32,
        ))
        .depjoin()
        .even_levels()
        .reduce_with(|(left_input, left_output), (right_input, right_output)| {
            let new_output = fuse_slices(left_output, right_output);
            #[cfg(feature = "logs")]
            {
                subgraph("parallel fusion", new_output.len(), || {
                    left_input
                        .as_ref()
                        .into_par_iter()
                        .merge(right_input.as_ref())
                        .zip(&mut new_output[..])
                        .rayon(
                            (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                                - (rayon::current_num_threads() as f32).log2().floor())
                                as usize,
                        )
                        .for_each(|(inp, out)| {
                            *out = *inp;
                        });
                });
            }
            #[cfg(not(feature = "logs"))]
            {
                (left_input.as_ref())
                    .into_par_iter()
                    .merge(right_input.as_ref())
                    .zip(&mut new_output[..])
                    .rayon(
                        (2.0 * (rayon::current_num_threads() as f32).log2().ceil()
                            - (rayon::current_num_threads() as f32).log2().floor())
                            as usize,
                    )
                    .for_each(|(inp, out)| {
                        *out = *inp;
                    });
            }
            (new_output, fuse_slices(left_input, right_input))
        });
}
