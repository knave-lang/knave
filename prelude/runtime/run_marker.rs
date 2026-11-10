pub fn run_marker() -> &'static str {
    static MARKER: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    MARKER.get_or_init(|| {
        let marker = std::env::var(TEST_MARKER_VARIABLE).unwrap_or_default();
        unsafe { std::env::remove_var(TEST_MARKER_VARIABLE) };
        marker
    })
}
