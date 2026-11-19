#[track_caller]
fn duration_count(unit: &str, count: i64) -> u64 {
    match u64::try_from(count) {
        Ok(count) => count,
        Err(_) => panic!("a duration cannot be negative: ({unit} {count})"),
    }
}
