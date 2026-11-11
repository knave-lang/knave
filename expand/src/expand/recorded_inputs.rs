use std::rc::Rc;

use super::module_notes::{ModuleNotes, NotesCheckpoint};
use crate::core::Namespace;
use crate::source::{SourceId, Span};

#[derive(Default)]
pub(super) struct RecordedInputs {
    inputs: Vec<Span>,
    bound: Vec<BoundName>,
    loaded: Vec<LoadedModule>,
    registering: Option<Span>,
    noting: Vec<Span>,
}

pub(super) struct Registration {
    outer: Option<Span>,
    notes: NotesCheckpoint,
}

pub(super) struct InputRecord {
    bound: Vec<BoundName>,
    adds_notes: bool,
}

impl InputRecord {
    pub(super) fn is_lone_document(&self, left_out: &[Span]) -> bool {
        self.adds_notes && left_out.is_empty()
    }

    pub(super) fn into_bound(self) -> Vec<BoundName> {
        self.bound
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundName {
    pub name: Rc<str>,
    pub namespace: Namespace,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadedModule {
    pub source: SourceId,
    pub input: Span,
}

impl BoundName {
    pub fn names_the_same(&self, other: &BoundName) -> bool {
        self.name == other.name && self.namespace == other.namespace
    }
}

impl RecordedInputs {
    pub(super) fn new(inputs: Vec<Span>) -> Self {
        Self {
            inputs,
            ..Self::default()
        }
    }

    pub(super) fn note_bound(&mut self, span: Span, name: impl FnOnce() -> Option<BoundName>) {
        if self.surrounds(span) {
            self.bound.extend(name());
        }
    }

    pub(super) fn enter(&mut self, written: Span, notes: &ModuleNotes) -> Registration {
        let outer = self.registering;
        if self.inputs.contains(&written) {
            self.registering = Some(written);
        }
        Registration {
            outer,
            notes: notes.checkpoint(),
        }
    }

    pub(super) fn leave(&mut self, registration: Registration, notes: &ModuleNotes) {
        if registration.notes != notes.checkpoint() {
            self.noting.extend(self.registering);
        }
        self.registering = registration.outer;
    }

    pub(super) fn note_loaded(&mut self, source: SourceId) {
        if let Some(input) = self.registering {
            self.loaded.push(LoadedModule { source, input });
        }
    }

    pub(super) fn of(&self, span: Span) -> InputRecord {
        InputRecord {
            bound: self
                .bound
                .iter()
                .filter(|name| span.contains(name.span))
                .cloned()
                .collect(),
            adds_notes: self.noting.contains(&span),
        }
    }

    pub(super) fn loaded(&self) -> &[LoadedModule] {
        &self.loaded
    }

    fn surrounds(&self, span: Span) -> bool {
        self.inputs.iter().any(|input| input.contains(span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ModuleId;
    use crate::reader::{Form, FormKind, Symbol};

    struct RecordFixture;

    impl RecordFixture {
        const INPUT: usize = 0;
        const OUTSIDE: usize = 20;

        fn span(start: usize) -> Span {
            Span::new(SourceId::new(0), start..start + 1)
        }

        fn name(span: Span) -> Option<BoundName> {
            Some(BoundName {
                name: "bound".into(),
                namespace: Namespace::Value,
                span,
            })
        }

        fn noted() -> ModuleNotes {
            let mut notes = ModuleNotes::default();
            let note = Form::new(
                FormKind::Symbol(Symbol::new("note")),
                Self::span(Self::INPUT),
            );
            notes
                .add(ModuleId::new(0), "key", note)
                .unwrap_or_else(|_| panic!("one note fits"));
            notes
        }

        fn recording() -> RecordedInputs {
            RecordedInputs::new(vec![Self::span(Self::INPUT)])
        }
    }

    #[test]
    fn a_name_bound_outside_every_input_is_not_recorded() {
        let mut recorded = RecordFixture::recording();
        let outside = RecordFixture::span(RecordFixture::OUTSIDE);
        recorded.note_bound(outside, || RecordFixture::name(outside));
        let covering = Span::new(SourceId::new(0), 0..RecordFixture::OUTSIDE + 1);
        assert!(recorded.of(covering).into_bound().is_empty());
    }

    #[test]
    fn an_input_that_adds_a_module_note_is_recorded() {
        let mut recorded = RecordFixture::recording();
        let input = RecordFixture::span(RecordFixture::INPUT);
        let registration = recorded.enter(input, &ModuleNotes::default());
        recorded.leave(registration, &RecordFixture::noted());
        assert!(recorded.of(input).is_lone_document(&[]));
    }

    #[test]
    fn an_input_that_adds_no_module_note_is_not_recorded() {
        let mut recorded = RecordFixture::recording();
        let input = RecordFixture::span(RecordFixture::INPUT);
        let unnoted = ModuleNotes::default();
        let registration = recorded.enter(input, &unnoted);
        recorded.leave(registration, &unnoted);
        assert!(!recorded.of(input).is_lone_document(&[]));
    }

    #[test]
    fn an_input_that_leaves_out_an_expression_is_not_a_lone_document() {
        let mut recorded = RecordFixture::recording();
        let input = RecordFixture::span(RecordFixture::INPUT);
        let registration = recorded.enter(input, &ModuleNotes::default());
        recorded.leave(registration, &RecordFixture::noted());
        let left_out = RecordFixture::span(RecordFixture::OUTSIDE);
        assert!(!recorded.of(input).is_lone_document(&[left_out]));
    }

    #[test]
    fn a_module_loaded_while_an_input_registers_belongs_to_the_input() {
        let mut recorded = RecordFixture::recording();
        let input = RecordFixture::span(RecordFixture::INPUT);
        let notes = ModuleNotes::default();
        let outer = recorded.enter(input, &notes);
        let nested = recorded.enter(RecordFixture::span(RecordFixture::OUTSIDE), &notes);
        recorded.note_loaded(SourceId::new(1));
        recorded.leave(nested, &notes);
        recorded.leave(outer, &notes);
        recorded.note_loaded(SourceId::new(2));
        assert_eq!(
            recorded.loaded(),
            [LoadedModule {
                source: SourceId::new(1),
                input
            }]
        );
    }
}
