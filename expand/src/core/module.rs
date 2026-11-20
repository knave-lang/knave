use std::rc::Rc;

use super::{
    BindingArena, BindingId, Expr, FieldLabel, ImplementorRow, ImplementorRows, Lambda, LintFacts,
    MismatchHelp, ModuleId, ModulePath, ParsedTemplate, PrimitiveType, PrintFormat,
    StructuralClause, TypeBinder, TypeBound, TypeExpr, TypeExprKind,
};
use crate::rust_trait_path::RustTraitPath;
use crate::source::Span;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
    pub hidden: bool,
}

pub const RUST_TRAIT_METHOD_MARKER: &str = "method";
pub const RUST_TRAIT_HELP_MARKER: &str = "help";
pub const RUST_TRAIT_ASSOCIATED_MARKER: &str = "associated";
pub const RUST_TRAIT_STRUCTURAL_MARKER: &str = "structural";
pub const RUST_TRAIT_REQUIRES_MARKER: &str = "requires";
pub const RUST_TRAIT_INCLUDES_MARKER: &str = "includes";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RustTraitAssociatedType {
    pub binding: BindingId,
    pub parameter: Rc<str>,
    pub rust_name: Rc<str>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AssociatedValue {
    pub binding: BindingId,
    pub value: TypeExpr,
    pub span: Span,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RustTraitImplementor {
    pub implementor: TypeExpr,
    pub associated: Vec<AssociatedValue>,
}

impl ImplementorRow for RustTraitImplementor {
    fn primitive(&self) -> Option<PrimitiveType> {
        match self.implementor.kind() {
            TypeExprKind::Primitive(primitive) => Some(*primitive),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TraitRelations {
    pub requires: Vec<RequiredTrait>,
    pub included: Option<BindingId>,
}

impl TraitRelations {
    pub fn included_clause(&self, mut name_of: impl FnMut(BindingId) -> String) -> Option<String> {
        let included = name_of(self.included?);
        Some(format!("({RUST_TRAIT_INCLUDES_MARKER} {included})"))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RequiredTrait {
    pub bound: TypeBound,
    pub span: Span,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct RustTraitDeclaration {
    pub binding: BindingId,
    pub path: Rc<str>,
    pub resolved_path: RustTraitPath,
    pub self_variable: BindingId,
    pub methods: Vec<TraitMethod>,
    pub help: Option<Rc<str>>,
    pub relations: TraitRelations,
    pub structural: Option<StructuralClause>,
    pub associated: Vec<RustTraitAssociatedType>,
    pub implementors: ImplementorRows<RustTraitImplementor>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EnumDefiner {
    pub name: Rc<str>,
    pub flat_fields: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub enum ItemKind {
    Def(BindingId, Rc<Expr>),
    Signature(BindingId, TypeExpr),
    Struct(BindingId, Vec<TypeBinder>, Vec<(FieldLabel, TypeExpr)>),
    Enum(
        BindingId,
        Vec<TypeBinder>,
        Vec<VariantDeclaration>,
        Option<EnumRust>,
        Option<EnumDefiner>,
    ),
    Derive(BindingId, Span, Vec<(BindingId, Span)>),
    RustTrait(RustTraitDeclaration),
    Trait(
        BindingId,
        Rc<str>,
        BindingId,
        Vec<TypeBound>,
        Vec<TraitAssociatedType>,
        Vec<TraitMethod>,
    ),
    Impl(
        BindingId,
        TypeExpr,
        Vec<ImplAssociatedType>,
        Vec<ImplMethod>,
    ),
    RustType(RustTypeDeclaration),
    MismatchHelp(MismatchHelp),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RustTypeDeclaration {
    pub binding: BindingId,
    pub binders: Vec<TypeBinder>,
    pub unsized_parameters: Vec<usize>,
    pub template: Rc<ParsedTemplate>,
    pub clauses: Vec<(TypeExpr, Rc<ParsedTemplate>)>,
    pub format: PrintFormat,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TraitAssociatedType {
    pub binding: BindingId,
    pub name: Rc<str>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ImplAssociatedType {
    pub trait_type: BindingId,
    pub name: Rc<str>,
    pub value: TypeExpr,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TraitMethod {
    pub binding: BindingId,
    pub signature: TypeExpr,
    pub default: Option<BindingId>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ImplMethod {
    pub binding: BindingId,
    pub trait_method: BindingId,
    pub name: Rc<str>,
    pub lambda: Rc<Lambda>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EnumRust {
    pub path: Rc<str>,
    pub template: Rc<ParsedTemplate>,
    pub template_clause: bool,
    pub bare: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VariantDeclaration {
    pub binding: BindingId,
    pub name: Rc<str>,
    pub payload: VariantPayload,
    pub span: Span,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum VariantPayload {
    None,
    Positional(Vec<TypeExpr>),
    Named(Vec<(FieldLabel, TypeExpr)>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct CoreModule {
    pub id: ModuleId,
    pub path: ModulePath,
    pub items: Vec<Item>,
    exports: ModuleExports,
    pub library: Option<Rc<str>>,
    #[serde(skip)]
    pub lint_facts: LintFacts,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ModuleExports {
    bindings: Vec<BindingId>,
    written: Vec<WrittenExport>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct WrittenExport {
    position: usize,
    name: Rc<str>,
}

impl ItemKind {
    pub fn declared_type(&self) -> Option<(BindingId, &[TypeBinder])> {
        match self {
            Self::Struct(binding, binders, _) | Self::Enum(binding, binders, ..) => {
                Some((*binding, binders))
            }
            Self::RustType(declaration) => Some((declaration.binding, &declaration.binders)),
            _ => None,
        }
    }

    pub fn method_bodies(&self) -> Vec<BindingId> {
        match self {
            Self::Impl(_, _, _, methods) => methods.iter().map(|method| method.binding).collect(),
            Self::Trait(_, _, _, _, _, methods) => {
                methods.iter().filter_map(|method| method.default).collect()
            }
            _ => Vec::new(),
        }
    }
}

impl CoreModule {
    pub fn new(id: ModuleId, path: ModulePath, library: Option<Rc<str>>) -> Self {
        Self {
            id,
            path,
            items: Vec::new(),
            exports: ModuleExports::default(),
            library,
            lint_facts: LintFacts::default(),
        }
    }

    pub fn exports(&self) -> &ModuleExports {
        &self.exports
    }

    pub fn push_export(&mut self, binding: BindingId, written: &Rc<str>, arena: &BindingArena) {
        self.exports.push(binding, written, arena);
    }

    pub fn definitions(&self) -> impl Iterator<Item = (BindingId, &Rc<Expr>)> {
        self.items.iter().filter_map(|item| match &item.kind {
            ItemKind::Def(binding, value) => Some((*binding, value)),
            _ => None,
        })
    }
}

impl ModuleExports {
    pub fn push(&mut self, binding: BindingId, written: &Rc<str>, arena: &BindingArena) {
        if arena.get(binding).name() != written.as_ref() {
            self.written.push(WrittenExport {
                position: self.bindings.len(),
                name: Rc::clone(written),
            });
        }
        self.bindings.push(binding);
    }

    pub fn bindings(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.bindings.iter().copied()
    }

    pub fn contains(&self, binding: BindingId) -> bool {
        self.bindings.contains(&binding)
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn names(&self, mut binding_name: impl FnMut(BindingId) -> String) -> Vec<String> {
        self.bindings
            .iter()
            .enumerate()
            .map(|(position, binding)| match self.written_name(position) {
                Some(written) => written.to_string(),
                None => binding_name(*binding),
            })
            .collect()
    }

    pub fn binding_named(&self, name: &str, arena: &BindingArena) -> Option<BindingId> {
        self.bindings
            .iter()
            .enumerate()
            .find(|(position, binding)| {
                self.written_name(*position)
                    .unwrap_or_else(|| arena.get(**binding).name())
                    == name
            })
            .map(|(_, binding)| *binding)
    }

    fn written_name(&self, position: usize) -> Option<&str> {
        self.written
            .iter()
            .find(|written| written.position == position)
            .map(|written| written.name.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::ModuleExports;
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind};

    struct ExportsFixture {
        arena: BindingArena,
        exports: ModuleExports,
    }

    impl ExportsFixture {
        fn new() -> Self {
            Self {
                arena: BindingArena::default(),
                exports: ModuleExports::default(),
            }
        }

        fn binding(&mut self, name: &str) -> BindingId {
            self.arena
                .add(BindingInfo::new(name.into(), BindingKind::Local, None))
        }

        fn export_as(&mut self, binding: BindingId, written: &str) {
            let written: Rc<str> = written.into();
            self.exports.push(binding, &written, &self.arena);
        }

        fn names(&self) -> Vec<String> {
            self.exports
                .names(|binding| self.arena.get(binding).name().to_string())
        }

        fn exported(&self, name: &str) -> Option<BindingId> {
            self.exports.binding_named(name, &self.arena)
        }
    }

    #[test]
    fn an_export_under_its_binding_name_prints_that_name() {
        let mut fixture = ExportsFixture::new();
        let binding = fixture.binding("int");
        fixture.export_as(binding, "int");
        assert_eq!(fixture.names(), ["int"]);
    }

    #[test]
    fn one_binding_exported_under_two_names_prints_both() {
        let mut fixture = ExportsFixture::new();
        let binding = fixture.binding("int");
        fixture.export_as(binding, "int");
        fixture.export_as(binding, "i64");
        assert_eq!(fixture.names(), ["int", "i64"]);
    }

    #[test]
    fn an_export_is_contained() {
        let mut fixture = ExportsFixture::new();
        let binding = fixture.binding("point");
        assert!(fixture.exports.is_empty());
        fixture.export_as(binding, "point");
        assert!(fixture.exports.contains(binding));
        assert_eq!(fixture.exports.bindings().collect::<Vec<_>>(), [binding]);
    }

    #[test]
    fn a_written_name_finds_its_binding() {
        let mut fixture = ExportsFixture::new();
        let shown = fixture.binding("int");
        let other = fixture.binding("bool");
        fixture.export_as(other, "bool");
        fixture.export_as(shown, "i64");
        assert_eq!(fixture.exported("i64"), Some(shown));
        assert_eq!(fixture.exported("bool"), Some(other));
        assert_eq!(fixture.exported("int"), None);
    }
}
