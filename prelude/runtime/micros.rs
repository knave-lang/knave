pub fn micros(count: i64) -> std::time::Duration {
    std::time::Duration::from_micros(duration_count("micros", count))
}
