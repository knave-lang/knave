use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::core::{
    BindingId, BindingKind, CoreModule, ItemKind, KernelType, ModuleId, ModulePath, PrimitiveType,
    RustTraitDeclaration, TypeExprKind,
};
use crate::modules::{ModuleRecord, ModuleRegistry, ModuleSources};

#[derive(Clone, Copy, PartialEq, Eq)]
enum NumberFamily {
    Integer,
    Float,
}

impl NumberFamily {
    fn of(primitive: PrimitiveType) -> Option<Self> {
        if primitive.is_integer() {
            Some(Self::Integer)
        } else if primitive.is_float() {
            Some(Self::Float)
        } else {
            None
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Float => "floating-point",
        }
    }

    fn members(self) -> impl Iterator<Item = PrimitiveType> {
        PrimitiveType::all().filter(move |primitive| Self::of(*primitive) == Some(self))
    }
}

pub(super) struct NumericBound {
    family: NumberFamily,
    first: Rc<str>,
    others: Vec<Rc<str>>,
}

impl NumericBound {
    pub fn of_kind(
        kind: BindingKind,
        modules: &ModuleRegistry,
        expanded: &FxHashMap<ModuleId, CoreModule>,
    ) -> Option<Self> {
        let BindingKind::KernelType(KernelType::Primitive(primitive)) = kind else {
            return None;
        };
        let family = NumberFamily::of(primitive)?;
        let prelude = modules.get(modules.find(&ModulePath::new(ModuleSources::PRELUDE))?);
        let mut traits = Self::covering_traits(family, prelude, &Self::declarations(expanded));
        traits.sort();
        let mut names = traits.into_iter().map(|(_, name)| name);
        let first = names.next()?;
        Some(Self {
            family,
            first,
            others: names.collect(),
        })
    }

    pub fn family(&self) -> &'static str {
        self.family.description()
    }

    pub fn first(&self) -> &str {
        &self.first
    }

    pub fn others(&self) -> &[Rc<str>] {
        &self.others
    }

    fn declarations(
        expanded: &FxHashMap<ModuleId, CoreModule>,
    ) -> FxHashMap<BindingId, &RustTraitDeclaration> {
        expanded
            .values()
            .flat_map(|module| module.items.iter())
            .filter_map(|item| match &item.kind {
                ItemKind::RustTrait(declaration) => Some((declaration.binding, declaration)),
                _ => None,
            })
            .collect()
    }

    fn covering_traits(
        family: NumberFamily,
        prelude: &ModuleRecord,
        declarations: &FxHashMap<BindingId, &RustTraitDeclaration>,
    ) -> Vec<(usize, Rc<str>)> {
        prelude
            .exports()
            .entries()
            .filter_map(|(name, _, binding)| {
                let rows = Self::covered_rows(family, declarations.get(&binding)?)?;
                Some((rows, name))
            })
            .collect()
    }

    fn covered_rows(family: NumberFamily, declaration: &RustTraitDeclaration) -> Option<usize> {
        if !declaration.implementors.all_rows_primitive() {
            return None;
        }
        let primitives: Vec<PrimitiveType> = declaration
            .implementors
            .get()
            .iter()
            .filter_map(|row| match row.implementor.kind() {
                TypeExprKind::Primitive(primitive) => Some(*primitive),
                _ => None,
            })
            .collect();
        let only_numbers = primitives
            .iter()
            .all(|primitive| NumberFamily::of(*primitive).is_some());
        let whole_family = family.members().all(|member| primitives.contains(&member));
        (only_numbers && whole_family).then_some(primitives.len())
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::{NumberFamily, NumericBound};
    use crate::core::{
        BindingArena, BindingInfo, BindingKind, CoreModule, ImplementorRows, Item, ItemKind,
        KernelType, ModuleId, ModulePath, Namespace, PrimitiveType, RustTraitDeclaration,
        RustTraitImplementor, TypeExpr, TypeExprKind,
    };
    use crate::modules::{ModuleRegistry, ModuleScopes, ModuleSources};
    use crate::scope::{ScopeKind, Scopes};
    use crate::source::{SourceId, Span};

    struct PreludeFixture {
        declarations: DeclarationFixture,
        modules: ModuleRegistry,
        expanded: FxHashMap<ModuleId, CoreModule>,
        prelude: ModuleId,
    }

    impl PreludeFixture {
        fn new() -> Self {
            let mut scopes = Scopes::default();
            let mut modules = ModuleRegistry::default();
            let path = ModulePath::new(ModuleSources::PRELUDE);
            let prelude = modules.register(
                path.clone(),
                ModuleScopes::new(
                    scopes.fresh(ScopeKind::Module),
                    scopes.fresh(ScopeKind::Module),
                ),
            );
            let mut expanded = FxHashMap::default();
            expanded.insert(prelude, CoreModule::new(prelude, path, None));
            Self {
                declarations: DeclarationFixture::new(),
                modules,
                expanded,
                prelude,
            }
        }

        fn integer_trait(&mut self, name: &str) {
            let declaration = self.declarations.rows_of(NumberFamily::Integer.members());
            self.modules.get_mut(self.prelude).export(
                name.into(),
                Namespace::Type,
                declaration.binding,
            );
            self.declare(declaration);
        }

        fn private_integer_trait(&mut self) {
            let declaration = self.declarations.rows_of(NumberFamily::Integer.members());
            self.declare(declaration);
        }

        fn declare(&mut self, declaration: RustTraitDeclaration) {
            let module = self
                .expanded
                .get_mut(&self.prelude)
                .expect("the prelude module");
            module.items.push(Item {
                kind: ItemKind::RustTrait(declaration),
                span: DeclarationFixture::span(),
                hidden: false,
            });
        }

        fn traits_for(&self, primitive: PrimitiveType) -> Option<Vec<String>> {
            let kind = BindingKind::KernelType(KernelType::Primitive(primitive));
            let bound = NumericBound::of_kind(kind, &self.modules, &self.expanded)?;
            let others = bound.others().iter().map(|name| name.to_string());
            Some(
                std::iter::once(bound.first().to_string())
                    .chain(others)
                    .collect(),
            )
        }
    }

    #[test]
    fn the_names_come_from_the_prelude_rows() {
        let mut prelude = PreludeFixture::new();
        prelude.integer_trait("whole");
        assert_eq!(
            prelude.traits_for(PrimitiveType::I64),
            Some(vec!["whole".to_string()])
        );
    }

    #[test]
    fn a_trait_the_prelude_does_not_export_is_left_out() {
        let mut prelude = PreludeFixture::new();
        prelude.private_integer_trait();
        assert_eq!(prelude.traits_for(PrimitiveType::I64), None);
    }

    #[test]
    fn no_trait_of_the_family_gives_no_bound() {
        let mut prelude = PreludeFixture::new();
        prelude.integer_trait("whole");
        assert_eq!(prelude.traits_for(PrimitiveType::F64), None);
    }

    #[test]
    fn a_type_outside_the_number_families_gives_no_bound() {
        let mut prelude = PreludeFixture::new();
        prelude.integer_trait("whole");
        assert_eq!(prelude.traits_for(PrimitiveType::Bool), None);
    }

    struct DeclarationFixture {
        arena: BindingArena,
    }

    impl DeclarationFixture {
        fn new() -> Self {
            Self {
                arena: BindingArena::default(),
            }
        }

        fn rows_of(
            &mut self,
            primitives: impl Iterator<Item = PrimitiveType>,
        ) -> RustTraitDeclaration {
            let rows = primitives
                .map(|primitive| RustTraitImplementor {
                    implementor: TypeExpr::new(TypeExprKind::Primitive(primitive), Self::span()),
                    associated: Vec::new(),
                })
                .collect();
            RustTraitDeclaration {
                binding: self.binding("marker"),
                path: "Marker".into(),
                resolved_path: "Marker".into(),
                self_variable: self.binding("self"),
                methods: Vec::new(),
                help: None,
                requires: Vec::new(),
                structural: None,
                associated: Vec::new(),
                implementors: ImplementorRows::new(rows),
            }
        }

        fn binding(&mut self, name: &str) -> crate::core::BindingId {
            self.arena
                .add(BindingInfo::new(name.into(), BindingKind::Local, None))
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }
    }

    #[test]
    fn a_trait_over_every_integer_covers_the_integers() {
        let mut fixture = DeclarationFixture::new();
        let declaration = fixture.rows_of(NumberFamily::Integer.members());
        assert_eq!(
            NumericBound::covered_rows(NumberFamily::Integer, &declaration),
            Some(12)
        );
    }

    #[test]
    fn a_trait_that_also_holds_a_non_number_covers_no_family() {
        let mut fixture = DeclarationFixture::new();
        let rows = NumberFamily::Integer.members().chain([PrimitiveType::Bool]);
        let declaration = fixture.rows_of(rows);
        assert_eq!(
            NumericBound::covered_rows(NumberFamily::Integer, &declaration),
            None
        );
    }

    #[test]
    fn a_trait_that_also_holds_a_type_with_arguments_covers_no_family() {
        let mut fixture = DeclarationFixture::new();
        let mut declaration = fixture.rows_of(NumberFamily::Integer.members());
        let vector = fixture.binding("vec");
        let mut rows = declaration.implementors.get().to_vec();
        rows.push(RustTraitImplementor {
            implementor: TypeExpr::new(
                TypeExprKind::Named(vector, Vec::new()),
                DeclarationFixture::span(),
            ),
            associated: Vec::new(),
        });
        declaration.implementors = ImplementorRows::new(rows);
        assert_eq!(
            NumericBound::covered_rows(NumberFamily::Integer, &declaration),
            None
        );
    }

    #[test]
    fn a_trait_over_one_float_covers_no_family() {
        let mut fixture = DeclarationFixture::new();
        let declaration = fixture.rows_of([PrimitiveType::F64].into_iter());
        assert_eq!(
            NumericBound::covered_rows(NumberFamily::Float, &declaration),
            None
        );
    }
}
