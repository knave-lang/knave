pub fn run_marker() -> &'static str {
    static MARKER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    MARKER.get_or_init(|| std::env::var("KNAVE_TEST_MARKER").unwrap_or_default())
}
