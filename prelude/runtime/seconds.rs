#[track_caller]
pub fn seconds(count: i64) -> std::time::Duration {
    std::time::Duration::from_secs(duration_count("seconds", count))
}
