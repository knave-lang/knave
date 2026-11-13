use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{
    BindingId, ConsultedTraits, CoreModule, ItemKind, References, RustTraitDeclaration, TypeBound,
    TypeExpr, TypeExprKind, VariantPayload,
};
use crate::modules::ModuleSources;

#[derive(Clone)]
pub struct ReachedTraits {
    traits: FxHashSet<BindingId>,
}

impl ReachedTraits {
    pub fn reaches(&self, trait_binding: BindingId) -> bool {
        self.traits.contains(&trait_binding)
    }
}

pub struct TraitReach<'p> {
    declarations: FxHashMap<BindingId, &'p RustTraitDeclaration>,
    owners: FxHashMap<BindingId, BindingId>,
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
                reach.reach(*binding);
            }
        }
        reach.close();
        ReachedTraits {
            traits: reach.reached,
        }
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
        let owners = declarations
            .values()
            .flat_map(|declaration| {
                let methods = declaration.methods.iter().map(|method| method.binding);
                let associated = declaration.associated.iter().map(|value| value.binding);
                methods
                    .chain(associated)
                    .map(|owned| (owned, declaration.binding))
            })
            .collect();
        Self {
            declarations,
            owners,
            reached: FxHashSet::default(),
            waiting: Vec::new(),
        }
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
            ConsultedTraits::PATHS.contains(&declaration.path.as_ref())
                || Self::lists_only_primitives(declaration)
        });
        let mut writes_a_type = false;
        for module in modules {
            let written = !ModuleSources::is_prelude(&module.path);
            for item in &module.items {
                if written {
                    self.reach_written(&item.kind);
                    writes_a_type |= Self::writes_type(&item.kind);
                }
                self.reach_item(&item.kind);
            }
        }
        if writes_a_type {
            self.reach_declared(|declaration| declaration.structural.is_some());
        }
    }

    fn lists_only_primitives(declaration: &RustTraitDeclaration) -> bool {
        let rows = &declaration.implementors;
        !rows.is_empty()
            && rows
                .iter()
                .all(|row| matches!(row.implementor.kind(), TypeExprKind::Primitive(_)))
    }

    fn reach_written(&mut self, kind: &ItemKind) {
        if let ItemKind::RustTrait(declaration) = kind {
            self.reach(declaration.binding);
        }
    }

    fn writes_type(kind: &ItemKind) -> bool {
        matches!(kind, ItemKind::Struct(..) | ItemKind::Enum(..))
    }

    fn reach_item(&mut self, kind: &ItemKind) {
        match kind {
            ItemKind::Signature(_, signature) => self.type_expr(signature),
            ItemKind::Struct(_, _, fields) => {
                self.types(fields.iter().map(|(_, field)| field));
            }
            ItemKind::Enum(_, _, variants, ..) => {
                self.types(
                    variants
                        .iter()
                        .flat_map(|variant| Self::payload(&variant.payload)),
                );
            }
            ItemKind::Derive(_, _, derived) => {
                self.reach_all(derived.iter().map(|(binding, _)| *binding));
            }
            _ => self.reach_declaration(kind),
        }
    }

    fn reach_declaration(&mut self, kind: &ItemKind) {
        match kind {
            ItemKind::Trait(_, _, _, supertraits, _, methods) => {
                self.bounds(supertraits.iter());
                self.types(methods.iter().map(|method| &method.signature));
            }
            ItemKind::Impl(trait_binding, implementor, associated, _) => {
                self.reach(*trait_binding);
                self.type_expr(implementor);
                self.types(associated.iter().map(|value| &value.value));
            }
            ItemKind::RustType(_, _, _, _, clauses) => {
                self.types(clauses.iter().map(|(target, _)| target));
            }
            _ => {}
        }
    }

    fn payload(payload: &VariantPayload) -> Vec<&TypeExpr> {
        match payload {
            VariantPayload::None => Vec::new(),
            VariantPayload::Positional(fields) => fields.iter().collect(),
            VariantPayload::Named(fields) => fields.iter().map(|(_, field)| field).collect(),
        }
    }

    fn close(&mut self) {
        while let Some(binding) = self.waiting.pop() {
            let Some(declaration) = self.declarations.get(&binding).copied() else {
                continue;
            };
            self.bounds(declaration.requires.iter().map(|required| &required.bound));
            self.types(declaration.methods.iter().map(|method| &method.signature));
            let rows = declaration.implementors.iter();
            self.types(rows.flat_map(|row| {
                std::iter::once(&row.implementor)
                    .chain(row.associated.iter().map(|value| &value.value))
            }));
        }
    }

    fn reach_all(&mut self, bindings: impl Iterator<Item = BindingId>) {
        for binding in bindings {
            self.reach(binding);
        }
    }

    fn reach(&mut self, binding: BindingId) {
        let owner = self.owners.get(&binding).copied().unwrap_or(binding);
        if self.declarations.contains_key(&owner) && self.reached.insert(owner) {
            self.waiting.push(owner);
        }
    }

    fn type_expr(&mut self, type_expr: &TypeExpr) {
        self.reach_all(type_expr.named_bindings().into_iter());
    }

    fn types<'t>(&mut self, written: impl Iterator<Item = &'t TypeExpr>) {
        for type_expr in written {
            self.type_expr(type_expr);
        }
    }

    fn bounds<'b>(&mut self, bounds: impl Iterator<Item = &'b TypeBound>) {
        for bound in bounds {
            self.reach_all(bound.named_bindings().into_iter());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReachedTraits, TraitReach};
    use std::rc::Rc;

    use crate::core::{
        AssociatedValue, BindingArena, BindingId, BindingInfo, BindingKind, CoreModule, Expr,
        ExprKind, FieldLabel, FieldSpelling, ImplAssociatedType, Item, ItemKind, Leaf, ModuleId,
        ModulePath, ParsedTemplate, PrimitiveType, References, RustTraitAssociatedType,
        RustTraitDeclaration, RustTraitImplementor, TraitMethod, TypeBinder, TypeBound, TypeExpr,
        TypeExprKind, VariantDeclaration, VariantPayload,
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

        fn of(program: &str) -> Self {
            let source = TypedSource::expand(program);
            let reached = DefinitionGraph::of(source.program())
                .reach()
                .traits()
                .clone();
            Self { source, reached }
        }

        fn reaches(&self, trait_name: &str) -> bool {
            let program = self.source.program();
            let binding = program
                .modules
                .iter()
                .flat_map(|module| &module.items)
                .find_map(|item| match &item.kind {
                    ItemKind::RustTrait(declaration)
                        if program.bindings.get(declaration.binding).name() == trait_name =>
                    {
                        Some(declaration.binding)
                    }
                    _ => None,
                })
                .expect("the program declares the trait");
            self.reached.reaches(binding)
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
    fn a_reached_trait_reaches_the_traits_it_requires() {
        let fixture = ReachFixture::of(ReachFixture::EMPTY_PROGRAM);
        assert!(fixture.reaches("ord"));
        assert!(fixture.reaches("partial-ord"));
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
                self_variable: self.binding("self"),
                methods: Vec::new(),
                help: None,
                requires: Vec::new(),
                structural: None,
                associated: Vec::new(),
                implementors: Vec::new(),
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

        fn reached_from(&self, names: &[BindingId]) -> ReachedTraits {
            let modules = [
                CoreModule {
                    id: ModuleId::new(0),
                    path: ModulePath::new("prelude/traits"),
                    items: self.prelude.clone(),
                    exports: Vec::new(),
                    library: None,
                },
                CoreModule {
                    id: ModuleId::new(1),
                    path: ModulePath::new("main"),
                    items: self.written.clone(),
                    exports: Vec::new(),
                    library: None,
                },
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
        giving.implementors.push(RustTraitImplementor {
            implementor: DeclaredTraits::named(given),
            associated: vec![AssociatedValue {
                binding: given,
                value: DeclaredTraits::named(projection),
                span: DeclaredTraits::span(),
            }],
        });
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
            ItemKind::RustType(
                traits.binding("kept"),
                Vec::new(),
                Vec::new(),
                DeclaredTraits::template(),
                vec![clause],
            )
        }));
    }

    #[test]
    fn a_trait_whose_rows_are_all_primitives_is_reached_unnamed() {
        let mut traits = DeclaredTraits::new();
        let mut numeric = traits.declare("numeric");
        numeric.implementors.push(RustTraitImplementor {
            implementor: TypeExpr::new(
                TypeExprKind::Primitive(PrimitiveType::I64),
                DeclaredTraits::span(),
            ),
            associated: Vec::new(),
        });
        let numeric = traits.add(numeric);
        let unrelated = traits.declare("unrelated");
        let unrelated = traits.add(unrelated);
        let reached = traits.reached_from(&[]);
        assert!(reached.reaches(numeric));
        assert!(!reached.reaches(unrelated));
    }
}
