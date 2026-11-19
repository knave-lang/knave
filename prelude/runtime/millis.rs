#[track_caller]
pub fn millis(count: i64) -> std::time::Duration {
    std::time::Duration::from_millis(duration_count("millis", count))
}
