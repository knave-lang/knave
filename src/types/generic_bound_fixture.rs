use rustc_hash::FxHashMap;

use super::bound_proof::BoundProof;
use super::generic_bound_check::GenericBoundCheck;
use super::generic_bound_tables::GenericBoundTables;
use super::implied_bounds::ImpliedBounds;
use super::unifier::Unifier;
use super::{
    Bound, GenericId, GenericParam, Implementor, LoweredRows, RustTrait, Scheme, TraitTable, Type,
};
use crate::core::{BindingId, PrimitiveType};
use crate::source::{SourceId, Span};

pub(super) struct GenericBoundFixture {
    unifier: Unifier,
    implied_bounds: ImpliedBounds,
    traits: TraitTable,
    generics: FxHashMap<GenericId, GenericParam>,
}

impl GenericBoundFixture {
    pub(super) const ORDERED: BindingId = BindingId::from_index(0);
    pub(super) const EQUAL: BindingId = BindingId::from_index(1);
    pub(super) const SHOWN: BindingId = BindingId::from_index(2);
    pub(super) const PAIRED: BindingId = BindingId::from_index(3);

    pub(super) fn new() -> Self {
        let mut traits = TraitTable::default();
        traits.add(
            Self::ORDERED,
            Self::rust_trait(Vec::new(), vec![Bound::plain(Self::EQUAL)]),
        );
        traits.add(
            Self::EQUAL,
            Self::rust_trait(vec![Self::implementor(PrimitiveType::I64)], Vec::new()),
        );
        traits.add(
            Self::SHOWN,
            Self::rust_trait(vec![Self::implementor(PrimitiveType::I64)], Vec::new()),
        );
        traits.add(
            Self::PAIRED,
            Self::rust_trait(vec![Self::paired_implementor()], Vec::new()),
        );
        Self {
            unifier: Unifier::default(),
            implied_bounds: ImpliedBounds::default(),
            traits,
            generics: FxHashMap::default(),
        }
    }

    pub(super) fn declare(&mut self, bounds: Vec<Bound>) {
        let generic = Self::generic();
        self.unifier.register_generic(generic, bounds.clone());
        self.generics.insert(
            generic,
            GenericParam {
                id: generic,
                name: "T".into(),
                bounds,
            },
        );
    }

    pub(super) fn mark_impl_generic(&mut self) {
        let param = GenericParam {
            id: Self::generic(),
            name: "T".into(),
            bounds: Vec::new(),
        };
        self.implied_bounds.mark_impl_generics(&[param]);
    }

    pub(super) fn settle(&mut self, bound: Bound) -> BoundProof {
        let tables = GenericBoundTables {
            traits: &self.traits,
            generics: &self.generics,
        };
        GenericBoundCheck::new(&mut self.unifier, &mut self.implied_bounds, tables).settle(
            Self::generic(),
            bound,
            Self::span(),
        )
    }

    pub(super) fn implied_traits(&self) -> Vec<BindingId> {
        (self.implied_bounds.of(Self::generic()))
            .map(|bound| bound.binding)
            .collect()
    }

    pub(super) fn var(&mut self) -> Type {
        self.unifier.fresh()
    }

    pub(super) fn resolve(&self, ty: &Type) -> Type {
        self.unifier.resolve(ty)
    }

    pub(super) fn int() -> Type {
        Type::Primitive(PrimitiveType::I64)
    }

    fn generic() -> GenericId {
        GenericId::new(0)
    }

    fn span() -> Span {
        Span::new(SourceId::new(0), 0..0)
    }

    fn rust_trait(implementors: Vec<Implementor>, supertraits: Vec<Bound>) -> RustTrait {
        RustTrait::written(
            "Shown".into(),
            None,
            LoweredRows::new(implementors, supertraits),
        )
    }

    fn paired_implementor() -> Implementor {
        Implementor {
            argument: Some(Self::int()),
            ..Self::implementor(PrimitiveType::I64)
        }
    }

    fn implementor(body: PrimitiveType) -> Implementor {
        Implementor {
            scheme: Scheme {
                lifetimes: Vec::new(),
                params: Vec::new(),
                body: Type::Primitive(body),
            },
            argument: None,
        }
    }
}
