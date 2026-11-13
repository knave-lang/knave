use rustc_hash::{FxHashMap, FxHashSet};

use super::declarations::Declarations;
use super::size::TypeSizes;
use super::traits::TraitTable;
use super::ty::{Bound, GenericId, GenericParam, Type};
use crate::core::{BindingArena, BindingId, CopyTrait};

#[derive(Clone, Default)]
pub struct InferredCopies {
    parameters: FxHashSet<BindingId>,
}

impl InferredCopies {
    pub fn insert(&mut self, binding: BindingId) {
        self.parameters.insert(binding);
    }

    pub fn extend(&mut self, other: InferredCopies) {
        self.parameters.extend(other.parameters);
    }

    pub fn contains(&self, binding: BindingId) -> bool {
        self.parameters.contains(&binding)
    }

    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }
}

pub struct CopyRules<'a> {
    traits: &'a TraitTable,
    generics: &'a FxHashMap<GenericId, GenericParam>,
    sizes: TypeSizes<'a>,
    copy_traits: Vec<BindingId>,
}

impl<'a> CopyRules<'a> {
    pub fn new(
        declarations: &'a Declarations,
        generics: &'a FxHashMap<GenericId, GenericParam>,
    ) -> Self {
        let traits = declarations.traits().table();
        Self {
            traits,
            generics,
            sizes: TypeSizes::new(declarations.types().structs(), declarations.types().enums()),
            copy_traits: traits.with_paths(&[CopyTrait::PATH]),
        }
    }

    pub fn copy_trait_name<'b>(&self, bindings: &'b BindingArena) -> &'b str {
        let binding = self
            .copy_traits
            .first()
            .expect("the prelude defines a copy trait");
        bindings.get(*binding).name()
    }

    pub fn is_copy(&self, value: &Type) -> bool {
        self.copy_traits.iter().any(|copy| {
            self.traits
                .satisfies(&Bound::plain(*copy), value, &|generic, _| {
                    self.generic_is_copy(generic)
                })
        })
    }

    pub fn is_small_copy(&self, value: &Type) -> bool {
        self.is_copy(value) && self.sizes.is_small(value)
    }

    fn generic_is_copy(&self, generic: GenericId) -> bool {
        self.generics.get(&generic).is_some_and(|param| {
            param.bounds.iter().any(|bound| {
                self.copy_traits.contains(&bound.binding)
                    || self.bound_allows_only_copy(bound.binding)
            })
        })
    }

    pub fn bound_allows_only_copy_values(&self, bound: BindingId) -> bool {
        self.bound_allows_only_copy(bound)
            && self.traits.get(bound).is_some_and(|rust_trait| {
                rust_trait
                    .rows()
                    .implementors()
                    .iter()
                    .all(|implementor| !implementor.scheme.body.is_reference())
            })
    }

    fn bound_allows_only_copy(&self, bound: BindingId) -> bool {
        self.traits.get(bound).is_some_and(|rust_trait| {
            let implementors = rust_trait.rows().implementors();
            !implementors.is_empty()
                && implementors.iter().all(|implementor| {
                    implementor.scheme.params.is_empty() && self.is_copy(&implementor.scheme.body)
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::CopyRules;
    use crate::core::{BindingArena, BindingInfo, BindingKind, ModuleId, PrimitiveType};
    use crate::types::Type;
    use crate::types::rust_trait::{Implementor, LoweredRows, RustTrait};
    use crate::types::traits::TraitTable;
    use crate::types::ty::Scheme;

    struct CopyTraitFixture {
        declarations: crate::types::Declarations,
        generics: FxHashMap<crate::types::GenericId, crate::types::GenericParam>,
        bound: crate::core::BindingId,
    }

    impl CopyTraitFixture {
        fn new() -> Self {
            let mut bindings = BindingArena::default();
            let bound = bindings.add(BindingInfo::new(
                "copy".into(),
                BindingKind::Trait(ModuleId::new(0)),
                None,
            ));
            let mut traits = TraitTable::default();
            traits.add(
                bound,
                RustTrait::written(
                    "Copy".into(),
                    None,
                    LoweredRows::new(
                        vec![Implementor {
                            scheme: Scheme::monomorphic(Type::Primitive(PrimitiveType::I64)),
                            argument: None,
                        }],
                        Vec::new(),
                    ),
                ),
            );
            Self {
                declarations: crate::types::Declarations::with_traits(traits),
                generics: FxHashMap::default(),
                bound,
            }
        }

        fn rules(&self) -> CopyRules<'_> {
            CopyRules::new(&self.declarations, &self.generics)
        }
    }

    #[test]
    fn a_trait_whose_implementors_are_copy_allows_only_copy_values() {
        let fixture = CopyTraitFixture::new();
        assert!(fixture.rules().bound_allows_only_copy_values(fixture.bound));
    }
}
