use crate::diagnostic::{Diagnostic, Diagnostics};

#[derive(Clone, Copy)]
pub(super) struct ProblemMark {
    position: usize,
}

impl ProblemMark {
    pub(super) fn end_of(list: &impl ProblemList) -> Self {
        Self {
            position: list.count(),
        }
    }

    pub(super) fn since<L: ProblemList>(self, list: &mut L) -> L {
        list.split_at_mark(self.position)
    }
}

pub(super) trait ProblemList {
    fn count(&self) -> usize;

    fn split_at_mark(&mut self, position: usize) -> Self;
}

impl ProblemList for Vec<Diagnostic> {
    fn count(&self) -> usize {
        self.len()
    }

    fn split_at_mark(&mut self, position: usize) -> Self {
        self.split_off(position)
    }
}

impl ProblemList for Diagnostics {
    fn count(&self) -> usize {
        self.len()
    }

    fn split_at_mark(&mut self, position: usize) -> Self {
        self.split_off(position)
    }
}

#[cfg(test)]
mod tests {
    use super::ProblemMark;
    use crate::diagnostic::{Diagnostic, Diagnostics};
    use crate::expand::problems::ExpandProblems;
    use crate::source::{SourceId, Span};

    struct MarkFixture;

    impl MarkFixture {
        fn problem() -> Diagnostic {
            ExpandProblems::empty_call(Span::new(SourceId::new(0), 0..2))
        }

        fn errors(count: usize) -> Vec<Diagnostic> {
            (0..count).map(|_| Self::problem()).collect()
        }
    }

    #[test]
    fn a_mark_takes_the_errors_added_after_it() {
        let mut errors = MarkFixture::errors(1);
        let mark = ProblemMark::end_of(&errors);
        errors.extend(MarkFixture::errors(2));
        assert_eq!(mark.since(&mut errors).len(), 2);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn a_mark_with_nothing_after_it_takes_nothing() {
        let mut errors = MarkFixture::errors(2);
        let mark = ProblemMark::end_of(&errors);
        assert!(mark.since(&mut errors).is_empty());
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn a_mark_takes_the_diagnostics_added_after_it() {
        let mut diagnostics = Diagnostics::default();
        diagnostics.push(MarkFixture::problem());
        let mark = ProblemMark::end_of(&diagnostics);
        diagnostics.push(MarkFixture::problem());
        assert_eq!(mark.since(&mut diagnostics).len(), 1);
        assert_eq!(diagnostics.len(), 1);
    }
}
