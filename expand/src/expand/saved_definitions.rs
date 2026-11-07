use std::rc::Rc;

use rustc_hash::FxHashSet;

use super::saved_index::{BindingIndex, SavedWords};
use crate::core::{
    Binders, BindingArena, BindingId, BindingInfo, CoreModule, Expr, Item, ItemKind, ModuleId,
    References,
};

#[derive(Default, serde::Serialize)]
pub struct SavedTables {
    entries: Vec<u8>,
    references: Vec<u8>,
    items: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct SavedDefinitions {
    index: BindingIndex,
    references: &'static [u8],
    items: &'static [u8],
}

impl Default for SavedDefinitions {
    fn default() -> Self {
        Self::borrowing(&[], &[], &[])
    }
}

impl<'de: 'static> serde::Deserialize<'de> for SavedDefinitions {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (entries, references, items) = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self::borrowing(entries, references, items))
    }
}

#[derive(Clone, Copy)]
struct SavedItem {
    binding: u32,
    module: u32,
    position: u32,
    item_start: u32,
    item_end: u32,
    references_start: u32,
    references_end: u32,
}

impl SavedItem {
    const WORDS: usize = 7;

    fn read(index: &BindingIndex, row: usize) -> Self {
        let word = |field: usize| index.word(row, field);
        Self {
            binding: word(0),
            module: word(1),
            position: word(2),
            item_start: word(3),
            item_end: word(4),
            references_start: word(5),
            references_end: word(6),
        }
    }

    fn write(self, bytes: &mut Vec<u8>) {
        let words = [
            self.binding,
            self.module,
            self.position,
            self.item_start,
            self.item_end,
            self.references_start,
            self.references_end,
        ];
        SavedWords::write(&words, bytes);
    }
}

impl SavedDefinitions {
    fn borrowing(entries: &'static [u8], references: &'static [u8], items: &'static [u8]) -> Self {
        Self {
            index: BindingIndex::new(entries, SavedItem::WORDS),
            references,
            items,
        }
    }

    fn entry(&self, row: usize) -> SavedItem {
        SavedItem::read(&self.index, row)
    }

    fn bindings(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.index.all_rows().map(|row| self.index.binding(row))
    }

    fn entries_of(&self, binding: BindingId) -> impl Iterator<Item = SavedItem> + '_ {
        self.index.rows_of(binding).map(|row| self.entry(row))
    }

    fn item(&self, entry: SavedItem) -> (Item, Vec<(BindingId, BindingInfo)>) {
        let encoded = &self.items[entry.item_start as usize..entry.item_end as usize];
        bincode::deserialize(encoded).expect("a saved prelude item decodes")
    }

    fn references(&self, entry: SavedItem) -> impl Iterator<Item = BindingId> + '_ {
        self.references[entry.references_start as usize..entry.references_end as usize]
            .chunks_exact(SavedWords::SIZE)
            .map(|word| BindingId::from_index(SavedWords::read(word)))
    }

    pub fn decode_value(
        &self,
        binding: BindingId,
        bindings: &mut BindingArena,
    ) -> Option<Rc<Expr>> {
        self.entries_of(binding).find_map(|entry| {
            let (item, locals) = self.item(entry);
            let ItemKind::Def(_, value) = item.kind else {
                return None;
            };
            Self::restore_locals(bindings, locals);
            Some(value)
        })
    }

    fn restore_locals(bindings: &mut BindingArena, locals: Vec<(BindingId, BindingInfo)>) {
        for (binding, info) in locals {
            bindings.restore(binding, info);
        }
    }
}

#[derive(Default)]
pub struct PendingDefinitions {
    saved: SavedDefinitions,
    restored: FxHashSet<BindingId>,
}

impl PendingDefinitions {
    pub fn new(saved: SavedDefinitions) -> Self {
        Self {
            saved,
            restored: FxHashSet::default(),
        }
    }

    pub fn saved_bindings(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.saved.bindings()
    }

    pub fn undecoded(&self) -> impl Iterator<Item = BindingId> + '_ {
        self.saved
            .bindings()
            .filter(|binding| !self.restored.contains(binding))
    }

    pub fn restore(
        &mut self,
        modules: &mut [CoreModule],
        bindings: &mut BindingArena,
        roots: Vec<BindingId>,
    ) {
        let mut found = self.reached_entries(roots);
        found.sort_by_key(|entry| (entry.module, entry.position));
        for entry in found {
            let module = ModuleId::new(entry.module as usize);
            let holder = modules.iter_mut().find(|candidate| candidate.id == module);
            let holder = holder.expect("a saved item names a loaded module");
            let (item, locals) = self.saved.item(entry);
            SavedDefinitions::restore_locals(bindings, locals);
            holder.items.push(item);
        }
    }

    fn reached_entries(&mut self, roots: Vec<BindingId>) -> Vec<SavedItem> {
        let mut pending = roots;
        let mut found = Vec::new();
        while let Some(binding) = pending.pop() {
            if self.restored.insert(binding) {
                for entry in self.saved.entries_of(binding) {
                    pending.extend(self.saved.references(entry));
                    found.push(entry);
                }
            }
        }
        found
    }
}

struct EncodedItem {
    entry: SavedItem,
    item: Vec<u8>,
    references: Vec<BindingId>,
}

pub struct DefinitionSaver<'a> {
    bindings: &'a BindingArena,
    kept: FxHashSet<BindingId>,
    encoded: Vec<EncodedItem>,
    set_aside: Vec<BindingId>,
}

impl<'a> DefinitionSaver<'a> {
    pub fn new(bindings: &'a BindingArena, string_view: Option<BindingId>) -> Self {
        Self {
            bindings,
            kept: string_view.into_iter().collect(),
            encoded: Vec::new(),
            set_aside: Vec::new(),
        }
    }

    pub fn take_from(&mut self, module: &mut CoreModule) {
        let items = std::mem::take(&mut module.items);
        self.kept
            .extend(items.iter().flat_map(|item| item.kind.method_bodies()));
        let saved = self.saved_bindings(&items);
        let mut taken = Vec::new();
        for (position, item) in items.into_iter().enumerate() {
            match Self::named_binding(&item).filter(|binding| saved.contains(binding)) {
                Some(binding) => taken.push((binding, position, item)),
                None => module.items.push(item),
            }
        }
        for (binding, position, item) in taken {
            let locals = self.locals_of(&item);
            self.encode(binding, module.id, position, &item, locals);
        }
    }

    fn locals_of(&mut self, item: &Item) -> Vec<(BindingId, BindingInfo)> {
        let ItemKind::Def(_, value) = &item.kind else {
            return Vec::new();
        };
        let locals = Binders::of(value);
        self.set_aside.extend(&locals);
        let info = |binding: BindingId| (binding, self.bindings.get(binding).clone());
        locals.into_iter().map(info).collect()
    }

    fn saved_bindings(&self, items: &[Item]) -> FxHashSet<BindingId> {
        let definitions = items.iter().filter_map(|item| match item.kind {
            ItemKind::Def(binding, _) => Some(binding),
            _ => None,
        });
        definitions
            .filter(|binding| !self.kept.contains(binding))
            .collect()
    }

    fn named_binding(item: &Item) -> Option<BindingId> {
        match item.kind {
            ItemKind::Def(binding, _) | ItemKind::Signature(binding, _) => Some(binding),
            _ => None,
        }
    }

    fn encode(
        &mut self,
        binding: BindingId,
        module: ModuleId,
        position: usize,
        item: &Item,
        locals: Vec<(BindingId, BindingInfo)>,
    ) {
        self.encoded.push(EncodedItem {
            entry: SavedItem {
                binding: binding.index(),
                module: SavedWords::index(module.index()),
                position: SavedWords::index(position),
                item_start: 0,
                item_end: 0,
                references_start: 0,
                references_end: 0,
            },
            item: bincode::serialize(&(item, locals)).expect("every prelude item serializes"),
            references: self.references_of(item),
        });
    }

    fn references_of(&self, item: &Item) -> Vec<BindingId> {
        match &item.kind {
            ItemKind::Def(_, value) => References::of(value, self.bindings).into_globals(),
            _ => Vec::new(),
        }
    }

    pub fn finish(mut self) -> (SavedTables, Vec<BindingId>) {
        self.encoded
            .sort_by_key(|encoded| (encoded.entry.binding, encoded.entry.position));
        let (mut entries, mut references, mut items) = (Vec::new(), Vec::new(), Vec::new());
        for encoded in self.encoded {
            let mut entry = encoded.entry;
            entry.item_start = SavedWords::index(items.len());
            items.extend(encoded.item);
            entry.item_end = SavedWords::index(items.len());
            entry.references_start = SavedWords::index(references.len());
            references.extend(
                encoded
                    .references
                    .iter()
                    .flat_map(|reference| reference.index().to_le_bytes()),
            );
            entry.references_end = SavedWords::index(references.len());
            entry.write(&mut entries);
        }
        let tables = SavedTables {
            entries,
            references,
            items,
        };
        (tables, self.set_aside)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rustc_hash::FxHashSet;

    use super::{DefinitionSaver, SavedDefinitions};
    use crate::core::Binders;
    use crate::core::{BindingArena, BindingId, BindingKind, ItemKind, ModulePath};
    use crate::expand::{Expander, PreludeState, Program};
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct BuiltModule {
        program: Program,
    }

    impl BuiltModule {
        const TRAIT_AND_IMPL: &str = "
            (trait named
              (name (fn ((ref self)) i64) (self) 1)
              (rank (fn ((ref self)) i64)))
            (defstruct car :year i64)
            (impl named car
              (rank (c) (:year c)))
            (defn helper () 1)
            (defn main ())";
        const METHOD_BODIES: usize = 2;

        fn expand(text: &str) -> Self {
            let mut sources = SourceMap::default();
            let entry = sources.add(LoadedProgram::FILE, text);
            let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
            let program = Expander::new(&mut sources, module_sources)
                .expand(ModulePath::new(LoadedProgram::MODULE), entry)
                .unwrap_or_else(|_| panic!("the module expands"));
            Self { program }
        }

        fn method_bodies(&self) -> Vec<BindingId> {
            let items = &self.program.entry_module().items;
            items
                .iter()
                .flat_map(|item| item.kind.method_bodies())
                .collect()
        }

        fn definition(&self, name: &str) -> BindingId {
            let program = &self.program;
            let found = program
                .entry_module()
                .definitions()
                .map(|(binding, _)| binding);
            let mut found = found.filter(|binding| program.bindings.get(*binding).name() == name);
            found.next().expect("the module defines the name")
        }

        fn defines(&self, binding: BindingId) -> bool {
            let mut definitions = self.program.entry_module().definitions();
            definitions.any(|(defined, _)| defined == binding)
        }

        fn set_aside_locals(&mut self) -> Vec<BindingId> {
            let entry = self.program.entry;
            let mut saver = DefinitionSaver::new(&self.program.bindings, None);
            let module = self
                .program
                .modules
                .iter_mut()
                .find(|module| module.id == entry);
            saver.take_from(module.expect("the entry module is expanded"));
            let (_, set_aside) = saver.finish();
            set_aside
        }

        fn method_locals(&self) -> Vec<BindingId> {
            let items = &self.program.entry_module().items;
            let methods: Vec<BindingId> = self.method_bodies();
            let bodies = items.iter().filter_map(|item| match &item.kind {
                ItemKind::Def(binding, value) if methods.contains(binding) => Some(value),
                _ => None,
            });
            bodies.flat_map(|value| Binders::of(value)).collect()
        }

        fn save_definitions(&mut self) -> SavedDefinitions {
            let entry = self.program.entry;
            let mut saver = DefinitionSaver::new(&self.program.bindings, None);
            let module = self
                .program
                .modules
                .iter_mut()
                .find(|module| module.id == entry);
            saver.take_from(module.expect("the entry module is expanded"));
            let (tables, _) = saver.finish();
            let encoded = bincode::serialize(&tables).expect("the tables serialize");
            bincode::deserialize(encoded.leak()).expect("the tables load")
        }

        fn saves(saved: &SavedDefinitions, binding: BindingId) -> bool {
            saved.entries_of(binding).next().is_some()
        }
    }

    struct LoadedProgram;

    impl LoadedProgram {
        const MODULE: &str = "program";
        const FILE: &str = "program.rslisp";
        const EMPTY: &str = "(defn main ())";
        const SUMMING: &str = "(defn main () (println (sum (vec 1 2))))";
        const MACRO_HELPER: &str = "(use code rest)
            (use prelude/macros map-list)
            (def one (macro (form) (do (map-list (fn (item) item) (rest form)) 1)))
            (defn main () (println (one a b)))";

        fn expand(text: &str) -> Program {
            let state = PreludeState::load(PreludeState::build().save().leak())
                .expect("the saved prelude loads");
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::FILE, text);
            let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
            Expander::with_prelude(&mut sources, module_sources, state)
                .expand(ModulePath::new(Self::MODULE), entry)
                .unwrap_or_else(|_| panic!("the program expands"))
        }

        fn holds(program: &Program, name: &str, named: fn(&ItemKind) -> Option<BindingId>) -> bool {
            program
                .modules
                .iter()
                .flat_map(|module| &module.items)
                .filter_map(|item| named(&item.kind))
                .any(|binding| program.bindings.get(binding).name() == name)
        }

        fn holds_definition(program: &Program, name: &str) -> bool {
            Self::holds(program, name, |kind| match kind {
                ItemKind::Def(binding, _) => Some(*binding),
                _ => None,
            })
        }

        fn holds_signature(program: &Program, name: &str) -> bool {
            Self::holds(program, name, |kind| match kind {
                ItemKind::Signature(binding, _) => Some(*binding),
                _ => None,
            })
        }

        fn undecoded_names(program: &Program) -> FxHashSet<String> {
            let names = program
                .pending_definitions
                .undecoded()
                .map(|binding| program.bindings.get(binding).name().to_string());
            names.collect()
        }

        fn defined_binding(program: &Program, name: &str) -> BindingId {
            program
                .modules
                .iter()
                .flat_map(|module| module.definitions())
                .map(|(binding, _)| binding)
                .find(|binding| program.bindings.get(*binding).name() == name)
                .expect("the program holds the definition")
        }

        fn local_slots_left_undecoded(program: &Program) -> usize {
            program.bindings.ids().count() - program.bindings.iter().count()
        }

        fn binding_of(program: &Program, name: &str) -> BindingId {
            program
                .pending_definitions
                .undecoded()
                .find(|binding| program.bindings.get(*binding).name() == name)
                .expect("the name is undecoded")
        }
    }

    #[test]
    fn a_program_decodes_no_definition_it_does_not_reach() {
        let program = LoadedProgram::expand(LoadedProgram::EMPTY);
        assert!(!LoadedProgram::holds_definition(&program, "sum"));
        assert!(LoadedProgram::undecoded_names(&program).contains("sum"));
    }

    #[test]
    fn a_reached_definition_returns_with_what_it_reaches() {
        let program = LoadedProgram::expand(LoadedProgram::SUMMING);
        assert!(LoadedProgram::holds_definition(&program, "sum"));
        assert!(LoadedProgram::holds_definition(&program, "add"));
    }

    #[test]
    fn a_reached_definition_returns_with_its_signature() {
        let program = LoadedProgram::expand(LoadedProgram::SUMMING);
        assert!(LoadedProgram::holds_signature(&program, "sum"));
    }

    #[test]
    fn a_restored_definition_leaves_the_undecoded_names() {
        let program = LoadedProgram::expand(LoadedProgram::SUMMING);
        assert!(!LoadedProgram::undecoded_names(&program).contains("sum"));
    }

    #[test]
    fn a_checked_root_returns_though_nothing_reaches_it() {
        let mut program = LoadedProgram::expand(LoadedProgram::EMPTY);
        let sum = LoadedProgram::binding_of(&program, "sum");
        program.check_also(sum);
        assert!(LoadedProgram::holds_definition(&program, "sum"));
    }

    #[test]
    fn a_restore_keeps_the_order_of_its_module_within_one_batch() {
        let program = LoadedProgram::expand(LoadedProgram::SUMMING);
        let names: Vec<&str> = program
            .modules
            .iter()
            .flat_map(|module| module.definitions())
            .map(|(binding, _)| program.bindings.get(binding).name())
            .collect();
        let position = |name| {
            let found = names.iter().position(|found| *found == name);
            found.expect("the program holds the definition")
        };
        assert!(position("fold-items") < position("sum"));
    }

    #[test]
    fn a_definition_returns_once() {
        let mut program = LoadedProgram::expand(LoadedProgram::SUMMING);
        let sum = LoadedProgram::defined_binding(&program, "sum");
        program.check_also(sum);
        let copies = program
            .modules
            .iter()
            .flat_map(|module| module.definitions())
            .filter(|(binding, _)| *binding == sum)
            .count();
        assert_eq!(copies, 1);
    }

    #[test]
    fn an_unreached_signature_stays_undecoded() {
        let program = LoadedProgram::expand(LoadedProgram::EMPTY);
        assert!(!LoadedProgram::holds_signature(&program, "sum"));
    }

    #[test]
    fn a_saver_keeps_every_method_body_in_its_module() {
        let mut built = BuiltModule::expand(BuiltModule::TRAIT_AND_IMPL);
        let methods = built.method_bodies();
        assert_eq!(methods.len(), BuiltModule::METHOD_BODIES);
        let saved = built.save_definitions();
        for method in methods {
            assert!(built.defines(method));
            assert!(!BuiltModule::saves(&saved, method));
        }
    }

    #[test]
    fn a_saver_keeps_the_locals_of_every_method_body() {
        let mut built = BuiltModule::expand(BuiltModule::TRAIT_AND_IMPL);
        let method_locals = built.method_locals();
        assert!(!method_locals.is_empty());
        let set_aside = built.set_aside_locals();
        assert!(!method_locals.iter().any(|local| set_aside.contains(local)));
    }

    #[test]
    fn a_saver_saves_a_plain_definition() {
        let mut built = BuiltModule::expand(BuiltModule::TRAIT_AND_IMPL);
        let helper = built.definition("helper");
        let saved = built.save_definitions();
        assert!(!built.defines(helper));
        assert!(BuiltModule::saves(&saved, helper));
    }

    #[test]
    fn an_unreached_definition_leaves_its_locals_undecoded() {
        let program = LoadedProgram::expand(LoadedProgram::EMPTY);
        assert!(LoadedProgram::local_slots_left_undecoded(&program) > 0);
        assert!(
            !program
                .bindings
                .iter()
                .any(|info| info.name() == "values" && matches!(info.kind(), BindingKind::Local))
        );
    }

    #[test]
    fn a_reached_definition_decodes_its_locals() {
        let program = LoadedProgram::expand(LoadedProgram::SUMMING);
        assert!(
            program
                .bindings
                .iter()
                .any(|info| info.name() == "values" && matches!(info.kind(), BindingKind::Local))
        );
    }

    #[test]
    fn a_checked_root_decodes_its_locals() {
        let mut program = LoadedProgram::expand(LoadedProgram::EMPTY);
        let sum = LoadedProgram::binding_of(&program, "sum");
        program.check_also(sum);
        assert!(
            program
                .bindings
                .iter()
                .any(|info| info.name() == "values" && matches!(info.kind(), BindingKind::Local))
        );
    }

    #[test]
    fn the_string_view_stays_decoded() {
        let program = LoadedProgram::expand(LoadedProgram::EMPTY);
        let view = program.string_view.expect("the prelude views strings");
        assert!(
            !program
                .pending_definitions
                .undecoded()
                .any(|bound| bound == view)
        );
    }

    #[test]
    fn a_macro_runs_a_saved_prelude_function_at_compile_time() {
        let program = LoadedProgram::expand(LoadedProgram::MACRO_HELPER);
        assert!(!LoadedProgram::holds_definition(&program, "map-list"));
    }

    #[test]
    fn no_saved_definitions_hold_no_value() {
        let unknown = BindingId::from_index(0);
        let mut bindings = BindingArena::default();
        let saved = SavedDefinitions::default();
        assert!(saved.decode_value(unknown, &mut bindings).is_none());
    }
}
