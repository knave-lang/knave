#[track_caller]
pub fn nanos(count: i64) -> std::time::Duration {
    std::time::Duration::from_nanos(duration_count("nanos", count))
}
