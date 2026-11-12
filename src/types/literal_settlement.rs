use super::selection::Selection;
use super::traits::TraitTable;
use super::ty::{Bound, Type};
use super::unifier::Unifier;

pub struct LiteralSettlement<'c> {
    unifier: &'c Unifier,
    traits: &'c TraitTable,
}

impl<'c> LiteralSettlement<'c> {
    pub fn new(unifier: &'c Unifier, traits: &'c TraitTable) -> Self {
        Self { unifier, traits }
    }

    pub fn select(&self, bound: &Bound, implementor: &Type) -> Selection {
        match self.traits.rows(bound.binding).select(bound, implementor) {
            Selection::Undecided => self.settle(bound, implementor),
            decided => decided,
        }
    }

    fn settle(&self, bound: &Bound, implementor: &Type) -> Selection {
        let opens_only_literals = bound.argument.as_ref().is_some_and(|argument| {
            let resolved = self.unifier.resolve(argument);
            resolved.has_vars() && self.admits_only_literals(&resolved)
        });
        if !opens_only_literals {
            return Selection::Undecided;
        }
        self.traits
            .rows(bound.binding)
            .settle_lone_row(bound, implementor)
            .unwrap_or(Selection::Undecided)
    }

    fn admits_only_literals(&self, value: &Type) -> bool {
        match value {
            Type::Var(var) => self.unifier.literal_kind(*var).is_some(),
            _ => value
                .parts()
                .into_iter()
                .all(|part| self.admits_only_literals(part)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleId, PrimitiveType};
    use crate::types::traits::{Implementor, RustTrait};
    use crate::types::ty::Scheme;
    use crate::types::unifier::LiteralKind;

    struct LiteralSettlementFixture {
        unifier: Unifier,
        traits: TraitTable,
        holds: BindingId,
        list: BindingId,
    }

    impl LiteralSettlementFixture {
        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let holds = bindings.add(BindingInfo::new(
                "Holds".into(),
                BindingKind::Trait(ModuleId::new(0)),
                None,
            ));
            let list = bindings.add(BindingInfo::new(
                "list".into(),
                BindingKind::TypeVariable,
                None,
            ));
            let mut fixture = Self {
                unifier: Unifier::default(),
                traits: TraitTable::default(),
                holds,
                list,
            };
            fixture.add_holds();
            fixture
        }

        fn add_holds(&mut self) {
            let byte = Type::Primitive(PrimitiveType::U8);
            let rows = [
                (PrimitiveType::I64, byte.clone()),
                (PrimitiveType::U16, Type::Named(self.list, vec![byte])),
            ];
            self.traits.add(
                self.holds,
                RustTrait {
                    path: "Holds".into(),
                    implementors: rows
                        .into_iter()
                        .map(|(implementor, second)| Implementor {
                            scheme: Scheme::monomorphic(Type::Primitive(implementor)),
                            argument: Some(second),
                        })
                        .collect(),
                    help: None,
                    supertraits: Vec::new(),
                    associated: None,
                },
            );
        }

        fn settled(&self, implementor: PrimitiveType, argument: Type) -> Option<Type> {
            let bound = Bound::with_argument(self.holds, argument);
            match LiteralSettlement::new(&self.unifier, &self.traits)
                .select(&bound, &Type::Primitive(implementor))
            {
                Selection::Implemented { argument, .. } => argument,
                Selection::Undecided | Selection::Missing => None,
            }
        }
    }

    #[test]
    fn a_literal_takes_the_type_of_the_one_fitting_row() {
        let mut fixture = LiteralSettlementFixture::new();
        let literal = fixture.unifier.fresh_literal(LiteralKind::Integer);
        assert_eq!(
            fixture.settled(PrimitiveType::I64, literal),
            Some(Type::Primitive(PrimitiveType::U8))
        );
    }

    #[test]
    fn a_variable_that_is_not_a_literal_stays_open() {
        let mut fixture = LiteralSettlementFixture::new();
        let open = fixture.unifier.fresh();
        assert_eq!(fixture.settled(PrimitiveType::I64, open), None);
    }

    #[test]
    fn a_literal_inside_the_argument_takes_its_part_of_the_row() {
        let mut fixture = LiteralSettlementFixture::new();
        let literal = fixture.unifier.fresh_literal(LiteralKind::Integer);
        let list = Type::Named(fixture.list, vec![literal]);
        assert_eq!(
            fixture.settled(PrimitiveType::U16, list),
            Some(Type::Named(
                fixture.list,
                vec![Type::Primitive(PrimitiveType::U8)]
            ))
        );
    }
}
