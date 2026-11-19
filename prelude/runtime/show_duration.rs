impl Show for std::time::Duration {
    fn write_form(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nanos = self.as_nanos();
        let unit = DURATION_UNITS
            .iter()
            .find(|unit| nanos.is_multiple_of(unit.nanos))
            .unwrap_or(&NANOS);
        write!(out, "({} {})", unit.name, nanos / unit.nanos)
    }
}

struct DurationUnit {
    name: &'static str,
    nanos: u128,
}

const NANOS: DurationUnit = DurationUnit {
    name: "nanos",
    nanos: 1,
};

const DURATION_UNITS: [DurationUnit; 4] = [
    DurationUnit {
        name: "seconds",
        nanos: 1_000_000_000,
    },
    DurationUnit {
        name: "millis",
        nanos: 1_000_000,
    },
    DurationUnit {
        name: "micros",
        nanos: 1_000,
    },
    NANOS,
];
