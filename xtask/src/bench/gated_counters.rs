pub struct GatedCounters;

impl GatedCounters {
    pub const NAMES: [&'static str; 3] = [
        "allocator.allocated_bytes",
        "allocator.allocations",
        "allocator.peak_live_bytes",
    ];
    pub const TOLERANCE_PERCENT: u64 = 1;

    pub fn listed() -> String {
        let quoted = Self::NAMES.map(|counter| format!("`{counter}`"));
        let [leading @ .., last] = &quoted;
        format!("{} or {last}", leading.join(", "))
    }
}
