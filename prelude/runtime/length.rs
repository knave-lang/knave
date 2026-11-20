#[inline]
#[track_caller]
pub fn length(value: usize) -> i64 {
    match i64::try_from(value) {
        Ok(length) => length,
        Err(_) => too_long(value),
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn too_long(value: usize) -> ! {
    panic!("length does not fit in int: {value}")
}
