#[inline]
#[track_caller]
pub fn index(value: i64) -> usize {
    match usize::try_from(value) {
        Ok(index) => index,
        Err(_) => negative(value),
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn negative(value: i64) -> ! {
    panic!("attempt to use a negative value as an index or a size: {value}")
}
