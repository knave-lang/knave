use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_position::TextPositions;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::value_parts::ValueParts;
use super::walk_context::WalkContext;
use crate::core::BindingId;
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
            return ValueParts::of(value).part(argument, part.span);
        }
        match payloads.field(variant, field) {
            Some((owner, ty)) => self.field(&owner, ty, part),
            None => TextValue::default(),
        }
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
        self.context.read(&Slot::from(owner.clone()), read)
    }
}
