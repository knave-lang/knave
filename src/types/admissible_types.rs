use rustc_hash::FxHashMap;

use super::admissible::Admissible;
use super::traits::{RustTrait, TraitTable};
use super::ty::{GenericId, GenericParam, Type};

pub(super) struct AdmissibleTypes<'a> {
    generics: &'a FxHashMap<GenericId, GenericParam>,
    traits: &'a TraitTable,
}

impl<'a> AdmissibleTypes<'a> {
    pub(super) fn new(
        generics: &'a FxHashMap<GenericId, GenericParam>,
        traits: &'a TraitTable,
    ) -> Self {
        Self { generics, traits }
    }

    pub(super) fn of(&self, generic: GenericId) -> Admissible {
        let traits = self.bounding_traits(generic);
        let Some((first, others)) = traits.split_first() else {
            return Admissible::Unbounded;
        };
        let mut admissible: Vec<Type> = first
            .implementors
            .iter()
            .map(|implementor| implementor.scheme.body.clone())
            .collect();
        for rust_trait in others {
            admissible.retain(|candidate| {
                rust_trait
                    .implementors
                    .iter()
                    .any(|implementor| implementor.scheme.body == *candidate)
            });
        }
        if traits.iter().any(|rust_trait| rust_trait.is_open()) {
            Admissible::Open(admissible)
        } else {
            Admissible::Closed(admissible)
        }
    }

    fn bounding_traits(&self, generic: GenericId) -> Vec<&'a RustTrait> {
        self.generics
            .get(&generic)
            .map(|param| {
                param
                    .bounds
                    .iter()
                    .filter_map(|bound| self.traits.get(bound.binding))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::AdmissibleTypes;
    use crate::core::{BindingId, PrimitiveType};
    use crate::types::admissible::Admissible;
    use crate::types::{
        Bound, GenericId, GenericParam, Implementor, RustTrait, Scheme, TraitTable, Type,
    };

    struct AdmissibleFixture {
        generics: FxHashMap<GenericId, GenericParam>,
        traits: TraitTable,
    }

    impl AdmissibleFixture {
        const FIRST_TRAIT: BindingId = BindingId::from_index(0);
        const SECOND_TRAIT: BindingId = BindingId::from_index(1);

        fn new() -> Self {
            Self {
                generics: FxHashMap::default(),
                traits: TraitTable::default(),
            }
        }

        fn bounded_by(mut self, traits: &[BindingId]) -> Self {
            let generic = Self::generic();
            let bounds = traits.iter().copied().map(Bound::plain).collect();
            self.generics.insert(
                generic,
                GenericParam {
                    id: generic,
                    name: "T".into(),
                    bounds,
                },
            );
            self
        }

        fn implemented_by(mut self, rust_trait: BindingId, implementors: Vec<Implementor>) -> Self {
            self.traits.add(
                rust_trait,
                RustTrait {
                    path: "Shown".into(),
                    implementors,
                    help: None,
                    supertraits: Vec::new(),
                    associated: None,
                },
            );
            self
        }

        fn admissible(&self) -> Admissible {
            AdmissibleTypes::new(&self.generics, &self.traits).of(Self::generic())
        }

        fn generic() -> GenericId {
            GenericId::new(0)
        }

        fn concrete(body: PrimitiveType) -> Implementor {
            Self::implementor(Vec::new(), Type::Primitive(body))
        }

        fn generic_implementor() -> Implementor {
            let parameter = GenericId::new(1);
            let param = GenericParam {
                id: parameter,
                name: "U".into(),
                bounds: Vec::new(),
            };
            Self::implementor(vec![param], Type::Generic(parameter))
        }

        fn implementor(params: Vec<GenericParam>, body: Type) -> Implementor {
            Implementor {
                scheme: Scheme {
                    lifetimes: Vec::new(),
                    params,
                    body,
                },
                argument: None,
            }
        }
    }

    #[test]
    fn a_generic_without_a_trait_bound_admits_every_type() {
        let fixture = AdmissibleFixture::new().bounded_by(&[]);
        assert_eq!(fixture.admissible(), Admissible::Unbounded);
    }

    #[test]
    fn one_bound_admits_its_implementors() {
        let fixture = AdmissibleFixture::new()
            .bounded_by(&[AdmissibleFixture::FIRST_TRAIT])
            .implemented_by(
                AdmissibleFixture::FIRST_TRAIT,
                vec![AdmissibleFixture::concrete(PrimitiveType::I64)],
            );
        assert_eq!(
            fixture.admissible(),
            Admissible::Closed(vec![Type::Primitive(PrimitiveType::I64)])
        );
    }

    #[test]
    fn two_bounds_admit_only_the_types_that_implement_both() {
        let fixture = AdmissibleFixture::new()
            .bounded_by(&[
                AdmissibleFixture::FIRST_TRAIT,
                AdmissibleFixture::SECOND_TRAIT,
            ])
            .implemented_by(
                AdmissibleFixture::FIRST_TRAIT,
                vec![
                    AdmissibleFixture::concrete(PrimitiveType::I64),
                    AdmissibleFixture::concrete(PrimitiveType::F64),
                ],
            )
            .implemented_by(
                AdmissibleFixture::SECOND_TRAIT,
                vec![
                    AdmissibleFixture::concrete(PrimitiveType::U8),
                    AdmissibleFixture::concrete(PrimitiveType::I64),
                ],
            );
        assert_eq!(
            fixture.admissible(),
            Admissible::Closed(vec![Type::Primitive(PrimitiveType::I64)])
        );
    }

    #[test]
    fn one_bound_with_a_generic_implementor_leaves_the_types_open() {
        let fixture = AdmissibleFixture::new()
            .bounded_by(&[
                AdmissibleFixture::FIRST_TRAIT,
                AdmissibleFixture::SECOND_TRAIT,
            ])
            .implemented_by(
                AdmissibleFixture::FIRST_TRAIT,
                vec![AdmissibleFixture::concrete(PrimitiveType::I64)],
            )
            .implemented_by(
                AdmissibleFixture::SECOND_TRAIT,
                vec![
                    AdmissibleFixture::concrete(PrimitiveType::I64),
                    AdmissibleFixture::generic_implementor(),
                ],
            );
        assert_eq!(
            fixture.admissible(),
            Admissible::Open(vec![Type::Primitive(PrimitiveType::I64)])
        );
    }
}
