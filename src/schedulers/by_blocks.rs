//! sequential scheduler on macro blocks using
//! an internal parallel scheduler.
use crate::prelude::*;

pub(crate) struct ByBlocksScheduler<P, R> {
    inner_scheduler: Box<dyn Scheduler<P, R>>,
}

impl<P, R> Scheduler<P, R> for ByBlocksScheduler<P, R>
where
    P: Producer,
    P::Item: Send,
    R: Reducer<P::Item>,
{
    fn schedule(&self, producer: P, reducer: &R) -> P::Item {
        let sizes = std::iter::successors(Some(rayon::current_num_threads()), |s| {
            Some(s.saturating_mul(2))
        });
        // let's get a sequential iterator of producers of increasing sizes
        let producers = sizes.scan(Some(producer), |p, s| {
            let remaining_producer = p.take().unwrap();
            let (_, upper_bound) = remaining_producer.size_hint();
            let capped_size = if let Some(bound) = upper_bound {
                if bound == 0 {
                    return None;
                } else {
                    s.min(bound)
                }
            } else {
                s
            };
            let (left, right) = remaining_producer.divide_at(capped_size);
            *p = Some(right);
            Some(left)
        });
        reducer.fold(&mut producers.map(|p| self.inner_scheduler.schedule(p, reducer)))
    }
}
