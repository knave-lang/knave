use rustc_hash::{FxHashMap, FxHashSet};

use super::reached_traits::ReachedTraits;
use super::rust_types::RustType;
use crate::core::{
    BindingId, ConsultedTraits, CoreModule, ItemKind, MemberOwners, NamedTypes, References,
    RustTraitDeclaration,
};
use crate::modules::ModuleSources;

pub struct TraitReach<'p> {
    declarations: FxHashMap<BindingId, &'p RustTraitDeclaration>,
    owners: FxHashMap<BindingId, BindingId>,
    map_types: FxHashSet<BindingId>,
    reached: FxHashSet<BindingId>,
    waiting: Vec<BindingId>,
}

impl<'p> TraitReach<'p> {
    pub fn of<'r>(
        modules: &'p [CoreModule],
        definitions: impl Iterator<Item = &'r References>,
    ) -> ReachedTraits {
        let mut reach = Self::new(modules);
        reach.roots(modules);
        for references in definitions {
            if reach.reached.len() == reach.declarations.len() {
                break;
            }
            let named = references.written_types().named();
            for binding in references.globals().iter().chain(named) {
                reach.reach_noting_maps(*binding);
            }
        }
        reach.close();
        ReachedTraits::new(reach.reached)
    }

    fn new(modules: &'p [CoreModule]) -> Self {
        let declarations: FxHashMap<BindingId, &RustTraitDeclaration> = modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::RustTrait(declaration) => Some((declaration.binding, declaration)),
                _ => None,
            })
            .collect();
        let mut owners = FxHashMap::default();
        for item in modules.iter().flat_map(|module| &module.items) {
            MemberOwners::of(&item.kind).visit(&mut |member, owner| {
                owners.insert(member, owner);
            });
        }
        Self {
            declarations,
            owners,
            map_types: Self::map_types(modules),
            reached: FxHashSet::default(),
            waiting: Vec::new(),
        }
    }

    fn map_types(modules: &[CoreModule]) -> FxHashSet<BindingId> {
        modules
            .iter()
            .flat_map(|module| &module.items)
            .filter_map(|item| match &item.kind {
                ItemKind::RustType(declaration)
                    if RustType::spells_a_map(&declaration.template) =>
                {
                    Some(declaration.binding)
                }
                _ => None,
            })
            .collect()
    }

    fn reach_declared(&mut self, chosen: impl Fn(&RustTraitDeclaration) -> bool) {
        let declared: Vec<BindingId> = self
            .declarations
            .values()
            .filter(|declaration| chosen(declaration))
            .map(|declaration| declaration.binding)
            .collect();
        for binding in declared {
            self.reach(binding);
        }
    }

    fn roots(&mut self, modules: &[CoreModule]) {
        self.reach_declared(|declaration| {
            ConsultedTraits::BY_EVERY_PROGRAM.contains(&declaration.path.as_ref())
                || declaration.implementors.all_rows_primitive()
        });
        let written_items = modules
            .iter()
            .filter(|module| !ModuleSources::is_prelude(&module.path))
            .flat_map(|module| &module.items);
        let mut declares_a_type = false;
        for item in written_items {
            self.reach_written(&item.kind);
            declares_a_type |= Self::declares_type(&item.kind);
            self.reach_written_types(&item.kind);
        }
        if declares_a_type {
            self.reach_map_keys();
            self.reach_declared(|declaration| declaration.structural.is_some());
        }
    }

    fn reach_written(&mut self, kind: &ItemKind) {
        if let ItemKind::RustTrait(declaration) = kind {
            self.reach(declaration.binding);
        }
    }

    fn declares_type(kind: &ItemKind) -> bool {
        matches!(kind, ItemKind::Struct(..) | ItemKind::Enum(..))
    }

    fn reach_written_types(&mut self, kind: &ItemKind) {
        NamedTypes::of(kind).visit(&mut |binding| self.reach_noting_maps(binding));
    }

    fn reach_noting_maps(&mut self, binding: BindingId) {
        if self.map_types.contains(&binding) {
            self.reach_map_keys();
        }
        self.reach(binding);
    }

    fn reach_map_keys(&mut self) {
        self.map_types.clear();
        self.reach_declared(|declaration| {
            ConsultedTraits::BY_MAPS_AND_DERIVES.contains(&declaration.path.as_ref())
        });
    }

    fn close(&mut self) {
        while let Some(binding) = self.waiting.pop() {
            let Some(declaration) = self.declarations.get(&binding).copied() else {
                continue;
            };
            NamedTypes::of_declaration(declaration).visit(&mut |named| self.reach(named));
        }
    }

    fn reach(&mut self, binding: BindingId) {
        let owner = self.owners.get(&binding).copied().unwrap_or(binding);
        if self.declarations.contains_key(&owner) && self.reached.insert(owner) {
            self.waiting.push(owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TraitReach;
    use crate::types::reached_traits::ReachedTraits;
    use std::rc::Rc;

    use crate::core::{
        AssociatedValue, BindingArena, BindingId, BindingInfo, BindingKind, CoreModule, Expr,
        ExprKind, FieldLabel, FieldSpelling, ImplAssociatedType, ImplementorRows, Item, ItemKind,
        Leaf, ModuleId, ModulePath, ParsedTemplate, PrimitiveType, PrintFormat, References,
        RustTraitAssociatedType, RustTraitDeclaration, RustTraitImplementor, RustTypeDeclaration,
        TraitMethod, TraitRelations, TypeBinder, TypeBound, TypeExpr, TypeExprKind,
        VariantDeclaration, VariantPayload,
    };
    use crate::source::{SourceId, Span};
    use crate::types::TypedSource;
    use crate::types::graph::DefinitionGraph;

    struct ReachFixture {
        source: TypedSource,
        reached: ReachedTraits,
    }

    impl ReachFixture {
        const EMPTY_PROGRAM: &str = "(defn main ())";
        const MAP_KEY_TRAITS: [&str; 4] = ["ord", "eq", "hash", "borrow"];

        fn of(program: &str) -> Self {
            Self::reaching(TypedSource::expand(program))
        }

        fn loaded(program: &str) -> Self {
            Self::reaching(TypedSource::expand_on_a_loaded_prelude(program))
        }

        fn reaching(source: TypedSource) -> Self {
            let reached = DefinitionGraph::of(source.program())
                .reach()
                .traits()
                .clone();
            Self { source, reached }
        }

        fn declared_traits(&self) -> impl Iterator<Item = &RustTraitDeclaration> {
            self.source
                .program()
                .modules
                .iter()
                .flat_map(|module| &module.items)
                .filter_map(|item| match &item.kind {
                    ItemKind::RustTrait(declaration) => Some(declaration),
                    _ => None,
                })
        }

        fn reaches(&self, trait_name: &str) -> bool {
            let bindings = &self.source.program().bindings;
            let declaration = self
                .declared_traits()
                .find(|declaration| bindings.get(declaration.binding).name() == trait_name)
                .expect("the program declares the trait");
            self.reached.reaches(declaration.binding)
        }
    }

    #[test]
    fn an_empty_program_reaches_the_kernel_traits_only() {
        let fixture = ReachFixture::of(ReachFixture::EMPTY_PROGRAM);
        assert!(fixture.reaches("copy"));
        assert!(fixture.reaches("number"));
        assert!(!fixture.reaches("display"));
        assert!(!fixture.reaches("from-str"));
        assert!(!fixture.reaches("sequence"));
    }

    #[test]
    fn an_empty_program_reaches_no_map_key_trait() {
        let fixture = ReachFixture::of(ReachFixture::EMPTY_PROGRAM);
        for map_key_trait in ReachFixture::MAP_KEY_TRAITS {
            assert!(!fixture.reaches(map_key_trait), "{map_key_trait}");
        }
    }

    #[test]
    fn a_reached_trait_reaches_the_traits_it_requires() {
        let fixture = ReachFixture::of(ReachFixture::EMPTY_PROGRAM);
        assert!(fixture.reaches("clone"));
    }

    #[test]
    fn a_declared_type_reaches_the_map_key_traits_its_derives_consult() {
        let fixture = ReachFixture::of("(defstruct point :x i64) (defn main ())");
        for map_key_trait in ReachFixture::MAP_KEY_TRAITS {
            assert!(fixture.reaches(map_key_trait), "{map_key_trait}");
        }
        assert!(fixture.reaches("partial-ord"));
    }

    #[test]
    fn a_written_map_type_reaches_the_map_key_traits() {
        let fixture = ReachFixture::of(
            "(defn total ((counts (sorted-map string i64))) counts)
             (defn main ())",
        );
        for map_key_trait in ReachFixture::MAP_KEY_TRAITS {
            assert!(fixture.reaches(map_key_trait), "{map_key_trait}");
        }
    }

    #[test]
    fn a_map_that_a_reached_definition_names_reaches_the_map_key_traits() {
        let fixture = ReachFixture::of("(defn main () (into-hash-map (vec)))");
        assert!(fixture.reaches("borrow"));
        assert!(fixture.reaches("ord"));
    }

    #[test]
    fn a_program_that_names_no_map_reaches_no_map_key_trait() {
        let fixture = ReachFixture::of("(defn main () (len (vec 1)))");
        assert!(!fixture.reaches("borrow"));
    }

    #[test]
    fn a_reached_definition_reaches_the_traits_its_type_names() {
        let fixture = ReachFixture::of("(defn main () (len (vec 1)))");
        assert!(fixture.reaches("sequence"));
        assert!(!fixture.reaches("from-str"));
    }

    #[test]
    fn an_associated_type_reaches_the_trait_that_declares_it() {
        let fixture = ReachFixture::of(
            "(defn reason ((failure (parse-error i64))) failure)
             (defn main ())",
        );
        assert!(fixture.reaches("from-str"));
    }

    #[test]
    fn a_declared_type_reaches_the_structural_traits() {
        let fixture = ReachFixture::of("(defstruct point :x i64) (defn main ())");
        assert!(fixture.reaches("show"));
        assert!(!ReachFixture::of(ReachFixture::EMPTY_PROGRAM).reaches("show"));
    }

    struct DeclaredTraits {
        bindings: BindingArena,
        prelude: Vec<Item>,
        written: Vec<Item>,
    }

    impl DeclaredTraits {
        fn new() -> Self {
            Self {
                bindings: BindingArena::default(),
                prelude: Vec::new(),
                written: Vec::new(),
            }
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn binding(&mut self, name: &str) -> BindingId {
            let kind = BindingKind::Global(ModuleId::new(0));
            self.bindings.add(BindingInfo::new(name.into(), kind, None))
        }

        fn named(binding: BindingId) -> TypeExpr {
            TypeExpr::new(TypeExprKind::Named(binding, Vec::new()), Self::span())
        }

        fn item(kind: ItemKind) -> Item {
            Item {
                kind,
                span: Self::span(),
                hidden: false,
            }
        }

        fn declare(&mut self, name: &str) -> RustTraitDeclaration {
            RustTraitDeclaration {
                binding: self.binding(name),
                path: name.into(),
                resolved_path: name.into(),
                self_variable: self.binding("self"),
                methods: Vec::new(),
                help: None,
                relations: TraitRelations::default(),
                structural: None,
                associated: Vec::new(),
                implementors: ImplementorRows::new(Vec::new()),
            }
        }

        fn add(&mut self, declaration: RustTraitDeclaration) -> BindingId {
            let binding = declaration.binding;
            self.prelude
                .push(Self::item(ItemKind::RustTrait(declaration)));
            binding
        }

        fn method_bounded_by(&mut self, bound: BindingId) -> TraitMethod {
            let variable = self.binding("value");
            let binder = TypeBinder {
                variable,
                bounds: vec![TypeBound {
                    binding: bound,
                    argument: None,
                    field: None,
                }],
                span: Self::span(),
                lifetime: false,
            };
            TraitMethod {
                binding: self.binding("method"),
                signature: TypeExpr::new(
                    TypeExprKind::Where(vec![binder], Box::new(Self::named(variable))),
                    Self::span(),
                ),
                default: None,
            }
        }

        fn core_module(index: usize, path: &str, items: &[Item]) -> CoreModule {
            let mut module = CoreModule::new(ModuleId::new(index), ModulePath::new(path), None);
            module.items = items.to_vec();
            module
        }

        fn reached_from(&self, names: &[BindingId]) -> ReachedTraits {
            let modules = [
                Self::core_module(0, "prelude/traits", &self.prelude),
                Self::core_module(1, "main", &self.written),
            ];
            let calls: Vec<Expr> = names
                .iter()
                .map(|name| Expr::new(ExprKind::Leaf(Leaf::Var(*name)), Self::span()))
                .collect();
            let references: Vec<References> = calls
                .iter()
                .map(|call| References::of(call, &self.bindings))
                .collect();
            TraitReach::of(&modules, references.iter())
        }
    }

    #[test]
    fn a_called_method_reaches_its_trait_and_the_traits_its_type_names() {
        let mut traits = DeclaredTraits::new();
        let bound = traits.declare("bound");
        let bound = traits.add(bound);
        let mut owner = traits.declare("owner");
        let method = traits.method_bounded_by(bound);
        let called = method.binding;
        owner.methods.push(method);
        let owner = traits.add(owner);
        let reached = traits.reached_from(&[called]);
        assert!(reached.reaches(owner));
        assert!(reached.reaches(bound));
    }

    #[test]
    fn a_row_reaches_the_trait_of_an_associated_type_it_gives() {
        let mut traits = DeclaredTraits::new();
        let mut declaring = traits.declare("declaring");
        let projection = traits.binding("output");
        declaring.associated.push(RustTraitAssociatedType {
            binding: projection,
            parameter: "item".into(),
            rust_name: "Output".into(),
        });
        let declaring = traits.add(declaring);
        let mut giving = traits.declare("giving");
        let given = traits.binding("given");
        giving.implementors = ImplementorRows::new(vec![RustTraitImplementor {
            implementor: DeclaredTraits::named(given),
            associated: vec![AssociatedValue {
                binding: given,
                value: DeclaredTraits::named(projection),
                span: DeclaredTraits::span(),
            }],
        }]);
        let giving = traits.add(giving);
        let reached = traits.reached_from(&[giving]);
        assert!(reached.reaches(declaring));
    }

    #[test]
    fn a_written_impl_reaches_the_prelude_trait_it_implements() {
        let mut traits = DeclaredTraits::new();
        let implemented = traits.declare("implemented");
        let implemented = traits.add(implemented);
        let point = traits.binding("point");
        traits.written.push(DeclaredTraits::item(ItemKind::Impl(
            implemented,
            DeclaredTraits::named(point),
            Vec::new(),
            Vec::new(),
        )));
        assert!(traits.reached_from(&[]).reaches(implemented));
        traits.written.clear();
        assert!(!traits.reached_from(&[]).reaches(implemented));
    }

    impl DeclaredTraits {
        fn label() -> FieldLabel {
            FieldLabel {
                name: "field".into(),
                span: Self::span(),
                spelling: FieldSpelling::Bare,
            }
        }

        fn template() -> Rc<ParsedTemplate> {
            Rc::new(ParsedTemplate::parse("Kept").expect("the template parses"))
        }

        fn reaches_through(
            &mut self,
            written: impl FnOnce(&mut Self, BindingId) -> ItemKind,
        ) -> bool {
            let named = self.declare("named");
            let named = self.add(named);
            let kind = written(self, named);
            self.written.push(Self::item(kind));
            self.reached_from(&[]).reaches(named)
        }
    }

    #[test]
    fn a_signature_reaches_the_traits_its_type_names() {
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            ItemKind::Signature(traits.binding("value"), DeclaredTraits::named(named))
        }));
    }

    #[test]
    fn a_struct_reaches_the_traits_of_its_fields() {
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            let field = (DeclaredTraits::label(), DeclaredTraits::named(named));
            ItemKind::Struct(traits.binding("point"), Vec::new(), vec![field])
        }));
    }

    #[test]
    fn an_enum_reaches_the_traits_of_its_payloads() {
        let positional = |named| VariantPayload::Positional(vec![DeclaredTraits::named(named)]);
        let labeled = |named| {
            VariantPayload::Named(vec![(
                DeclaredTraits::label(),
                DeclaredTraits::named(named),
            )])
        };
        for payload in [positional as fn(BindingId) -> VariantPayload, labeled] {
            assert!(DeclaredTraits::new().reaches_through(|traits, named| {
                let variant = VariantDeclaration {
                    binding: traits.binding("circle"),
                    name: "circle".into(),
                    payload: payload(named),
                    span: DeclaredTraits::span(),
                };
                ItemKind::Enum(
                    traits.binding("shape"),
                    Vec::new(),
                    vec![variant],
                    None,
                    None,
                )
            }));
        }
    }

    #[test]
    fn a_derive_reaches_the_traits_it_derives() {
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            ItemKind::Derive(
                traits.binding("point"),
                DeclaredTraits::span(),
                vec![(named, DeclaredTraits::span())],
            )
        }));
    }

    #[test]
    fn a_written_trait_reaches_its_supertraits_and_method_types() {
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            let supertrait = TypeBound {
                binding: named,
                argument: None,
                field: None,
            };
            let self_variable = traits.binding("self");
            ItemKind::Trait(
                traits.binding("shown"),
                "Shown".into(),
                self_variable,
                vec![supertrait],
                Vec::new(),
                Vec::new(),
            )
        }));
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            let method = TraitMethod {
                binding: traits.binding("show"),
                signature: DeclaredTraits::named(named),
                default: None,
            };
            let self_variable = traits.binding("self");
            ItemKind::Trait(
                traits.binding("shown"),
                "Shown".into(),
                self_variable,
                Vec::new(),
                Vec::new(),
                vec![method],
            )
        }));
    }

    #[test]
    fn an_impl_reaches_the_traits_of_its_type_and_associated_values() {
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            ItemKind::Impl(
                traits.binding("other"),
                DeclaredTraits::named(named),
                Vec::new(),
                Vec::new(),
            )
        }));
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            let value = ImplAssociatedType {
                trait_type: traits.binding("output"),
                name: "output".into(),
                value: DeclaredTraits::named(named),
            };
            let point = DeclaredTraits::named(traits.binding("point"));
            ItemKind::Impl(traits.binding("other"), point, vec![value], Vec::new())
        }));
    }

    #[test]
    fn a_rust_type_reaches_the_traits_of_its_clauses() {
        assert!(DeclaredTraits::new().reaches_through(|traits, named| {
            let clause = (DeclaredTraits::named(named), DeclaredTraits::template());
            ItemKind::RustType(RustTypeDeclaration {
                binding: traits.binding("kept"),
                binders: Vec::new(),
                unsized_parameters: Vec::new(),
                template: DeclaredTraits::template(),
                clauses: vec![clause],
                format: PrintFormat::Display,
            })
        }));
    }

    #[test]
    fn a_trait_whose_rows_are_all_primitives_is_reached_unnamed() {
        let mut traits = DeclaredTraits::new();
        let mut numeric = traits.declare("numeric");
        numeric.implementors = ImplementorRows::new(vec![RustTraitImplementor {
            implementor: TypeExpr::new(
                TypeExprKind::Primitive(PrimitiveType::I64),
                DeclaredTraits::span(),
            ),
            associated: Vec::new(),
        }]);
        let numeric = traits.add(numeric);
        let unrelated = traits.declare("unrelated");
        let unrelated = traits.add(unrelated);
        let reached = traits.reached_from(&[]);
        assert!(reached.reaches(numeric));
        assert!(!reached.reaches(unrelated));
    }

    #[test]
    fn every_prelude_trait_saves_whether_its_rows_are_all_primitives() {
        let fixture = ReachFixture::loaded(ReachFixture::EMPTY_PROGRAM);
        for declaration in fixture.declared_traits() {
            let rows = &declaration.implementors;
            let rebuilt = ImplementorRows::new(rows.get().to_vec());
            assert_eq!(
                rows.all_rows_primitive(),
                rebuilt.all_rows_primitive(),
                "{}",
                declaration.path
            );
        }
    }
}
