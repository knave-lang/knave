use super::read_back_writer::ReadBackWriter;
use super::spanned_forms::SpannedForms;
use crate::core::{
    BindingArena, BindingId, BindingKind, Pattern, PatternArguments, PatternKind, PatternPart,
};
use crate::reader::Form;
use crate::source::Span;

pub struct PatternForms<'w, 's> {
    arena: &'w BindingArena,
    writer: ReadBackWriter<'w, 's>,
}

type PatternHead<'n> = (BindingId, Option<&'n str>);

impl<'w, 's> PatternForms<'w, 's> {
    const WILDCARD: &'static str = "_";

    pub fn new(arena: &'w BindingArena, writer: ReadBackWriter<'w, 's>) -> Self {
        Self { arena, writer }
    }

    pub fn pattern(&mut self, pattern: &Pattern) -> Form {
        let span = pattern.span;
        match &pattern.kind {
            PatternKind::Wildcard => self.writer.leaf(SpannedForms::symbol(Self::WILDCARD, span)),
            PatternKind::Bind(binding) | PatternKind::BareName { binding, .. } => {
                self.name(*binding, span)
            }
            PatternKind::Variant(head, arguments) | PatternKind::Struct(head, arguments) => {
                self.headed((*head, None), arguments, span)
            }
            PatternKind::ShortVariant {
                name,
                candidates,
                arguments,
            } => {
                let candidate = candidates
                    .first()
                    .expect("expansion gives every short variant a candidate");
                self.headed((*candidate, Some(name)), arguments, span)
            }
            PatternKind::Literal(literal) => self.writer.leaf(SpannedForms::literal(literal, span)),
        }
    }

    fn name(&mut self, binding: BindingId, span: Span) -> Form {
        if self.arena.get(binding).kind() == BindingKind::Local {
            self.writer.binder(binding, span)
        } else {
            self.writer.reference(binding, None, span)
        }
    }

    fn headed(&mut self, head: PatternHead<'_>, arguments: &PatternArguments, span: Span) -> Form {
        let (binding, written) = head;
        let row = self.writer.open();
        let mut parts = vec![self.writer.reference(binding, written, span)];
        self.push_arguments(&mut parts, arguments);
        self.writer.close(row, parts, span)
    }

    fn push_arguments(&mut self, parts: &mut Vec<Form>, arguments: &PatternArguments) {
        for part in &arguments.parts {
            match part {
                PatternPart::Positional(pattern) => parts.push(self.pattern(pattern)),
                PatternPart::Labeled(label, pattern) => {
                    let spelled = SpannedForms::symbol(&label.spelled(&label.name), label.span);
                    parts.push(self.writer.leaf(spelled));
                    parts.extend(pattern.iter().map(|pattern| self.pattern(pattern)));
                }
            }
        }
    }
}
