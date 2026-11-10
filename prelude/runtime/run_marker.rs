pub fn run_marker() -> &'static str {
    static MARKER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    MARKER.get_or_init(|| std::env::var(TEST_MARKER_VARIABLE).unwrap_or_default())
}
