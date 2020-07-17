use std::iter::repeat;
pub fn index_without_first_value<T: Eq>(slice: &[T]) -> usize {
    //Returns: The smallest i at which slice[i]!=slice[0]
    //PRECONDITION: slice is not empty
    match slice.first() {
        Some(target) => {
            let searching_range_end = repeat(())
                .scan(1, |acc, _| {
                    *acc *= 2;
                    Some(*acc)
                })
                .take_while(|&i| i < slice.len())
                .find(|&i| unsafe { slice.get_unchecked(i) != target })
                .unwrap_or_else(|| slice.len());
            slice[..searching_range_end]
                .binary_search_by(|x| {
                    if x.eq(target) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                })
                .unwrap_err()
        }
        None => 0,
    }
}
pub fn subslice_without_first_value<T: Eq>(slice: &[T]) -> &[T] {
    //Returns: The smallest i at which slice[i]!=slice[0]
    //PRECONDITION: slice is not empty
    match slice.first() {
        Some(target) => {
            let searching_range_end = repeat(())
                .scan(1, |acc, _| {
                    *acc *= 2;
                    Some(*acc)
                })
                .take_while(|&i| i < slice.len())
                .find(|&i| unsafe { slice.get_unchecked(i) != target })
                .unwrap_or_else(|| slice.len());
            let index = slice[..searching_range_end]
                .binary_search_by(|x| {
                    if x.eq(target) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                })
                .unwrap_err();
            &slice[index..]
        }
        None => slice,
    }
}
pub fn index_without_last_value<T: Eq>(slice: &[T]) -> usize {
    //returns the largest index at which the value is not equal to the last value
    //don't pass in empty slice
    match slice.split_last() {
        Some((target, slice)) => {
            let searching_range_start = repeat(())
                .scan(1, |acc, _| {
                    *acc *= 2;
                    Some(*acc)
                }) // iterate on all powers of 2
                .take_while(|&i| i < slice.len())
                .map(|i| slice.len() - i) // go farther and farther from end of slice
                .find(|&i| unsafe { slice.get_unchecked(i) != target })
                .unwrap_or(0);

            let index = slice[searching_range_start..]
                .binary_search_by(|x| {
                    if x.eq(target) {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                })
                .unwrap_err();
            (searching_range_start + index).saturating_sub(1)
        }
        None => 0,
    }
}
pub fn subslice_without_last_value<T: Eq>(slice: &[T]) -> &[T] {
    //returns the largest index at which the value is not equal to the last value
    //don't pass in empty slice
    match slice.split_last() {
        Some((target, slice)) => {
            let searching_range_start = repeat(())
                .scan(1, |acc, _| {
                    *acc *= 2;
                    Some(*acc)
                }) // iterate on all powers of 2
                .take_while(|&i| i < slice.len())
                .map(|i| slice.len() - i) // go farther and farther from end of slice
                .find(|&i| unsafe { slice.get_unchecked(i) != target })
                .unwrap_or(0);

            let index = slice[searching_range_start..]
                .binary_search_by(|x| {
                    if x.eq(target) {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Less
                    }
                })
                .unwrap_err();
            &slice[..(searching_range_start + index)]
        }
        None => slice,
    }
}
