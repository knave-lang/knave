use crate::diagnostic::Diagnostics;
use crate::source::Span;

pub struct LabeledSpans(Vec<Span>);

impl LabeledSpans {
    pub fn of(diagnostics: &Diagnostics) -> Self {
        Self(
            diagnostics
                .into_iter()
                .flat_map(|diagnostic| diagnostic.labels().map(|label| label.span()))
                .collect(),
        )
    }

    pub fn cover_any(&self, spans: &[Span]) -> bool {
        self.0
            .iter()
            .any(|label| spans.iter().any(|span| label.contains(*span)))
    }
}
