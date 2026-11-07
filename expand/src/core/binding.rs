use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

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
    infos: Vec<Option<BindingInfo>>,
    introduced: FxHashSet<BindingId>,
    answering_macros: FxHashMap<BindingId, BindingId>,
}

impl BindingArena {
    pub fn add(&mut self, info: BindingInfo) -> BindingId {
        let id = BindingId(u32::try_from(self.infos.len()).expect("fewer than 2^32 bindings"));
        self.infos.push(Some(info));
        id
    }

    #[cfg(test)]
    pub fn ids(&self) -> impl Iterator<Item = BindingId> {
        (0..self.infos.len())
            .map(|index| BindingId(u32::try_from(index).expect("fewer than 2^32 bindings")))
    }

    pub fn iter(&self) -> impl Iterator<Item = &BindingInfo> {
        self.infos.iter().flatten()
    }

    #[inline]
    pub fn get(&self, id: BindingId) -> &BindingInfo {
        self.infos[id.0 as usize]
            .as_ref()
            .expect("a saved local decodes with its definition")
    }

    #[cfg(test)]
    pub fn decoded(&self, id: BindingId) -> Option<&BindingInfo> {
        self.infos[id.0 as usize].as_ref()
    }

    pub fn set_aside(&mut self, id: BindingId) {
        self.infos[id.0 as usize] = None;
    }

    pub fn restore(&mut self, id: BindingId, info: BindingInfo) {
        self.infos[id.0 as usize] = Some(info);
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
