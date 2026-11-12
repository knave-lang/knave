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
        match self.traits.select(bound, implementor) {
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
