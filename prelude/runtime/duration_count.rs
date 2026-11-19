fn duration_count(unit: &str, count: i64) -> u64 {
    u64::try_from(count)
        .unwrap_or_else(|_| panic!("a duration cannot be negative: ({unit} {count})"))
}
