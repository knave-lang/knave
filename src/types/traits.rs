use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::projection::ProjectionTable;
use super::rust_trait::{Implementor, LoweredRows, RustTrait};
use super::selection::{ImplementorMatch, Selection};
use super::ty::{Bound, GenericId, Scheme, Type};
use crate::core::BindingId;

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
            let Some(rows) = rust_trait.lowered_rows() else {
                continue;
            };
            let normalized: Vec<Implementor> = rows
                .implementors()
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

    pub fn rows(&self, binding: BindingId) -> TraitRows<'_> {
        TraitRows::new(
            self.traits.get(&binding),
            self.normalized_implementors(binding),
            &self.projections,
        )
    }

    pub fn links_argument(&self, binding: BindingId) -> bool {
        self.get(binding).is_none_or(|rust_trait| {
            let rows = rust_trait.rows().implementors();
            self.projections.declared_by(binding).is_empty()
                && rows.iter().any(|row| row.argument.is_none())
        })
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
        match self.rows(bound.binding).select(bound, candidate) {
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
                    .rows()
                    .supertraits()
                    .iter()
                    .any(|supertrait| self.brings_unvisited(supertrait, wanted, visited))
            })
    }

    pub fn implemented_exactly_by(&self, types: &[Type]) -> Option<BindingId> {
        let mut matching = self
            .traits
            .iter()
            .filter_map(|(binding, rust_trait)| Some((binding, rust_trait.lowered_rows()?)))
            .filter(|(_, rows)| Self::implementors_are(rows, types))
            .map(|(binding, _)| *binding);
        let found = matching.next()?;
        matching.next().is_none().then_some(found)
    }

    fn implementors_are(rows: &LoweredRows, types: &[Type]) -> bool {
        rows.implementors().len() == types.len()
            && types.iter().all(|ty| {
                rows.implementors()
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
            rust_trait.rows_mut().add(implementor);
        }
    }

    pub fn remove_struct_implementor(&mut self, binding: BindingId, struct_binding: BindingId) {
        if let Some(rust_trait) = self.traits.get_mut(&binding) {
            rust_trait.rows_mut().remove_struct(struct_binding);
        }
    }

    pub fn get(&self, binding: BindingId) -> Option<&RustTrait> {
        self.traits.get(&binding)
    }

    pub fn set_implementor_scheme(&mut self, binding: BindingId, body: &Type, scheme: Scheme) {
        if let Some(rust_trait) = self.traits.get_mut(&binding) {
            rust_trait.rows_mut().set_scheme(body, scheme);
        }
    }
}

pub struct TraitRows<'t> {
    rust_trait: Option<&'t RustTrait>,
    normalized: &'t [Implementor],
    projections: &'t ProjectionTable,
}

impl<'t> TraitRows<'t> {
    pub fn new(
        rust_trait: Option<&'t RustTrait>,
        normalized: &'t [Implementor],
        projections: &'t ProjectionTable,
    ) -> Self {
        Self {
            rust_trait,
            normalized,
            projections,
        }
    }

    pub fn select(&self, bound: &Bound, target: &Type) -> Selection {
        let Some(rust_trait) = self.rust_trait else {
            return Selection::Missing;
        };
        let normalized = self.projections.normalize_deep(target);
        let associated = rust_trait.associated.is_some();
        self.rows(rust_trait)
            .map(|row| ImplementorMatch::select(row, bound, &normalized, associated))
            .fold(Selection::Missing, Selection::best)
    }

    pub fn settle_lone_row(&self, bound: &Bound, target: &Type) -> Option<Selection> {
        let normalized = self.projections.normalize_deep(target);
        let mut open = self.rows(self.rust_trait?).filter(|row| {
            !matches!(
                ImplementorMatch::select(row, bound, &normalized, false),
                Selection::Missing
            )
        });
        let only = open.next()?;
        if open.next().is_some() {
            return None;
        }
        ImplementorMatch::settle(only, bound, &normalized)
    }

    fn rows(&self, rust_trait: &'t RustTrait) -> impl Iterator<Item = &'t Implementor> + use<'t> {
        rust_trait
            .rows()
            .implementors()
            .iter()
            .chain(self.normalized)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleId, PrimitiveType};
    use crate::types::selection::Selection;
    use crate::types::{
        Bound, GenericId, Implementor, LoweredRows, RustTrait, Scheme, TraitTable, Type, TypeVar,
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
                RustTrait::written(
                    std::rc::Rc::from(name),
                    None,
                    LoweredRows::new(
                        implementors
                            .iter()
                            .map(|primitive| Implementor {
                                scheme: Scheme::monomorphic(Type::Primitive(*primitive)),
                                argument: None,
                            })
                            .collect(),
                        Vec::new(),
                    ),
                ),
            );
            binding
        }

        fn add_seconds(&mut self, name: &str, seconds: &[PrimitiveType]) -> BindingId {
            let binding = self.add(name, &[]);
            self.traits.add(
                binding,
                RustTrait::written(
                    std::rc::Rc::from(name),
                    None,
                    LoweredRows::new(
                        seconds
                            .iter()
                            .map(|second| Implementor {
                                scheme: Scheme::monomorphic(Type::Primitive(PrimitiveType::I64)),
                                argument: Some(Type::Primitive(*second)),
                            })
                            .collect(),
                        Vec::new(),
                    ),
                ),
            );
            binding
        }

        fn settles_an_open_second(&self, binding: BindingId) -> Option<Type> {
            let bound = Bound::with_argument(binding, Type::Var(TypeVar::new(0)));
            match self
                .traits
                .rows(binding)
                .settle_lone_row(&bound, &Type::Primitive(PrimitiveType::I64))
            {
                Some(Selection::Implemented { argument, .. }) => argument,
                _ => None,
            }
        }

        fn require(&mut self, binding: BindingId, required: BindingId) {
            self.traits.add(
                binding,
                RustTrait::written(
                    std::rc::Rc::from("Cycled"),
                    None,
                    LoweredRows::new(Vec::new(), vec![Bound::plain(required)]),
                ),
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
    fn rows_select_the_row_of_a_closed_second_type() {
        let mut fixture = TraitsFixture::new();
        let holds = fixture.add_seconds("Holds", &[PrimitiveType::U8, PrimitiveType::I64]);
        let implementor = Type::Primitive(PrimitiveType::I64);
        let wanted = Bound::with_argument(holds, Type::Primitive(PrimitiveType::U8));
        let unwanted = Bound::with_argument(holds, Type::Primitive(PrimitiveType::Bool));
        let rows = fixture.traits.rows(holds);
        assert!(matches!(
            rows.select(&wanted, &implementor),
            Selection::Implemented { .. }
        ));
        assert!(matches!(
            rows.select(&unwanted, &implementor),
            Selection::Missing
        ));
    }

    #[test]
    fn a_trait_with_a_row_of_no_second_type_links_its_argument() {
        let mut fixture = TraitsFixture::new();
        let comparable = fixture.add("Comparable", &[PrimitiveType::I64]);
        let holds = fixture.add_seconds("Holds", &[PrimitiveType::U8]);
        assert!(fixture.traits.links_argument(comparable));
        assert!(!fixture.traits.links_argument(holds));
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
