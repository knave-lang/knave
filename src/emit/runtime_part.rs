#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimePart {
    Number,
    FromLiteral,
    FromFloatLiteral,
    Signed,
    Float,
    Integer,
    Sequence,
    Contains,
    Join,
    StartsWith,
    MapSequence,
    Io,
    Exit,
    Seconds,
    Millis,
    Micros,
    Nanos,
    DurationCount,
    Source,
    Separator,
    Lines,
    Signal,
    Show,
    ShowDuration,
    UnwrapOk,
    UnwrapErr,
    ExpectOk,
    FailedRecord,
    CheckFailure,
    CheckTrue,
    CheckEqual,
    CheckTrueWith,
    CheckEqualWith,
    NoteUntested,
    RunMarker,
    TestRun,
    RunTest,
    Max,
    Min,
    IsEven,
    Index,
    Convert,
}

impl RuntimePart {
    const NAMED: [(&'static str, RuntimePart); 35] = [
        ("Number", RuntimePart::Number),
        ("Signed", RuntimePart::Signed),
        ("Float", RuntimePart::Float),
        ("Integer", RuntimePart::Integer),
        ("Sequence", RuntimePart::Sequence),
        ("Contains", RuntimePart::Contains),
        ("Join", RuntimePart::Join),
        ("StartsWith", RuntimePart::StartsWith),
        ("read_line", RuntimePart::Io),
        ("exit", RuntimePart::Exit),
        ("seconds", RuntimePart::Seconds),
        ("millis", RuntimePart::Millis),
        ("micros", RuntimePart::Micros),
        ("nanos", RuntimePart::Nanos),
        ("Source", RuntimePart::Source),
        ("Separator", RuntimePart::Separator),
        ("lines", RuntimePart::Lines),
        ("Show", RuntimePart::Show),
        ("Shown", RuntimePart::Show),
        ("unwrap_ok", RuntimePart::UnwrapOk),
        ("unwrap_err", RuntimePart::UnwrapErr),
        ("expect_ok", RuntimePart::ExpectOk),
        ("check_true", RuntimePart::CheckTrue),
        ("check_equal", RuntimePart::CheckEqual),
        ("check_true_with", RuntimePart::CheckTrueWith),
        ("check_equal_with", RuntimePart::CheckEqualWith),
        ("note_untested", RuntimePart::NoteUntested),
        ("run_marker", RuntimePart::RunMarker),
        ("start_run", RuntimePart::TestRun),
        ("run_test", RuntimePart::RunTest),
        ("max", RuntimePart::Max),
        ("min", RuntimePart::Min),
        ("is_even", RuntimePart::IsEven),
        ("index", RuntimePart::Index),
        ("convert", RuntimePart::Convert),
    ];

    const REQUIRED: [(RuntimePart, &'static [RuntimePart]); 24] = [
        (RuntimePart::FromLiteral, &[RuntimePart::Number]),
        (RuntimePart::FromFloatLiteral, &[RuntimePart::Number]),
        (RuntimePart::Signed, &[RuntimePart::Number]),
        (RuntimePart::Integer, &[RuntimePart::Number]),
        (
            RuntimePart::IsEven,
            &[RuntimePart::Integer, RuntimePart::FromLiteral],
        ),
        (RuntimePart::Convert, &[RuntimePart::Integer]),
        (RuntimePart::Float, &[RuntimePart::Signed]),
        (RuntimePart::MapSequence, &[RuntimePart::Sequence]),
        (RuntimePart::Lines, &[RuntimePart::Io]),
        (RuntimePart::Seconds, &[RuntimePart::DurationCount]),
        (RuntimePart::Millis, &[RuntimePart::DurationCount]),
        (RuntimePart::Micros, &[RuntimePart::DurationCount]),
        (RuntimePart::Nanos, &[RuntimePart::DurationCount]),
        (RuntimePart::UnwrapOk, &[RuntimePart::Show]),
        (RuntimePart::UnwrapErr, &[RuntimePart::Show]),
        (RuntimePart::ExpectOk, &[RuntimePart::Show]),
        (
            RuntimePart::CheckFailure,
            &[RuntimePart::FailedRecord, RuntimePart::Show],
        ),
        (RuntimePart::CheckTrue, &[RuntimePart::CheckFailure]),
        (RuntimePart::CheckEqual, &[RuntimePart::CheckFailure]),
        (RuntimePart::CheckTrueWith, &[RuntimePart::CheckFailure]),
        (RuntimePart::CheckEqualWith, &[RuntimePart::CheckFailure]),
        (
            RuntimePart::NoteUntested,
            &[RuntimePart::Show, RuntimePart::Signal],
        ),
        (RuntimePart::TestRun, &[RuntimePart::Signal]),
        (
            RuntimePart::RunTest,
            &[
                RuntimePart::TestRun,
                RuntimePart::FailedRecord,
                RuntimePart::Show,
            ],
        ),
    ];

    pub fn named(name: &str) -> Option<RuntimePart> {
        Self::NAMED
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, part)| *part)
    }

    pub fn required(self) -> &'static [RuntimePart] {
        Self::REQUIRED
            .iter()
            .find(|(part, _)| *part == self)
            .map_or(&[], |(_, required)| *required)
    }
}
