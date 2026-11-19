use super::release_version::ReleaseVersion;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
use crate::language::Language;
use crate::source::Span;

pub struct CompilerVersionProblems;

impl CompilerVersionProblems {
    pub const EXPECTED_VERSION: &str = "a release version such as `\"0.18.0\"`";
    const VERSION_SHAPE_HELP: &str =
        "write it as three numbers without leading zeros, such as `\"0.18.0\"`";

    pub fn malformed_version(mismatch: Diagnostic) -> Diagnostic {
        mismatch.with_help(Self::VERSION_SHAPE_HELP)
    }

    pub fn older_compiler(
        recorded: ReleaseVersion,
        running: ReleaseVersion,
        span: Span,
    ) -> Diagnostic {
        Diagnostic::error(
            DiagnosticCode::COMPILER_OLDER_THAN_PROJECT,
            format!("the project uses Knave {recorded}, and this is Knave {running}"),
            Label::new(span, "the project's Knave version"),
        )
        .with_help(format!(
            "install that version with `cargo install {} --version {recorded} --locked`",
            Language::NAME
        ))
    }

    pub fn newer_compiler(
        recorded: ReleaseVersion,
        running: ReleaseVersion,
        span: Span,
    ) -> Diagnostic {
        Diagnostic::warning(
            DiagnosticCode::COMPILER_NEWER_THAN_PROJECT,
            format!("the project records Knave {recorded}, and this is Knave {running}"),
            Label::new(span, "recorded by an earlier Knave"),
        )
        .with_help(format!(
            "run `{} upgrade` to record Knave {running} and rewrite the files Knave wrote",
            Language::NAME
        ))
    }
}
