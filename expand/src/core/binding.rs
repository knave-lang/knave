use std::cell::RefCell;
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

impl BindingKind {
    pub fn defining_module(self) -> Option<ModuleId> {
        match self {
            Self::Global(module)
            | Self::Macro(module)
            | Self::Struct(module)
            | Self::Enum(module)
            | Self::Variant(module, _)
            | Self::Trait(module)
            | Self::RustType(module) => Some(module),
            _ => None,
        }
    }
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
    #[serde(skip)]
    definitions: RefCell<ModuleDefinitions>,
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
        self.definitions.take();
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

#[derive(Clone, Default)]
pub struct ModuleDefinitions {
    indexed: usize,
    by_module: FxHashMap<ModuleId, Vec<BindingId>>,
}

impl ModuleDefinitions {
    pub fn of(bindings: &BindingArena, module: ModuleId) -> Vec<BindingId> {
        let mut definitions = bindings.definitions.borrow_mut();
        definitions.index_new(bindings);
        definitions
            .by_module
            .get(&module)
            .into_iter()
            .flatten()
            .copied()
            .filter(|binding| !bindings.is_introduced(*binding))
            .collect()
    }

    fn index_new(&mut self, bindings: &BindingArena) {
        let added = bindings.ids().skip(self.indexed);
        for binding in added {
            if let Some(module) = bindings.get(binding).kind().defining_module() {
                self.by_module.entry(module).or_default().push(binding);
            }
            self.indexed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleDefinitions};
    use crate::core::ModuleId;

    struct DefinitionsFixture {
        bindings: BindingArena,
    }

    impl DefinitionsFixture {
        fn new() -> Self {
            Self {
                bindings: BindingArena::default(),
            }
        }

        fn shapes() -> ModuleId {
            ModuleId::new(1)
        }

        fn colors() -> ModuleId {
            ModuleId::new(2)
        }

        fn add(&mut self, name: &str, kind: BindingKind) -> BindingId {
            self.bindings.add(BindingInfo::new(name.into(), kind, None))
        }

        fn defined_in(&self, module: ModuleId) -> Vec<BindingId> {
            ModuleDefinitions::of(&self.bindings, module)
        }
    }

    #[test]
    fn a_module_lists_only_the_definitions_it_holds() {
        let mut fixture = DefinitionsFixture::new();
        let area = fixture.add("area", BindingKind::Global(DefinitionsFixture::shapes()));
        fixture.add("red", BindingKind::Global(DefinitionsFixture::colors()));
        fixture.add("width", BindingKind::Local);
        assert_eq!(fixture.defined_in(DefinitionsFixture::shapes()), [area]);
    }

    #[test]
    fn a_definition_added_after_a_listing_joins_the_next_listing() {
        let mut fixture = DefinitionsFixture::new();
        let area = fixture.add("area", BindingKind::Global(DefinitionsFixture::shapes()));
        assert_eq!(fixture.defined_in(DefinitionsFixture::shapes()), [area]);
        let square = fixture.add("square", BindingKind::Struct(DefinitionsFixture::shapes()));
        assert_eq!(
            fixture.defined_in(DefinitionsFixture::shapes()),
            [area, square]
        );
    }

    #[test]
    fn a_module_leaves_out_the_definitions_a_macro_introduced() {
        let mut fixture = DefinitionsFixture::new();
        let area = fixture.add("area", BindingKind::Global(DefinitionsFixture::shapes()));
        let helper = fixture.add("helper", BindingKind::Global(DefinitionsFixture::shapes()));
        fixture.defined_in(DefinitionsFixture::shapes());
        fixture.bindings.mark_introduced(helper);
        assert_eq!(fixture.defined_in(DefinitionsFixture::shapes()), [area]);
    }

    #[test]
    fn saving_the_bindings_forgets_their_definitions() {
        let mut fixture = DefinitionsFixture::new();
        fixture.add("area", BindingKind::Global(DefinitionsFixture::shapes()));
        fixture.add(
            "perimeter",
            BindingKind::Global(DefinitionsFixture::shapes()),
        );
        fixture.defined_in(DefinitionsFixture::shapes());
        fixture.bindings.take_saved();
        let red = fixture.add("red", BindingKind::Global(DefinitionsFixture::shapes()));
        assert_eq!(fixture.defined_in(DefinitionsFixture::shapes()), [red]);
    }
}
