use super::Diagnostic;
use super::distinct::DistinctDiagnostics;

#[derive(Clone, Debug, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.items.extend(other.items);
    }

    pub fn has_errors(&self) -> bool {
        self.items
            .iter()
            .any(|diagnostic| diagnostic.severity().is_error())
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn retain(&mut self, keep: impl FnMut(&Diagnostic) -> bool) {
        self.items.retain(keep);
    }

    pub fn split_off(&mut self, at: usize) -> Self {
        Self {
            items: self.items.split_off(at),
        }
    }

    pub fn map(self, relabel: impl FnMut(Diagnostic) -> Diagnostic) -> Self {
        Self {
            items: self.items.into_iter().map(relabel).collect(),
        }
    }

    pub fn distinct(self) -> Self {
        Self {
            items: DistinctDiagnostics::of(self.items).into_diagnostics(),
        }
    }

    pub fn in_source_order(&self) -> Vec<&Diagnostic> {
        let mut ordered: Vec<&Diagnostic> = self.items.iter().collect();
        ordered.sort_by_key(|diagnostic| {
            let span = diagnostic.primary().span();
            (span.source().index(), span.range().start)
        });
        ordered
    }

    pub fn into_result<T>(self, value: T) -> Result<T, Diagnostics> {
        if self.has_errors() {
            Err(self)
        } else {
            Ok(value)
        }
    }
}

impl FromIterator<Diagnostic> for Diagnostics {
    fn from_iter<I: IntoIterator<Item = Diagnostic>>(diagnostics: I) -> Self {
        Self {
            items: diagnostics.into_iter().collect(),
        }
    }
}

impl From<Diagnostic> for Diagnostics {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            items: vec![diagnostic],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{DiagnosticCode, Label};
    use crate::source::{SourceId, Span};

    struct CollectionFixture;

    impl CollectionFixture {
        fn unbound(name: &str) -> Diagnostic {
            let span = Span::new(SourceId::new(0), 0..4);
            Diagnostic::error(
                DiagnosticCode::UNBOUND_NAME,
                format!("cannot find `{name}` in this scope"),
                Label::new(span, "not found in this scope"),
            )
        }

        fn titles(diagnostics: &Diagnostics) -> Vec<&str> {
            diagnostics.items.iter().map(Diagnostic::title).collect()
        }
    }

    #[test]
    fn collecting_keeps_the_order_of_the_diagnostics() {
        let collected: Diagnostics = ["first", "second"]
            .into_iter()
            .map(CollectionFixture::unbound)
            .collect();
        assert_eq!(
            CollectionFixture::titles(&collected),
            vec![
                "cannot find `first` in this scope",
                "cannot find `second` in this scope"
            ]
        );
    }
}
