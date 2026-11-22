use super::slot::Slot;
use super::slot_owner::SlotOwner;
use super::text_read::TextRead;
use super::text_value::TextValue;
use super::transfer::Transfer;
use super::type_path::TypePath;
use super::walk_context::WalkContext;
use crate::core::{BindingId, FieldLabel};
use crate::types::{Type, TypedExpr};

pub struct ConstructionFlows<'c, 'w, 'p> {
    context: &'c mut WalkContext<'w, 'p>,
    expr: &'c TypedExpr,
}

impl<'c, 'w, 'p> ConstructionFlows<'c, 'w, 'p> {
    pub fn new(context: &'c mut WalkContext<'w, 'p>, expr: &'c TypedExpr) -> Self {
        Self { context, expr }
    }

    pub fn fields(self, structure: BindingId, fields: &[(&FieldLabel, TextValue)]) -> TextValue {
        for (label, value) in fields {
            let owner = SlotOwner::Field(structure, label.name.clone());
            self.context.receive(&Slot::from(owner), value);
        }
        TextValue::default()
    }

    pub fn field_read(self, structure_type: &Type, label: &FieldLabel) -> TextValue {
        match self.context.program().types().structure_of(structure_type) {
            Some((structure, _)) => {
                let field = SlotOwner::Field(structure, label.name.clone());
                self.read(&field, Transfer::Refer)
            }
            None => TextValue::default(),
        }
    }

    pub fn variant(self, variant: BindingId, payloads: &[(usize, TextValue)]) -> TextValue {
        for (field, value) in payloads {
            let slot = self.payload_slot(variant, *field);
            self.context.receive(&slot, value);
        }
        let value = SlotOwner::Value(self.expr.span);
        self.read(&value, Transfer::Move)
    }

    fn payload_slot(&self, variant: BindingId, field: usize) -> Slot {
        let payloads = self.context.program().types().payloads();
        match payloads.argument(variant, field) {
            Some(argument) => Slot::new(
                SlotOwner::Value(self.expr.span),
                TypePath::default().child(argument),
            ),
            None => Slot::from(
                payloads
                    .field(variant, field)
                    .map(|(owner, _)| owner)
                    .expect("a built variant has a declared case"),
            ),
        }
    }

    fn read(self, owner: &SlotOwner, transfer: Transfer) -> TextValue {
        let read = TextRead {
            ty: &self.expr.ty,
            span: self.expr.span,
            transfer,
        };
        self.context.read(owner, read)
    }
}
