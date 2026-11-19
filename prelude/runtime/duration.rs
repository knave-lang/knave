use std::time::Duration;

pub fn seconds(count: i64) -> Duration {
    Duration::from_secs(non_negative("seconds", count))
}

pub fn millis(count: i64) -> Duration {
    Duration::from_millis(non_negative("millis", count))
}

pub fn micros(count: i64) -> Duration {
    Duration::from_micros(non_negative("micros", count))
}

pub fn nanos(count: i64) -> Duration {
    Duration::from_nanos(non_negative("nanos", count))
}

fn non_negative(unit: &str, count: i64) -> u64 {
    u64::try_from(count)
        .unwrap_or_else(|_| panic!("a duration cannot be negative: ({unit} {count})"))
}
