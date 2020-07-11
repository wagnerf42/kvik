use crate::prelude::*;
use crate::Try;
use std::sync::atomic::{AtomicBool, Ordering};

/// basic try_reduce scheduler.
/// there is a strong pre-condition : there should always be a try_fold called before us
/// because we will not fold.
fn schedule_join_try_reduce<R, P, T>(mut producer: P, reducer: &R, stop: &AtomicBool) -> P::Item
where
    P: Producer,
    R: Reducer<P::Item>,
    P::Item: Try<Ok = T> + Send,
{
    if stop.load(Ordering::Relaxed) {
        reducer.identity()
    } else {
        if producer.should_be_divided() {
            let (left, right) = producer.divide();
            let (left_result, right_result) = rayon::join(
                || schedule_join_try_reduce(left, reducer, stop),
                || schedule_join_try_reduce(right, reducer, stop),
            );

            let final_result = reducer.reduce(left_result, right_result).into_result();
            match final_result {
                Err(final_error) => {
                    stop.store(true, Ordering::Relaxed);
                    P::Item::from_error(final_error)
                }
                Ok(final_ok) => P::Item::from_ok(final_ok),
            }
        } else {
            // this will only work if there is always a try_fold before us
            if let Some(final_result) = producer.next() {
                match final_result.into_result() {
                    Err(final_error) => {
                        stop.store(true, Ordering::Relaxed);
                        P::Item::from_error(final_error)
                    }
                    Ok(final_ok) => P::Item::from_ok(final_ok),
                }
            } else {
                reducer.identity()
            }
        }
    }
}
