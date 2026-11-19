use std::collections::BTreeSet;

use super::emitted_code::EmittedCode;
use super::runtime_part::RuntimePart;

pub struct ScannedPart {
    pub part: RuntimePart,
    pub markers: &'static [&'static str],
    pub companion: Option<RuntimePart>,
    pub bringers: &'static [RuntimePart],
    pub search: MarkerSearch,
}

impl ScannedPart {
    const STANDARD_STREAM_MARKERS: &'static [&'static str] = &[
        "print!(",
        "println!(",
        "std::io::stdout(",
        "eprint!(",
        "eprintln!(",
        "std::io::stderr(",
    ];
    const DURATION_COUNTS: &'static [RuntimePart] = &[
        RuntimePart::Seconds,
        RuntimePart::Millis,
        RuntimePart::Micros,
        RuntimePart::Nanos,
    ];
    pub const ALL: [ScannedPart; 5] = [
        ScannedPart {
            part: RuntimePart::FromLiteral,
            markers: &["::from_literal("],
            companion: None,
            bringers: &[],
            search: MarkerSearch::Anywhere,
        },
        ScannedPart {
            part: RuntimePart::FromFloatLiteral,
            markers: &["::from_float_literal("],
            companion: None,
            bringers: &[],
            search: MarkerSearch::Anywhere,
        },
        ScannedPart {
            part: RuntimePart::MapSequence,
            markers: &["std::collections::BTreeMap", "std::collections::HashMap"],
            companion: Some(RuntimePart::Sequence),
            bringers: &[],
            search: MarkerSearch::Anywhere,
        },
        ScannedPart {
            part: RuntimePart::Signal,
            markers: Self::STANDARD_STREAM_MARKERS,
            companion: None,
            bringers: &[],
            search: MarkerSearch::Unprefixed,
        },
        ScannedPart {
            part: RuntimePart::ShowDuration,
            markers: &["std::time::Duration", "std::time::Instant"],
            companion: Some(RuntimePart::Show),
            bringers: Self::DURATION_COUNTS,
            search: MarkerSearch::OutsideStrings,
        },
    ];

    pub fn found_in(&self, code: &EmittedCode, needed: &BTreeSet<RuntimePart>) -> bool {
        self.companion_is_needed(needed) && (self.brought_by(needed) || self.has_marker_in(code))
    }

    fn companion_is_needed(&self, needed: &BTreeSet<RuntimePart>) -> bool {
        self.companion
            .is_none_or(|companion| needed.contains(&companion))
    }

    fn brought_by(&self, needed: &BTreeSet<RuntimePart>) -> bool {
        self.bringers.iter().any(|bringer| needed.contains(bringer))
    }

    fn has_marker_in(&self, code: &EmittedCode) -> bool {
        self.markers
            .iter()
            .any(|marker| self.search.finds(code, marker))
    }

    pub fn part(&self) -> RuntimePart {
        self.part
    }
}

pub enum MarkerSearch {
    Anywhere,
    Unprefixed,
    OutsideStrings,
}

impl MarkerSearch {
    fn finds(&self, code: &EmittedCode, marker: &str) -> bool {
        match self {
            MarkerSearch::Anywhere => code.contains(marker),
            MarkerSearch::Unprefixed => code.contains_unprefixed(marker),
            MarkerSearch::OutsideStrings => code.contains_outside_strings(marker),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::ScannedPart;
    use crate::emit::emitted_code::EmittedCode;
    use crate::emit::runtime_part::RuntimePart;

    struct SignalScan;

    impl SignalScan {
        fn finds(code: &str) -> bool {
            ScannedPart::ALL
                .iter()
                .find(|scanned| scanned.part() == RuntimePart::Signal)
                .expect("the signal part is scanned")
                .found_in(&EmittedCode::new(code), &BTreeSet::new())
        }
    }

    #[test]
    fn a_stream_call_brings_the_signal_part() {
        assert!(SignalScan::finds("print!(\"a\")"));
    }

    #[test]
    fn a_stream_marker_inside_a_longer_name_brings_nothing() {
        assert!(!SignalScan::finds("sprint!(\"a\")"));
    }
}
