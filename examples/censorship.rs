use lipsum::lipsum;
use kvik::prelude::*;

// like replace but in place.
// it's a bit dirty.
fn replace(input: &mut str, target: &str, replacement: &str) {
    let input = unsafe { input.as_bytes_mut() };
    let target = target.as_bytes();
    let replacement = replacement.as_bytes();
    assert_eq!(target.len(), replacement.len());
    for i in 0..(input.len().saturating_sub(target.len())) {
        let r = i..(i + target.len());
        if input[r.clone()].eq(target) {
            input[r].copy_from_slice(replacement)
        }
    }
}

fn fuse_str_mut<'a: 'c, 'b: 'c, 'c>(s1: &'a mut str, s2: &'b mut str) -> &'c mut str {
    let ptr1 = s1.as_mut_ptr();
    unsafe {
        std::str::from_utf8_unchecked_mut(std::slice::from_raw_parts_mut(ptr1, s1.len() + s2.len()))
    }
}

fn middle_part<'a>(left: &'a mut str, right: &'a mut str, size: usize) -> &'a mut str {
    //TODO: this is not so nice because we are not assured to end on utf8-boundaries
    //it's ok in our example though because we don't have special characters.
    let i = left.len();
    let s = fuse_str_mut(left, right);
    let s_len = s.len();
    &mut s[i.saturating_sub(size)..(i.saturating_add(size).min(s_len))]
}

fn par_censor(input: &mut str, searching_for: &str, replacing_with: &str) {
    input
        .wrap_iter()
        .rayon(2)
        .map(|s| {
            replace(s, searching_for, replacing_with);
            s
        })
        .reduce_with(|l, r| {
            let mid = middle_part(l, r, searching_for.len());
            replace(mid, searching_for, replacing_with);
            fuse_str_mut(l, r)
        });
}

const WORDS_NUMBER: usize = 10_000_000;

fn main() {
    let searching_for = "Lorem";
    let replacing_with: String = searching_for.chars().map(|_| '*').collect();
    let mut input = lipsum(WORDS_NUMBER);
    input.replace(searching_for, &replacing_with);
    let start = std::time::Instant::now();
    par_censor(&mut input, searching_for, &replacing_with);
    println!("censored {} words in {:?}", WORDS_NUMBER, start.elapsed());
    assert!(!input.contains(searching_for));
}
