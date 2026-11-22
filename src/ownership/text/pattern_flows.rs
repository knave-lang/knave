use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::source::Source;
use super::text_position::TextPositions;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::walk_context::WalkContext;
use crate::core::BindingId;
use crate::source::Span;
use crate::types::{Type, TypedPattern, TypedPatternKind};

pub struct PatternFlows<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    transfer: Transfer,
}

impl<'c, 'w, 'p> PatternFlows<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, matched: &Type) -> Self {
        let transfer = if matched.is_reference() {
            Transfer::Refer
        } else {
            Transfer::Move
        };
        Self { context, transfer }
    }

    pub fn bind(&mut self, pattern: &TypedPattern, value: &TextValue) {
        match &pattern.kind {
            TypedPatternKind::Bind(binder) if TextPositions::holds_text(&binder.ty) => {
                let slot = Slot::from(SlotOwner::Local(binder.binding));
                self.context
                    .receive(&slot, &value.clone().transferred(self.transfer));
            }
            TypedPatternKind::Variant(variant, parts) => {
                for (field, part) in parts.iter().enumerate() {
                    let payload = self.payload(*variant, field, (part, value));
                    self.bind(part, &payload);
                }
            }
            TypedPatternKind::Struct(structure, parts) => self.fields(*structure, parts),
            _ => {}
        }
    }

    fn payload(
        &mut self,
        variant: BindingId,
        field: usize,
        (part, value): (&TypedPattern, &TextValue),
    ) -> TextValue {
        let payloads = self.context.program().types().payloads();
        if let Some(argument) = payloads.argument(variant, field) {
            return Self::part_of(value, argument, part.span);
        }
        match payloads.field(variant, field) {
            Some((owner, ty)) => self.field(&owner, ty, part),
            None => TextValue::default(),
        }
    }

    fn part_of(value: &TextValue, part: usize, span: Span) -> TextValue {
        value
            .nested()
            .iter()
            .filter_map(|(path, node)| Some((path.below(part)?, *node)))
            .fold(TextValue::default(), |read, (path, node)| {
                if path.is_top() {
                    read.merged(TextValue::from(Source::Slot(node), span))
                } else {
                    read.nesting(path, node)
                }
            })
    }

    fn fields(&mut self, structure: BindingId, parts: &[TypedPattern]) {
        let Some((_, definition)) = self
            .context
            .program()
            .types()
            .structure_of(&Type::Named(structure, Vec::new()))
        else {
            return;
        };
        for ((name, ty), part) in definition.fields.iter().zip(parts) {
            let owner = SlotOwner::Field(structure, name.clone());
            let value = self.field(&owner, ty, part);
            self.bind(part, &value);
        }
    }

    fn field(&mut self, owner: &SlotOwner, ty: &Type, part: &TypedPattern) -> TextValue {
        let read = TextRead {
            ty,
            span: part.span,
            transfer: Transfer::Move,
        };
        self.context.read(owner, read)
    }
}

#[cfg(test)]
mod tests {
    use super::{PatternFlows, TextValue};
    use crate::ownership::text::node_id::NodeId;
    use crate::ownership::text::type_path::TypePath;
    use crate::source::{SourceId, Span};

    #[test]
    fn a_part_of_a_value_reads_that_part_s_text() {
        let path = TypePath::default().child(1);
        let value = TextValue::default()
            .nesting(path.child(0), NodeId(4))
            .nesting(path, NodeId(5))
            .nesting(TypePath::default().child(0), NodeId(6));
        let part = PatternFlows::part_of(&value, 1, Span::new(SourceId::new(0), 0..1));
        assert_eq!(part.place(), Some(NodeId(5)));
        assert_eq!(part.nested(), &[(TypePath::default().child(0), NodeId(4))]);
    }
}
