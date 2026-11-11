use knave::diagnostic::{Diagnostic, DiagnosticRenderer, Diagnostics};
use knave::expand::ExpandProblems;
use knave::interpret::HostProblems;
use knave::source::{SourceId, SourceMap, Span};

struct DiagnosticFixture {
    sources: SourceMap,
}

impl DiagnosticFixture {
    const SOURCE_NAME: &'static str = "sample.knv";
    const SOURCE_TEXT: &'static str = "origin";

    fn new() -> Self {
        let mut sources = SourceMap::default();
        sources.add(Self::SOURCE_NAME, Self::SOURCE_TEXT);
        Self { sources }
    }

    fn render(&self, diagnostic: Diagnostic) -> String {
        let mut diagnostics = Diagnostics::default();
        diagnostics.push(diagnostic);
        DiagnosticRenderer::plain(&self.sources).render_all(&diagnostics)
    }

    fn span(&self) -> Span {
        Span::new(SourceId::new(0), 0..Self::SOURCE_TEXT.len())
    }
}

#[test]
fn not_at_compile_time_has_a_snapshot() {
    let fixture = DiagnosticFixture::new();
    let diagnostic = ExpandProblems::not_at_compile_time("origin", fixture.span());
    insta::assert_snapshot!(fixture.render(diagnostic));
}

#[test]
fn missing_host_group_of_a_builtin_has_a_snapshot() {
    let fixture = DiagnosticFixture::new();
    let diagnostic = HostProblems::missing_group_of_builtin("origin", fixture.span());
    insta::assert_snapshot!(fixture.render(diagnostic));
}

#[test]
fn missing_global_values_has_a_snapshot() {
    let fixture = DiagnosticFixture::new();
    let diagnostic = HostProblems::missing_global_values(fixture.span());
    insta::assert_snapshot!(fixture.render(diagnostic));
}
