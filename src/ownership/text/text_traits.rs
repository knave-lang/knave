use crate::core::{ClosureKind, PrimitiveType};
use crate::types::{Bound, GenericId, TraitTable, Type};

#[derive(Clone, Copy)]
pub struct TextTraits<'p> {
    table: &'p TraitTable,
}

impl<'p> TextTraits<'p> {
    pub fn new(table: &'p TraitTable) -> Self {
        Self { table }
    }

    pub fn owns_text_for(&self, bound: &Bound) -> bool {
        let anything = |_: GenericId, _: &Bound| true;
        let holds = |ty: Type| self.table.satisfies(bound, &ty, &anything);
        let holds_for_every_reference = holds(Type::shared(Self::function_type()));
        let views_hold = holds(Type::Primitive(PrimitiveType::Str))
            || holds(Type::str_ref()) && !holds_for_every_reference;
        holds(Type::Primitive(PrimitiveType::String)) && !views_hold
    }

    fn function_type() -> Type {
        Type::Function(ClosureKind::Fn, Vec::new(), Box::new(Type::unit()))
    }
}
