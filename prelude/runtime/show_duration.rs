impl Show for std::time::Duration {
    fn write_form(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nanos = self.as_nanos();
        let (unit, per_unit) = DURATION_UNITS
            .into_iter()
            .find(|&(_, per_unit)| nanos % per_unit == 0)
            .unwrap_or(NANOS);
        write!(out, "({unit} {})", nanos / per_unit)
    }
}

const NANOS: (&str, u128) = ("nanos", 1);

const DURATION_UNITS: [(&str, u128); 4] = [
    ("seconds", 1_000_000_000),
    ("millis", 1_000_000),
    ("micros", 1_000),
    NANOS,
];
