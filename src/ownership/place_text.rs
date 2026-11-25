use super::assigned_place::AssignedPlace;
use crate::core::{BindingArena, BindingId, Literal, ReferenceKind};
use crate::types::{TypedAssignment, TypedExpr, TypedExprKind, TypedLeaf, TypedStructExpr};

pub struct PlaceText<'a> {
    bindings: &'a BindingArena,
}

impl<'a> PlaceText<'a> {
    pub fn new(bindings: &'a BindingArena) -> Self {
        Self { bindings }
    }

    pub fn assigned(&self, assignment: &TypedAssignment) -> Option<AssignedPlace<'a>> {
        let mut place = self.place(&assignment.place().value)?;
        for key in assignment.keys() {
            let key = self.key(&key.value, &mut place.locals)?;
            place.written = format!("(at {} {key})", place.written);
        }
        Some(place)
    }

    fn place(&self, place: &TypedExpr) -> Option<AssignedPlace<'a>> {
        match &place.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => Some(AssignedPlace {
                written: self.name(*binding).to_string(),
                locals: vec![self.name(*binding)],
            }),
            TypedExprKind::Struct(TypedStructExpr::Field(owner, label)) => {
                self.wrapped(owner, |inner| format!("(:{} {inner})", label.name))
            }
            TypedExprKind::Borrow(ReferenceKind::Mutable, inner) => {
                self.wrapped(inner, |inner| format!("(mut {inner})"))
            }
            _ => None,
        }
    }

    fn wrapped(
        &self,
        inner: &TypedExpr,
        wrap: impl FnOnce(&str) -> String,
    ) -> Option<AssignedPlace<'a>> {
        let mut place = self.place(inner)?;
        place.written = wrap(&place.written);
        Some(place)
    }

    fn key(&self, key: &TypedExpr, locals: &mut Vec<&'a str>) -> Option<String> {
        match &key.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => {
                locals.push(self.name(*binding));
                Some(self.name(*binding).to_string())
            }
            TypedExprKind::Leaf(TypedLeaf::Literal(Literal::Number(number))) => {
                Some(number.text().to_string())
            }
            TypedExprKind::Leaf(TypedLeaf::Literal(Literal::String(text))) => {
                Some(format!("{text:?}"))
            }
            _ => None,
        }
    }

    fn name(&self, binding: BindingId) -> &'a str {
        self.bindings.get(binding).name()
    }
}
