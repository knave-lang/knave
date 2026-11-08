use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::saved_bindings::{BindingTables, SavedBindings};
use super::{Builtin, KernelForm, KernelType, ModuleId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VariantInfo {
    pub owner: BindingId,
    pub index: usize,
    pub arity: usize,
}
use crate::source::Span;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct BindingId(u32);

impl BindingId {
    pub fn from_index(index: u32) -> Self {
        Self(index)
    }

    pub fn index(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Namespace {
    Value,
    Type,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Declaration {
    Value,
    Macro,
    Type,
}

impl Declaration {
    pub fn namespace(self) -> Namespace {
        match self {
            Self::Value | Self::Macro => Namespace::Value,
            Self::Type => Namespace::Type,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BindingKind {
    KernelForm(KernelForm),
    KernelType(KernelType),
    Boolean(bool),
    CodeBuiltin(Builtin),
    Macro(ModuleId),
    Global(ModuleId),
    Local,
    Struct(ModuleId),
    Enum(ModuleId),
    Variant(ModuleId, VariantInfo),
    Trait(ModuleId),
    RustType(ModuleId),
    TypeVariable,
    AssociatedType,
    Lifetime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BindingInfo {
    name: Rc<str>,
    kind: BindingKind,
    span: Option<Span>,
}

impl BindingInfo {
    pub fn new(name: Rc<str>, kind: BindingKind, span: Option<Span>) -> Self {
        Self { name, kind, span }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn shared_name(&self) -> Rc<str> {
        Rc::clone(&self.name)
    }

    pub fn kind(&self) -> BindingKind {
        self.kind
    }

    pub fn span(&self) -> Option<Span> {
        self.span
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BindingArena {
    #[serde(skip)]
    saved: Rc<SavedBindings>,
    infos: Vec<BindingInfo>,
    introduced: FxHashSet<BindingId>,
    answering_macros: FxHashMap<BindingId, BindingId>,
}

impl BindingArena {
    pub fn add(&mut self, info: BindingInfo) -> BindingId {
        let index = self.saved.count() + self.infos.len();
        self.infos.push(info);
        BindingId(u32::try_from(index).expect("fewer than 2^32 bindings"))
    }

    pub fn take_saved(&mut self) -> BindingTables {
        let tables = BindingTables::of(self.infos.iter());
        self.infos.clear();
        tables
    }

    pub fn adopt_saved(&mut self, saved: SavedBindings) {
        assert!(
            self.infos.is_empty(),
            "saved bindings come before any other"
        );
        self.saved = Rc::new(saved);
    }

    pub fn ids(&self) -> impl Iterator<Item = BindingId> {
        let count = self.saved.count() + self.infos.len();
        (0..count).map(|index| BindingId(u32::try_from(index).expect("fewer than 2^32 bindings")))
    }

    pub fn is_saved(&self, id: BindingId) -> bool {
        (id.0 as usize) < self.saved.count()
    }

    #[cfg(test)]
    pub fn is_decoded(&self, id: BindingId) -> bool {
        (id.0 as usize) >= self.saved.count() || self.saved.is_decoded(id.0 as usize)
    }

    #[inline]
    pub fn get(&self, id: BindingId) -> &BindingInfo {
        let index = id.0 as usize;
        match index.checked_sub(self.saved.count()) {
            Some(own) => &self.infos[own],
            None => self.saved.get(index),
        }
    }

    pub fn mark_introduced(&mut self, id: BindingId) {
        self.introduced.insert(id);
    }

    pub fn is_introduced(&self, id: BindingId) -> bool {
        self.introduced.contains(&id)
    }

    pub fn record_answering_macro(&mut self, function: BindingId, answering: BindingId) {
        self.answering_macros.insert(function, answering);
    }

    pub fn answering_macro(&self, function: BindingId) -> Option<BindingId> {
        self.answering_macros.get(&function).copied()
    }
}
