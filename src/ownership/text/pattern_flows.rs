use super::slot::Slot;
use super::slot_listing::SlotListing;
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
    matched: &'c Type,
}

impl<'c, 'w, 'p> PatternFlows<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, matched: &'c Type) -> Self {
        Self { context, matched }
    }

    pub fn bind_arm(
        mut self,
        listing: &mut SlotListing,
        pattern: &TypedPattern,
        value: &TextValue,
    ) {
        for binder in pattern.binders() {
            listing.list(&SlotOwner::Local(binder.binding), &binder.ty);
        }
        let matched = self.matched;
        self.bind(pattern, value, (matched, Transfer::Move));
    }

    fn bind(&mut self, pattern: &TypedPattern, value: &TextValue, (ty, outer): (&Type, Transfer)) {
        let transfer = if ty.is_reference() {
            Transfer::Refer
        } else {
            outer
        };
        match &pattern.kind {
            TypedPatternKind::Bind(binder) if TextPositions::holds_text(&binder.ty) => {
                let slot = Slot::from(SlotOwner::Local(binder.binding));
                self.context
                    .receive(&slot, &value.clone().transferred(outer));
            }
            TypedPatternKind::Variant(variant, parts) => {
                for (field, part) in parts.iter().enumerate() {
                    let (payload, part_type) = self.payload(*variant, field, (part, value, ty));
                    self.bind(part, &payload, (&part_type, transfer));
                }
            }
            TypedPatternKind::Struct(structure, parts) => self.fields(*structure, parts, transfer),
            _ => {}
        }
    }

    fn payload(
        &mut self,
        variant: BindingId,
        field: usize,
        (part, value, ty): (&TypedPattern, &TextValue, &Type),
    ) -> (TextValue, Type) {
        let payloads = self.context.program().types().payloads();
        if let Some(argument) = payloads.argument(variant, field) {
            let part_type = Self::argument_of(ty, argument).unwrap_or_else(Type::unit);
            return (ValueParts::of(value).part(argument, part.span), part_type);
        }
        match payloads.field(variant, field) {
            Some((owner, field_type)) => (self.field(&owner, field_type, part), field_type.clone()),
            None => (TextValue::default(), Type::unit()),
        }
    }

    fn argument_of(ty: &Type, argument: usize) -> Option<Type> {
        match ty {
            Type::Ref(_, inner) | Type::Mut(_, inner) => Self::argument_of(inner, argument),
            Type::Named(_, arguments) => arguments.get(argument).cloned(),
            _ => None,
        }
    }

    fn fields(&mut self, structure: BindingId, parts: &[TypedPattern], transfer: Transfer) {
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
            self.bind(part, &value, (ty, transfer));
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
