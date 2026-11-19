use std::collections::BTreeSet;

use super::emitted_code::EmittedCode;
use super::runtime_part::RuntimePart;
use super::scanned_part::ScannedPart;
use crate::language::Language;

struct Conversion {
    part: RuntimePart,
    name: &'static str,
    input: &'static str,
}

pub struct RuntimeModule {
    parts: BTreeSet<RuntimePart>,
}

impl RuntimeModule {
    const SIGNED: &'static str = include_str!("../../prelude/runtime/signed.rs");
    const FLOAT: &'static str = include_str!("../../prelude/runtime/float.rs");
    const INTEGER: &'static str = include_str!("../../prelude/runtime/integer.rs");
    const SEQUENCE: &'static str = include_str!("../../prelude/runtime/sequence.rs");
    const CONTAINS: &'static str = include_str!("../../prelude/runtime/contains.rs");
    const JOIN: &'static str = include_str!("../../prelude/runtime/join.rs");
    const STARTS_WITH: &'static str = include_str!("../../prelude/runtime/starts_with.rs");
    const MAP_SEQUENCE: &'static str = include_str!("../../prelude/runtime/map_sequence.rs");
    const IO: &'static str = include_str!("../../prelude/runtime/io.rs");
    const EXIT: &'static str = include_str!("../../prelude/runtime/exit.rs");
    const SECONDS: &'static str = include_str!("../../prelude/runtime/seconds.rs");
    const MILLIS: &'static str = include_str!("../../prelude/runtime/millis.rs");
    const MICROS: &'static str = include_str!("../../prelude/runtime/micros.rs");
    const NANOS: &'static str = include_str!("../../prelude/runtime/nanos.rs");
    const DURATION_COUNT: &'static str = include_str!("../../prelude/runtime/duration_count.rs");
    const SOURCE: &'static str = include_str!("../../prelude/runtime/source.rs");
    const SEPARATOR: &'static str = include_str!("../../prelude/runtime/separator.rs");
    const LINES: &'static str = include_str!("../../prelude/runtime/lines.rs");
    const SIGNAL: &'static str = include_str!("../../prelude/runtime/signal.rs");
    const SHOW_DURATION: &'static str = include_str!("../../prelude/runtime/show_duration.rs");
    const SHOW: &'static str = include_str!("../../prelude/runtime/show.rs");
    const UNWRAP_OK: &'static str = include_str!("../../prelude/runtime/unwrap_ok.rs");
    const UNWRAP_ERR: &'static str = include_str!("../../prelude/runtime/unwrap_err.rs");
    const EXPECT_OK: &'static str = include_str!("../../prelude/runtime/expect_ok.rs");
    const FAILED_RECORD: &'static str = include_str!("../../prelude/runtime/failed_record.rs");
    const CHECK_FAILURE: &'static str = include_str!("../../prelude/runtime/check_failure.rs");
    const CHECK_TRUE: &'static str = include_str!("../../prelude/runtime/check_true.rs");
    const CHECK_EQUAL: &'static str = include_str!("../../prelude/runtime/check_equal.rs");
    const CHECK_TRUE_WITH: &'static str = include_str!("../../prelude/runtime/check_true_with.rs");
    const CHECK_EQUAL_WITH: &'static str =
        include_str!("../../prelude/runtime/check_equal_with.rs");
    const NOTE_UNTESTED: &'static str = include_str!("../../prelude/runtime/note_untested.rs");
    const RUN_MARKER: &'static str = include_str!("../../prelude/runtime/run_marker.rs");
    const TEST_RUN: &'static str = include_str!("../../prelude/runtime/test_run.rs");
    const RUN_TEST: &'static str = include_str!("../../prelude/runtime/run_test.rs");
    const MAX: &'static str = include_str!("../../prelude/runtime/max.rs");
    const MIN: &'static str = include_str!("../../prelude/runtime/min.rs");
    const IS_EVEN: &'static str = include_str!("../../prelude/runtime/is_even.rs");
    const INDEX: &'static str = include_str!("../../prelude/runtime/index.rs");
    const CONVERT: &'static str = include_str!("../../prelude/runtime/convert.rs");
    const START_UP_CALL: &'static str = "crate::rt::restore_default_sigpipe();";
    const FIXED_PARTS: [(RuntimePart, &'static str); 39] = [
        (RuntimePart::Signed, Self::SIGNED),
        (RuntimePart::Float, Self::FLOAT),
        (RuntimePart::Integer, Self::INTEGER),
        (RuntimePart::Sequence, Self::SEQUENCE),
        (RuntimePart::Contains, Self::CONTAINS),
        (RuntimePart::Join, Self::JOIN),
        (RuntimePart::StartsWith, Self::STARTS_WITH),
        (RuntimePart::MapSequence, Self::MAP_SEQUENCE),
        (RuntimePart::Io, Self::IO),
        (RuntimePart::Exit, Self::EXIT),
        (RuntimePart::Seconds, Self::SECONDS),
        (RuntimePart::Millis, Self::MILLIS),
        (RuntimePart::Micros, Self::MICROS),
        (RuntimePart::Nanos, Self::NANOS),
        (RuntimePart::DurationCount, Self::DURATION_COUNT),
        (RuntimePart::Source, Self::SOURCE),
        (RuntimePart::Separator, Self::SEPARATOR),
        (RuntimePart::Lines, Self::LINES),
        (RuntimePart::Signal, Self::SIGNAL),
        (RuntimePart::Show, Self::SHOW),
        (RuntimePart::ShowDuration, Self::SHOW_DURATION),
        (RuntimePart::UnwrapOk, Self::UNWRAP_OK),
        (RuntimePart::UnwrapErr, Self::UNWRAP_ERR),
        (RuntimePart::ExpectOk, Self::EXPECT_OK),
        (RuntimePart::FailedRecord, Self::FAILED_RECORD),
        (RuntimePart::CheckFailure, Self::CHECK_FAILURE),
        (RuntimePart::CheckTrue, Self::CHECK_TRUE),
        (RuntimePart::CheckEqual, Self::CHECK_EQUAL),
        (RuntimePart::CheckTrueWith, Self::CHECK_TRUE_WITH),
        (RuntimePart::CheckEqualWith, Self::CHECK_EQUAL_WITH),
        (RuntimePart::NoteUntested, Self::NOTE_UNTESTED),
        (RuntimePart::RunMarker, Self::RUN_MARKER),
        (RuntimePart::TestRun, Self::TEST_RUN),
        (RuntimePart::RunTest, Self::RUN_TEST),
        (RuntimePart::Max, Self::MAX),
        (RuntimePart::Min, Self::MIN),
        (RuntimePart::IsEven, Self::IS_EVEN),
        (RuntimePart::Index, Self::INDEX),
        (RuntimePart::Convert, Self::CONVERT),
    ];
    const CONVERSIONS: &'static [Conversion] = &[
        Conversion {
            part: RuntimePart::FromLiteral,
            name: "from_literal",
            input: "i128",
        },
        Conversion {
            part: RuntimePart::FromFloatLiteral,
            name: "from_float_literal",
            input: "f64",
        },
    ];
    const NUMBER_TRAIT: &'static str = "pub trait Number:\n    Copy + PartialOrd + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> + Rem<Output = Self>\n    + AddAssign + SubAssign + MulAssign + DivAssign + RemAssign\n";
    const NUMBER_IMPLEMENTATIONS: &'static str = "macro_rules! numbers {\n    ($($number:ty),*) => {\n        $(\n            impl Number for $number ";
    const NUMBER_IMPLEMENTATIONS_END: &'static str = "\n        )*\n    };\n}\n\nnumbers!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);";
    const IMPLEMENTATION_INDENT: &'static str = "            ";

    pub fn of(code: &str) -> Option<RuntimeModule> {
        let code = EmittedCode::new(code);
        let named = Self::with_required(code.named_parts());
        let scanned = ScannedPart::ALL
            .iter()
            .filter(|scanned| scanned.found_in(&code, &named))
            .map(ScannedPart::part);
        let parts = Self::with_required(named.iter().copied().chain(scanned).collect());
        (!parts.is_empty()).then_some(RuntimeModule { parts })
    }

    fn with_required(mut parts: BTreeSet<RuntimePart>) -> BTreeSet<RuntimePart> {
        let mut unchecked: Vec<RuntimePart> = parts.iter().copied().collect();
        while let Some(part) = unchecked.pop() {
            for required in part.required() {
                if parts.insert(*required) {
                    unchecked.push(*required);
                }
            }
        }
        parts
    }

    pub fn start_up_call(&self) -> Option<&'static str> {
        self.parts
            .contains(&RuntimePart::Signal)
            .then_some(Self::START_UP_CALL)
    }

    pub fn source(&self) -> String {
        let mut sections = Vec::new();
        if self.parts.contains(&RuntimePart::Number) {
            sections.push(self.arithmetic_imports());
            sections.push(self.number_trait());
            sections.push(self.number_implementations());
        }
        for (part, text) in Self::FIXED_PARTS {
            if self.parts.contains(&part) {
                sections.push(Self::rendered(part, text));
            }
        }
        sections.join("\n\n")
    }

    pub fn test_marker_variable() -> String {
        format!("{}_TEST_MARKER", Language::NAME.to_ascii_uppercase())
    }

    fn rendered(part: RuntimePart, text: &str) -> String {
        let text = text.trim_end();
        match part {
            RuntimePart::RunMarker => format!(
                "const TEST_MARKER_VARIABLE: &str = {:?};\n\n{text}",
                Self::test_marker_variable()
            ),
            _ => text.to_string(),
        }
    }

    fn arithmetic_imports(&self) -> String {
        let negation = if self.parts.contains(&RuntimePart::Signed) {
            "Neg, "
        } else {
            ""
        };
        format!(
            "use std::ops::{{Add, AddAssign, Div, DivAssign, Mul, MulAssign, {negation}Rem, RemAssign, Sub, SubAssign}};"
        )
    }

    fn number_trait(&self) -> String {
        let declarations: Vec<String> = self
            .conversions()
            .map(|conversion| {
                format!(
                    "    fn {}(value: {}) -> Self;",
                    conversion.name, conversion.input
                )
            })
            .collect();
        format!("{}{}", Self::NUMBER_TRAIT, Self::body(&declarations, ""))
    }

    fn number_implementations(&self) -> String {
        let indent = Self::IMPLEMENTATION_INDENT;
        let definitions: Vec<String> = self
            .conversions()
            .map(|conversion| {
                format!(
                    "{indent}    fn {}(value: {}) -> Self {{\n{indent}        value as Self\n{indent}    }}",
                    conversion.name, conversion.input
                )
            })
            .collect();
        format!(
            "{}{}{}",
            Self::NUMBER_IMPLEMENTATIONS,
            Self::body(&definitions, indent),
            Self::NUMBER_IMPLEMENTATIONS_END
        )
    }

    fn conversions(&self) -> impl Iterator<Item = &'static Conversion> {
        Self::CONVERSIONS
            .iter()
            .filter(|conversion| self.parts.contains(&conversion.part))
    }

    fn body(members: &[String], indent: &str) -> String {
        if members.is_empty() {
            return "{}".to_string();
        }
        format!("{{\n{}\n{indent}}}", members.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_standard_stream_write_restores_the_default_sigpipe() {
        for code in [
            "print!(\"a\")",
            "println!(\"a\")",
            "std::io::stdout()",
            "eprint!(\"a\")",
            "eprintln!(\"a\")",
            "std::io::stderr()",
        ] {
            let module = RuntimeModule::of(code).expect("the code needs the runtime");
            assert!(module.start_up_call().is_some(), "{code}");
        }
    }

    #[test]
    fn an_exit_call_brings_the_exit_function() {
        let module =
            RuntimeModule::of("crate::rt::exit(3_i32)").expect("the code needs the runtime");
        assert!(module.source().contains("pub fn exit(code: i32)"));
    }

    #[test]
    fn an_is_even_call_brings_the_literal_conversion_it_uses() {
        let module =
            RuntimeModule::of("crate::rt::is_even(x)").expect("the code needs the runtime");
        let source = module.source();
        assert!(source.contains("pub fn is_even<N: Integer>"));
        assert!(source.contains("fn from_literal(value: i128) -> Self;"));
    }

    #[test]
    fn an_index_conversion_brings_the_index_function() {
        let module =
            RuntimeModule::of("items[crate::rt::index(i)]").expect("the code needs the runtime");
        assert!(
            module
                .source()
                .contains("pub fn index(value: i64) -> usize")
        );
        assert!(module.start_up_call().is_none());
    }

    #[test]
    fn a_concrete_conversion_brings_the_integer_trait_it_bounds() {
        let module = RuntimeModule::of("crate::rt::convert::<i64, _>(count)")
            .expect("the code needs the runtime");
        let source = module.source();
        assert!(source.contains("pub fn convert<To: Integer, From: Integer>"));
        assert!(source.contains("pub trait Integer:"));
    }

    #[test]
    fn a_runtime_name_brings_its_part_only_when_whole() {
        let shown = RuntimeModule::of("crate::rt::Shown(value).to_string()")
            .expect("the code needs the runtime");
        assert!(shown.source().contains("pub trait Show"));
        assert!(RuntimeModule::of("crate::rt::exit_code").is_none());
    }

    #[test]
    fn a_map_brings_the_map_sequence_only_beside_a_sequence() {
        let map_alone = "std::collections::BTreeMap::new()";
        let measured = "<std::collections::HashMap<i64, i64> as crate::rt::Sequence>::len(&m)";
        assert!(RuntimeModule::of(map_alone).is_none());
        let module = RuntimeModule::of(measured).expect("the code needs the runtime");
        assert!(module.source().contains("HashMap"));
    }
}
