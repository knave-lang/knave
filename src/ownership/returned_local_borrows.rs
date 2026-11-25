use crate::diagnostic::{DiagnosticCode, Diagnostics};
use crate::source::Span;

pub struct ReturnedLocalBorrows(Vec<Span>);

impl ReturnedLocalBorrows {
    pub fn of(diagnostics: &Diagnostics) -> Self {
        Self(
            diagnostics
                .into_iter()
                .filter(|diagnostic| diagnostic.code() == Some(&DiagnosticCode::LOCAL_BORROW))
                .map(|diagnostic| diagnostic.primary().span())
                .collect(),
        )
    }

    pub fn any_within(&self, value: Span) -> bool {
        self.0.iter().any(|borrow| value.contains(*borrow))
    }
}
