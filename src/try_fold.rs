//! we need to re-implement try_fold because we don't have the real Try trait
use crate::Try;
pub(crate) fn try_fold<I, B, F, R>(iterator: &mut I, init: B, mut f: F) -> R
where
    F: FnMut(B, I::Item) -> R,
    R: Try<Ok = B>,
    I: Iterator,
{
    let mut accum = init;
    for x in iterator {
        let accum_value = f(accum, x);
        match accum_value.into_result() {
            Ok(e) => {
                accum = e;
            }
            Err(e) => return Try::from_error(e),
        }
    }
    Try::from_ok(accum)
}
