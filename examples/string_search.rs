use itertools::Itertools;
use rayon_try_fold::prelude::*;

fn fuse_str<'a: 'c, 'b: 'c, 'c>(s1: &'a str, s2: &'b str) -> &'c str {
    let ptr1 = s1.as_ptr();
    unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr1, s1.len() + s2.len())) }
}

fn main() {
    let target = "tati";
    assert!("tototatatititutu"
        .wrap_iter()
        .rayon(2)
        .map(|s| (s, s.contains(target)))
        .reduce_with(|(left_s, left_c), (right_s, right_c)| {
            let i = left_s.len();
            let s = fuse_str(left_s, right_s);
            (
                s,
                if left_c || right_c {
                    true
                } else {
                    s[i.saturating_sub(target.len())..(i.saturating_add(target.len()).min(s.len()))]
                        .contains(target)
                },
            )
        })
        .map(|(_, c)| c)
        .unwrap_or(false))
}
