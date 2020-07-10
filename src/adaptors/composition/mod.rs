use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) mod composed;
pub(crate) mod composed_counter;
pub(crate) mod composed_size;
pub(crate) mod composed_task;

thread_local! {
    pub(crate) static ALLOW_PARALLELISM: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

pub(crate) use {
    composed::Composed, composed_counter::ComposedCounter, composed_size::ComposedSize,
    composed_task::ComposedTask,
};
