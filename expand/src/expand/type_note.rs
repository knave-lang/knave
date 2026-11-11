use super::problems::ExpandProblems;
use crate::diagnostic::Diagnostic;
use crate::reader::Symbol;

#[derive(Clone, Copy, Default)]
pub(super) struct TypeNote {
    shown: bool,
}

impl TypeNote {
    pub(super) fn shown() -> Self {
        Self { shown: true }
    }

    pub(super) fn noted(&mut self, unbound: Diagnostic, type_name: Option<&Symbol>) -> Diagnostic {
        let Some(symbol) = type_name.filter(|_| !self.shown) else {
            return unbound;
        };
        self.shown = true;
        unbound.with_note(ExpandProblems::type_in_value_note(symbol.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::TypeNote;
    use crate::diagnostic::Diagnostic;
    use crate::expand::problems::ExpandProblems;
    use crate::reader::Symbol;
    use crate::source::{SourceId, Span};

    struct NoteFixture {
        name: Symbol,
    }

    impl NoteFixture {
        fn new() -> Self {
            Self {
                name: Symbol::new("point"),
            }
        }

        fn unbound() -> Diagnostic {
            ExpandProblems::empty_call(Span::new(SourceId::new(0), 0..5))
        }

        fn notes_naming_a_type(&self, note: &mut TypeNote) -> usize {
            note.noted(Self::unbound(), Some(&self.name)).notes().len()
        }

        fn notes_naming_no_type(note: &mut TypeNote) -> usize {
            note.noted(Self::unbound(), None).notes().len()
        }
    }

    #[test]
    fn a_name_of_a_type_gets_the_note() {
        let fixture = NoteFixture::new();
        assert_eq!(fixture.notes_naming_a_type(&mut TypeNote::default()), 1);
    }

    #[test]
    fn a_name_that_is_no_type_gets_no_note() {
        assert_eq!(
            NoteFixture::notes_naming_no_type(&mut TypeNote::default()),
            0
        );
    }

    #[test]
    fn the_note_is_shown_once() {
        let fixture = NoteFixture::new();
        let mut note = TypeNote::default();
        fixture.notes_naming_a_type(&mut note);
        assert_eq!(fixture.notes_naming_a_type(&mut note), 0);
    }

    #[test]
    fn a_note_already_shown_is_not_shown_again() {
        let fixture = NoteFixture::new();
        assert_eq!(fixture.notes_naming_a_type(&mut TypeNote::shown()), 0);
    }
}
