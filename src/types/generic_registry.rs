use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::ty::{Bound, GenericId, GenericParam};
use super::unifier::{LiteralKind, Unifier};

#[derive(Default)]
pub(super) struct GenericRegistry {
    params: FxHashMap<GenericId, GenericParam>,
    associated: FxHashMap<GenericId, Rc<str>>,
    literals: FxHashMap<GenericId, LiteralKind>,
}

pub(super) enum GenericRole {
    AssociatedType(Rc<str>),
    Literal(LiteralKind),
}

pub(super) struct GenericTables {
    pub(super) params: FxHashMap<GenericId, GenericParam>,
    pub(super) associated: FxHashMap<GenericId, Rc<str>>,
    pub(super) literals: FxHashMap<GenericId, LiteralKind>,
}

impl GenericRegistry {
    pub(super) fn declare(
        &mut self,
        name: &str,
        bounds: Vec<Bound>,
        unifier: &mut Unifier,
    ) -> GenericParam {
        let param = GenericParam {
            id: GenericId::new(self.params.len()),
            name: name.into(),
            bounds,
        };
        self.redeclare(&param, unifier);
        param
    }

    pub(super) fn redeclare(&mut self, param: &GenericParam, unifier: &mut Unifier) {
        self.params.insert(param.id, param.clone());
        unifier.register_generic(param.id, param.bounds.clone());
    }

    pub(super) fn assign(&mut self, generic: GenericId, role: GenericRole) {
        match role {
            GenericRole::AssociatedType(name) => {
                self.associated.insert(generic, name);
            }
            GenericRole::Literal(literal) => {
                self.literals.insert(generic, literal);
            }
        }
    }

    pub(super) fn params(&self) -> &FxHashMap<GenericId, GenericParam> {
        &self.params
    }

    pub(super) fn literal(&self, generic: GenericId) -> Option<LiteralKind> {
        self.literals.get(&generic).copied()
    }

    pub(super) fn into_tables(self) -> GenericTables {
        GenericTables {
            params: self.params,
            associated: self.associated,
            literals: self.literals,
        }
    }
}
