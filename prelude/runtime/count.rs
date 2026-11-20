#[inline]
#[track_caller]
pub fn count(items: impl Iterator) -> i64 {
    let counted = items.count();
    match i64::try_from(counted) {
        Ok(count) => count,
        Err(_) => count_overflow(counted),
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn count_overflow(counted: usize) -> ! {
    panic!("attempt to count with overflow: the count is {counted}")
}
