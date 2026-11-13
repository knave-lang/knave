use std::cell::OnceCell;
use std::fmt;
use std::rc::Rc;

use super::{
    BindingId, Expr, FieldLabel, Lambda, ParsedTemplate, StructuralClause, TypeBinder, TypeBound,
    TypeExpr, TypeExprKind,
};
use crate::source::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModuleId(u32);

impl ModuleId {
    pub fn new(index: usize) -> Self {
        Self(u32::try_from(index).expect("fewer than 2^32 modules"))
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModulePath(Rc<str>);

impl ModulePath {
    pub fn new(path: impl Into<Rc<str>>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

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

#[derive(Clone)]
pub struct ImplementorRows {
    decoded: OnceCell<Vec<RustTraitImplementor>>,
    saved: &'static [u8],
    only_primitives: bool,
}

impl ImplementorRows {
    pub fn new(rows: Vec<RustTraitImplementor>) -> Self {
        let only_primitives = !rows.is_empty()
            && rows
                .iter()
                .all(|row| matches!(row.implementor.kind(), TypeExprKind::Primitive(_)));
        Self {
            decoded: OnceCell::from(rows),
            saved: &[],
            only_primitives,
        }
    }

    pub fn get(&self) -> &[RustTraitImplementor] {
        self.decoded.get_or_init(|| self.decode())
    }

    pub fn only_primitives(&self) -> bool {
        self.only_primitives
    }

    fn decode(&self) -> Vec<RustTraitImplementor> {
        bincode::deserialize(self.saved).expect("saved rows decode")
    }

    fn encoded(&self) -> Vec<u8> {
        bincode::serialize(self.get()).expect("rows encode")
    }
}

impl fmt::Debug for ImplementorRows {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.get()).finish()
    }
}

impl serde::Serialize for ImplementorRows {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (self.only_primitives, self.encoded().as_slice()).serialize(serializer)
    }
}

impl<'de: 'static> serde::Deserialize<'de> for ImplementorRows {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (only_primitives, saved) = <(bool, &'static [u8])>::deserialize(deserializer)?;
        Ok(Self {
            decoded: OnceCell::new(),
            saved,
            only_primitives,
        })
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
    pub self_variable: BindingId,
    pub methods: Vec<TraitMethod>,
    pub help: Option<Rc<str>>,
    pub requires: Vec<RequiredTrait>,
    pub structural: Option<StructuralClause>,
    pub associated: Vec<RustTraitAssociatedType>,
    pub implementors: ImplementorRows,
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
    RustType(
        BindingId,
        Vec<TypeBinder>,
        Vec<usize>,
        Rc<ParsedTemplate>,
        Vec<(TypeExpr, Rc<ParsedTemplate>)>,
    ),
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
    pub exports: Vec<BindingId>,
    pub library: Option<Rc<str>>,
}

impl ItemKind {
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
            exports: Vec::new(),
            library,
        }
    }

    pub fn definitions(&self) -> impl Iterator<Item = (BindingId, &Rc<Expr>)> {
        self.items.iter().filter_map(|item| match &item.kind {
            ItemKind::Def(binding, value) => Some((*binding, value)),
            _ => None,
        })
    }
}

#[cfg(test)]
mod implementor_rows_tests {
    use super::ImplementorRows;
    use crate::core::{BindingId, PrimitiveType, RustTraitImplementor, TypeExpr, TypeExprKind};
    use crate::source::{SourceId, Span};

    struct RowsFixture;

    impl RowsFixture {
        fn row(kind: TypeExprKind) -> RustTraitImplementor {
            RustTraitImplementor {
                implementor: TypeExpr::new(kind, Span::new(SourceId::new(0), 0..0)),
                associated: Vec::new(),
            }
        }

        fn named(index: u32) -> RustTraitImplementor {
            Self::row(TypeExprKind::Named(
                BindingId::from_index(index),
                Vec::new(),
            ))
        }

        fn primitive() -> RustTraitImplementor {
            Self::row(TypeExprKind::Primitive(PrimitiveType::I64))
        }

        fn reloaded(rows: &ImplementorRows) -> ImplementorRows {
            let saved = bincode::serialize(rows).expect("the rows encode");
            bincode::deserialize(saved.leak()).expect("the rows decode")
        }

        fn implemented(rows: &ImplementorRows) -> Vec<u32> {
            rows.get()
                .iter()
                .map(|row| match row.implementor.kind() {
                    TypeExprKind::Named(binding, _) => binding.index(),
                    _ => u32::MAX,
                })
                .collect()
        }
    }

    #[test]
    fn saved_rows_decode_to_the_rows_written() {
        let rows = ImplementorRows::new(vec![RowsFixture::named(3), RowsFixture::named(5)]);
        assert_eq!(
            RowsFixture::implemented(&RowsFixture::reloaded(&rows)),
            vec![3, 5]
        );
    }

    #[test]
    fn loaded_rows_save_the_same_bytes_again() {
        let rows = ImplementorRows::new(vec![RowsFixture::named(7)]);
        let saved = bincode::serialize(&rows).expect("the rows encode");
        let resaved = bincode::serialize(&RowsFixture::reloaded(&rows)).expect("the rows encode");
        assert_eq!(resaved, saved);
    }

    #[test]
    fn rows_of_primitives_only_say_so_without_decoding() {
        let primitives = ImplementorRows::new(vec![RowsFixture::primitive()]);
        let mixed = ImplementorRows::new(vec![RowsFixture::primitive(), RowsFixture::named(1)]);
        let empty = ImplementorRows::new(Vec::new());
        assert!(RowsFixture::reloaded(&primitives).only_primitives());
        assert!(!RowsFixture::reloaded(&mixed).only_primitives());
        assert!(!RowsFixture::reloaded(&empty).only_primitives());
    }
}
