use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::core::{
    BindingId, BindingKind, CoreModule, ItemKind, KernelType, ModuleId, ModulePath, Namespace,
    PrimitiveType, RustTraitDeclaration, TypeExprKind,
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
    traits: Vec<Rc<str>>,
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
        let traits = traits.into_iter().map(|(_, name)| name).collect();
        Some(Self { family, traits })
    }

    pub fn family(&self) -> &'static str {
        self.family.description()
    }

    pub fn traits(&self) -> &[Rc<str>] {
        &self.traits
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
            .filter(|(_, namespace, _)| *namespace == Namespace::Type)
            .filter_map(|(name, _, binding)| {
                let rows = Self::covered_rows(family, declarations.get(&binding)?)?;
                Some((rows, Rc::clone(name)))
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
    use super::{NumberFamily, NumericBound};
    use crate::core::{
        BindingArena, BindingInfo, BindingKind, ImplementorRows, PrimitiveType,
        RustTraitDeclaration, RustTraitImplementor, TypeExpr, TypeExprKind,
    };
    use crate::source::{SourceId, Span};

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
    fn a_trait_that_also_holds_a_text_type_covers_no_family() {
        let mut fixture = DeclarationFixture::new();
        let rows = NumberFamily::Integer.members().chain([PrimitiveType::Bool]);
        let declaration = fixture.rows_of(rows);
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
