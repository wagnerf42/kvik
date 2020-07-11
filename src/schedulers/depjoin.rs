//! like join scheduler except that continuation is not always for left task
//! but instead for latest completing task.
use crate::prelude::*;
use crate::small_channel::small_channel;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct DepJoinScheduler;

impl<P, R> Scheduler<P, R> for DepJoinScheduler
where
    P: Producer,
    P::Item: Send,
    R: Reducer<P::Item>,
{
    fn schedule(&self, producer: P, reducer: &R) -> P::Item {
        if producer.should_be_divided() {
            let cleanup = AtomicBool::new(false);
            let (sender, receiver) = small_channel();
            let (sender1, receiver1) = small_channel();
            let (left, right) = producer.divide();
            let (left_r, right_r) = rayon::join(
                || {
                    let my_result = self.schedule(left, reducer);
                    let last = cleanup.swap(true, Ordering::SeqCst);
                    if last {
                        let his_result = receiver.recv().expect("receiving depjoin failed");
                        Some(reducer.reduce(my_result, his_result))
                    } else {
                        sender1.send(my_result);
                        None
                    }
                },
                || {
                    let my_result = self.schedule(right, reducer);
                    let last = cleanup.swap(true, Ordering::SeqCst);
                    if last {
                        let his_result = receiver1.recv().expect("receiving1 depjoin failed");
                        Some(reducer.reduce(his_result, my_result))
                    } else {
                        sender.send(my_result);
                        None
                    }
                },
            );
            left_r.or(right_r).unwrap()
        } else {
            reducer.fold(producer)
        }
    }
}
