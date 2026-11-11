pub struct FakePrograms;

impl FakePrograms {
    pub const MISTYPED: &str = "(def width (+ 1 \"two\"))\n";
    pub const UNREACHABLE_ARM: &str =
        "(enum shape (circle f64) point)\n\n(defn area (s)\n  (match s (_ 0.0) (point 1.0)))\n";
}
