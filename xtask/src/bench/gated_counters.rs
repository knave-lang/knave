pub struct GatedCounters;

impl GatedCounters {
    pub const NAMES: [&'static str; 2] = ["allocator.allocated_bytes", "allocator.peak_live_bytes"];
    pub const TOLERANCE_PERCENT: u64 = 1;

    pub fn listed() -> String {
        Self::NAMES
            .map(|counter| format!("`{counter}`"))
            .join(" or ")
    }
}
