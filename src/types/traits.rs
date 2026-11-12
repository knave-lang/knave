use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::projection::ProjectionTable;
use super::selection::{ImplementorMatch, Selection};
use super::ty::{Bound, GenericId, Scheme, Type};
use crate::core::BindingId;

pub struct Implementor {
    pub scheme: Scheme,
    pub argument: Option<Type>,
}

pub struct RustTrait {
    pub path: Rc<str>,
    pub implementors: Vec<Implementor>,
    pub help: Option<Rc<str>>,
    pub supertraits: Vec<Bound>,
    pub associated: Option<Rc<str>>,
}

impl RustTrait {
    pub fn is_open(&self) -> bool {
        self.implementors
            .iter()
            .any(|implementor| !implementor.scheme.params.is_empty())
    }

    pub fn links_argument_of(table: &TraitTable, binding: BindingId) -> bool {
        table
            .get(binding)
            .is_none_or(|rust_trait| rust_trait.links_its_argument(binding, table.projections()))
    }

    fn links_its_argument(&self, binding: BindingId, projections: &ProjectionTable) -> bool {
        projections.declared_by(binding).is_empty()
            && self
                .implementors
                .iter()
                .any(|implementor| implementor.argument.is_none())
    }
}

#[derive(Default)]
pub struct TraitTable {
    traits: FxHashMap<BindingId, RustTrait>,
    projections: Rc<ProjectionTable>,
    normalized_implementors: FxHashMap<BindingId, Vec<Implementor>>,
}

impl TraitTable {
    pub fn projections_mut(&mut self) -> &mut ProjectionTable {
        Rc::get_mut(&mut self.projections).expect("projections are shared only after collection")
    }

    pub fn share_projections(&mut self) -> Rc<ProjectionTable> {
        for (binding, rust_trait) in &self.traits {
            let normalized: Vec<Implementor> = rust_trait
                .implementors
                .iter()
                .filter_map(|implementor| Self::normalized(&self.projections, implementor))
                .collect();
            if !normalized.is_empty() {
                self.normalized_implementors.insert(*binding, normalized);
            }
        }
        Rc::clone(&self.projections)
    }

    fn normalized(projections: &ProjectionTable, implementor: &Implementor) -> Option<Implementor> {
        let body = projections.normalize(&implementor.scheme.body)?;
        Some(Implementor {
            scheme: Scheme {
                body,
                ..implementor.scheme.clone()
            },
            argument: implementor.argument.clone(),
        })
    }

    pub fn projections(&self) -> &ProjectionTable {
        &self.projections
    }

    pub fn add(&mut self, binding: BindingId, rust_trait: RustTrait) {
        self.traits.insert(binding, rust_trait);
    }

    pub fn select(&self, bound: &Bound, implementor: &Type) -> Selection {
        let normalized = self.projections.normalize_deep(implementor);
        self.traits.get(&bound.binding).map_or_else(
            || Selection::Missing,
            |rust_trait| {
                self.rows(bound.binding, rust_trait)
                    .map(|known| {
                        ImplementorMatch::select(
                            known,
                            bound,
                            &normalized,
                            rust_trait.associated.is_some(),
                        )
                    })
                    .fold(Selection::Missing, Selection::best)
            },
        )
    }

    pub fn settle_lone_row(&self, bound: &Bound, implementor: &Type) -> Option<Selection> {
        let rust_trait = self.traits.get(&bound.binding)?;
        let normalized = self.projections.normalize_deep(implementor);
        let mut open = self.rows(bound.binding, rust_trait).filter(|known| {
            !matches!(
                ImplementorMatch::select(known, bound, &normalized, false),
                Selection::Missing
            )
        });
        let only = open.next()?;
        if open.next().is_some() {
            return None;
        }
        ImplementorMatch::settle(only, bound, &normalized)
    }

    fn rows<'t>(
        &'t self,
        binding: BindingId,
        rust_trait: &'t RustTrait,
    ) -> impl Iterator<Item = &'t Implementor> {
        rust_trait
            .implementors
            .iter()
            .chain(self.normalized_implementors(binding))
    }

    fn normalized_implementors(&self, binding: BindingId) -> &[Implementor] {
        self.normalized_implementors
            .get(&binding)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn satisfies(
        &self,
        bound: &Bound,
        candidate: &Type,
        generic_satisfies: &dyn Fn(GenericId, &Bound) -> bool,
    ) -> bool {
        if let Type::Generic(generic) = candidate {
            return generic_satisfies(*generic, bound);
        }
        match self.select(bound, candidate) {
            Selection::Implemented { requirements, .. } => requirements.iter().all(|requirement| {
                self.satisfies(
                    &requirement.bound,
                    &requirement.implementor,
                    generic_satisfies,
                )
            }),
            Selection::Undecided | Selection::Missing => false,
        }
    }

    pub fn implies(&self, declared: &Bound, required: &Bound, implementor: GenericId) -> bool {
        self.brings(declared, &|brought| brought.meets(required, implementor))
    }

    pub fn brings(&self, declared: &Bound, wanted: &dyn Fn(&Bound) -> bool) -> bool {
        self.brings_unvisited(declared, wanted, &mut FxHashSet::default())
    }

    fn brings_unvisited(
        &self,
        declared: &Bound,
        wanted: &dyn Fn(&Bound) -> bool,
        visited: &mut FxHashSet<BindingId>,
    ) -> bool {
        if wanted(declared) {
            return true;
        }
        visited.insert(declared.binding)
            && self.get(declared.binding).is_some_and(|rust_trait| {
                rust_trait
                    .supertraits
                    .iter()
                    .any(|supertrait| self.brings_unvisited(supertrait, wanted, visited))
            })
    }

    pub fn implemented_exactly_by(&self, types: &[Type]) -> Option<BindingId> {
        let mut matching = self
            .traits
            .iter()
            .filter(|(_, rust_trait)| Self::implementors_are(rust_trait, types))
            .map(|(binding, _)| *binding);
        let found = matching.next()?;
        matching.next().is_none().then_some(found)
    }

    fn implementors_are(rust_trait: &RustTrait, types: &[Type]) -> bool {
        rust_trait.implementors.len() == types.len()
            && types.iter().all(|ty| {
                rust_trait
                    .implementors
                    .iter()
                    .any(|implementor| implementor.scheme.body == *ty)
            })
    }

    pub fn with_paths(&self, paths: &[&str]) -> Vec<BindingId> {
        let mut found: Vec<BindingId> = self
            .traits
            .iter()
            .filter(|(_, rust_trait)| paths.contains(&rust_trait.path.as_ref()))
            .map(|(binding, _)| *binding)
            .collect();
        found.sort();
        found
    }

    pub fn add_implementor(&mut self, binding: BindingId, implementor: Implementor) {
        if let Some(rust_trait) = self.traits.get_mut(&binding) {
            rust_trait.implementors.push(implementor);
        }
    }

    pub fn remove_struct_implementor(&mut self, binding: BindingId, struct_binding: BindingId) {
        if let Some(rust_trait) = self.traits.get_mut(&binding) {
            rust_trait.implementors.retain(|implementor| {
                !matches!(implementor.scheme.body, Type::Named(named, _) if named == struct_binding)
            });
        }
    }

    pub fn get(&self, binding: BindingId) -> Option<&RustTrait> {
        self.traits.get(&binding)
    }

    pub fn set_implementor_scheme(&mut self, binding: BindingId, body: &Type, scheme: Scheme) {
        if let Some(rust_trait) = self.traits.get_mut(&binding)
            && let Some(implementor) = rust_trait
                .implementors
                .iter_mut()
                .find(|implementor| implementor.scheme.body == *body)
        {
            implementor.scheme = scheme;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleId, PrimitiveType};
    use crate::types::selection::Selection;
    use crate::types::{
        Bound, GenericId, Implementor, RustTrait, Scheme, TraitTable, Type, TypeVar,
    };

    struct TraitsFixture {
        bindings: BindingArena,
        traits: TraitTable,
    }

    impl TraitsFixture {
        fn new() -> Self {
            Self {
                bindings: BindingArena::default(),
                traits: TraitTable::default(),
            }
        }

        fn add(&mut self, name: &str, implementors: &[PrimitiveType]) -> BindingId {
            let binding = self.bindings.add(BindingInfo::new(
                name.into(),
                BindingKind::Trait(ModuleId::new(0)),
                None,
            ));
            self.traits.add(
                binding,
                RustTrait {
                    path: std::rc::Rc::from(name),
                    implementors: implementors
                        .iter()
                        .map(|primitive| Implementor {
                            scheme: Scheme::monomorphic(Type::Primitive(*primitive)),
                            argument: None,
                        })
                        .collect(),
                    help: None,
                    supertraits: Vec::new(),
                    associated: None,
                },
            );
            binding
        }

        fn add_seconds(&mut self, name: &str, seconds: &[PrimitiveType]) -> BindingId {
            let binding = self.add(name, &[]);
            self.traits.add(
                binding,
                RustTrait {
                    path: std::rc::Rc::from(name),
                    implementors: seconds
                        .iter()
                        .map(|second| Implementor {
                            scheme: Scheme::monomorphic(Type::Primitive(PrimitiveType::I64)),
                            argument: Some(Type::Primitive(*second)),
                        })
                        .collect(),
                    help: None,
                    supertraits: Vec::new(),
                    associated: None,
                },
            );
            binding
        }

        fn settles_an_open_second(&self, binding: BindingId) -> Option<Type> {
            let bound = Bound::with_argument(binding, Type::Var(TypeVar::new(0)));
            match self
                .traits
                .settle_lone_row(&bound, &Type::Primitive(PrimitiveType::I64))
            {
                Some(Selection::Implemented { argument, .. }) => argument,
                _ => None,
            }
        }

        fn require(&mut self, binding: BindingId, required: BindingId) {
            self.traits.add(
                binding,
                RustTrait {
                    path: std::rc::Rc::from("Cycled"),
                    implementors: Vec::new(),
                    help: None,
                    supertraits: vec![Bound::plain(required)],
                    associated: None,
                },
            );
        }
    }

    #[test]
    fn a_lone_row_settles_an_open_second_type() {
        let mut fixture = TraitsFixture::new();
        let holds = fixture.add_seconds("Holds", &[PrimitiveType::U8]);
        assert_eq!(
            fixture.settles_an_open_second(holds),
            Some(Type::Primitive(PrimitiveType::U8))
        );
    }

    #[test]
    fn two_rows_leave_an_open_second_type_open() {
        let mut fixture = TraitsFixture::new();
        let holds = fixture.add_seconds("Holds", &[PrimitiveType::U8, PrimitiveType::I64]);
        assert_eq!(fixture.settles_an_open_second(holds), None);
    }

    #[test]
    fn a_walk_through_a_requirement_cycle_ends() {
        let mut fixture = TraitsFixture::new();
        let first = fixture.add("First", &[]);
        let second = fixture.add("Second", &[]);
        let unrelated = fixture.add("Third", &[]);
        fixture.require(first, second);
        fixture.require(second, first);
        let start = Bound::plain(first);
        assert!(
            fixture
                .traits
                .brings(&start, &|brought| brought.binding == second)
        );
        assert!(
            !fixture
                .traits
                .brings(&start, &|brought| brought.binding == unrelated)
        );
    }

    #[test]
    fn the_trait_implemented_by_exactly_the_types_is_found() {
        let mut fixture = TraitsFixture::new();
        let numbers = [PrimitiveType::I64, PrimitiveType::U8];
        fixture.add(
            "printable",
            &[PrimitiveType::I64, PrimitiveType::U8, PrimitiveType::Bool],
        );
        fixture.add("textual", &[PrimitiveType::Char, PrimitiveType::Bool]);
        let integer = fixture.add("integer", &numbers);
        let types = numbers.map(Type::Primitive);
        assert_eq!(fixture.traits.implemented_exactly_by(&types), Some(integer));
    }

    #[test]
    fn two_traits_implemented_by_exactly_the_types_find_none() {
        let mut fixture = TraitsFixture::new();
        let numbers = [PrimitiveType::I64, PrimitiveType::U8];
        fixture.add("integer", &numbers);
        fixture.add("countable", &numbers);
        let types = numbers.map(Type::Primitive);
        assert_eq!(fixture.traits.implemented_exactly_by(&types), None);
    }

    #[test]
    fn a_bound_with_a_second_type_implies_itself_and_not_another_second_type() {
        let mut fixture = TraitsFixture::new();
        let compared = fixture.add("PartialEq", &[]);
        let implementor = GenericId::new(0);
        let with_string = Bound::with_argument(compared, Type::Primitive(PrimitiveType::String));
        let with_str = Bound::with_argument(compared, Type::Primitive(PrimitiveType::Str));
        assert!(
            fixture
                .traits
                .implies(&with_string, &with_string, implementor)
        );
        assert!(!fixture.traits.implies(&with_string, &with_str, implementor));
    }
}
