use std::fmt;
use std::path::PathBuf;

use super::origin_tables::OriginTables;
use super::saved_definitions::{SavedDefinitions, SavedTables};
use super::saved_notes::{NoteTables, SavedNotes};
use super::saved_origins::SavedOrigins;
use super::saved_values::{SavedValues, ValueTables};
use super::{Expander, ExpansionTables, LoweredGlobals};
use crate::core::{BindingTables, SavedBindings};
use crate::core::{ModuleId, ModulePath};
use crate::diagnostic::DiagnosticRenderer;
use crate::interpret::{Closure, Value};
use crate::modules::{ModuleRecord, ModuleSource, ModuleSources};
use crate::reader::Form;
use crate::scope::{NameTables, SavedNames};
use crate::source::{SourceId, SourceMap, Span};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PreludeState {
    first_source: SourceId,
    prelude: ModuleId,
    modules: Vec<ModuleRecord>,
    tables: ExpansionTables,
    lowered_globals: LoweredGlobals,
    #[serde(skip)]
    definitions: PreludeDefinitions,
}

enum PreludeDefinitions {
    Built(BuiltTables),
    Loaded(LoadedTables),
}

#[derive(serde::Serialize)]
struct BuiltTables {
    bindings: BindingTables,
    names: NameTables,
    origins: OriginTables,
    definitions: SavedTables,
    macros: ValueTables,
    globals: ValueTables,
    notes: NoteTables,
}

#[derive(Default, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
struct LoadedTables {
    bindings: SavedBindings,
    names: SavedNames,
    origins: SavedOrigins,
    definitions: SavedDefinitions,
    macros: SavedValues<Closure>,
    globals: SavedValues<Value>,
    notes: SavedNotes,
}

impl Default for PreludeDefinitions {
    fn default() -> Self {
        Self::Loaded(LoadedTables::default())
    }
}

#[derive(Debug)]
pub struct UnreadablePrelude(bincode::Error);

impl fmt::Display for UnreadablePrelude {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "the saved prelude state is unreadable: {}",
            self.0
        )
    }
}

impl PreludeState {
    const PLACEHOLDER_ENTRY: &str = "saved-prelude-entry";

    pub fn build() -> Self {
        let mut sources = SourceMap::default();
        let entry = sources.add(Self::PLACEHOLDER_ENTRY, "");
        let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
        Expander::new(&mut sources, module_sources).capture_prelude(entry)
    }

    fn add_sources(&self, expander: &mut Expander<'_>) {
        assert_eq!(
            expander.sources.next_id(),
            self.first_source,
            "the entry is the only source before the prelude"
        );
        for record in &self.modules {
            expander.add_prelude_source(record.path());
        }
    }

    #[cfg(test)]
    pub(super) fn installed_tables(self) -> ExpansionTables {
        let PreludeDefinitions::Loaded(loaded) = self.definitions else {
            panic!("only a loaded prelude state installs")
        };
        let mut tables = self.tables;
        tables.bindings.adopt_saved(loaded.bindings);
        tables.table.adopt_saved(loaded.names);
        tables
    }

    #[cfg(test)]
    pub(super) fn installed_bindings(self) -> crate::core::BindingArena {
        self.installed_tables().bindings
    }

    #[cfg(test)]
    pub(super) fn tables(&self) -> &ExpansionTables {
        &self.tables
    }

    pub fn save(&self) -> Vec<u8> {
        let PreludeDefinitions::Built(tables) = &self.definitions else {
            panic!("only a built prelude state saves")
        };
        bincode::serialize(&(self, tables)).expect("every saved table serializes")
    }

    pub fn load(bytes: &'static [u8]) -> Result<Self, UnreadablePrelude> {
        let (state, tables): (Self, LoadedTables) =
            bincode::deserialize(bytes).map_err(UnreadablePrelude)?;
        Ok(Self {
            definitions: PreludeDefinitions::Loaded(tables),
            ..state
        })
    }

    pub fn module_note(&self, path: &ModulePath, key: &str) -> Option<&[Form]> {
        let PreludeDefinitions::Loaded(tables) = &self.definitions else {
            panic!("only a loaded prelude state holds readable notes")
        };
        let position = self
            .modules
            .iter()
            .position(|record| record.path() == path)?;
        let module = ModuleId::new(self.prelude.index() + position);
        tables.notes.of(module, key).map(Vec::as_slice)
    }

    pub(super) fn install(self, expander: &mut Expander<'_>) -> ModuleId {
        if !expander.holds_prelude_sources(self.first_source, &self.modules) {
            self.add_sources(expander);
        }
        for record in self.modules {
            expander.modules.adopt(record);
        }
        let PreludeDefinitions::Loaded(tables) = self.definitions else {
            panic!("a built prelude state installs only after a save and a load")
        };
        expander.saved_definitions = tables.definitions;
        expander.saved_macros = tables.macros;
        expander.saved_globals = tables.globals;
        expander.tables = self.tables;
        expander.tables.module_notes.adopt(tables.notes);
        expander.tables.bindings.adopt_saved(tables.bindings);
        expander.tables.table.adopt_saved(tables.names);
        expander.tables.macro_origins = tables.origins.into_macro_origins();
        expander
            .tables
            .restore_lowered_globals(&self.lowered_globals);
        self.prelude
    }
}

impl Expander<'_> {
    fn capture_prelude(mut self, entry: SourceId) -> PreludeState {
        let placeholder =
            self.register_source_module(ModulePath::new(PreludeState::PLACEHOLDER_ENTRY));
        let first_source = self.sources.next_id();
        let first_scope = self.tables.scopes.next_scope();
        let prelude = self
            .load_module(
                &ModulePath::new(ModuleSources::PRELUDE),
                Span::new(entry, 0..0),
            )
            .expect("the prelude loads");
        self.assert_no_problems();
        self.tables.string_view = self.tables.find_string_view();
        self.tables.forget_locals();
        let lowered_globals = self.tables.take_lowered_globals();
        let saved_tables = self.tables.save_definitions();
        let macro_tables = ValueTables::of(self.tables.macros.take_ready());
        let global_tables = ValueTables::of(self.tables.globals.take_values());
        let binding_tables = self.tables.bindings.take_saved();
        let prelude_scopes = first_scope..self.tables.scopes.next_scope();
        let name_tables = self.tables.table.take_saved(prelude_scopes);
        let origin_tables =
            OriginTables::of_own_layers(std::mem::take(&mut self.tables.macro_origins));
        let note_tables = self.tables.module_notes.take_saved();
        PreludeState {
            first_source,
            prelude,
            modules: self.modules.into_records_after(placeholder),
            tables: self.tables,
            lowered_globals,
            definitions: PreludeDefinitions::Built(BuiltTables {
                bindings: binding_tables,
                names: name_tables,
                origins: origin_tables,
                definitions: saved_tables,
                macros: macro_tables,
                globals: global_tables,
                notes: note_tables,
            }),
        }
    }

    fn assert_no_problems(&mut self) {
        let mut problems = self.take_diagnostics();
        problems.extend(self.problems.take_warnings());
        assert!(
            problems.is_empty(),
            "the prelude expands without problems, but it reports:\n{}",
            DiagnosticRenderer::plain(self.sources).render_all(&problems)
        );
    }

    fn add_prelude_source(&mut self, path: &ModulePath) {
        if let Some(ModuleSource::Text { name, text }) = self.module_sources.find(path) {
            self.sources.add(name, text);
        }
    }

    fn holds_prelude_sources(&self, first_source: SourceId, modules: &[ModuleRecord]) -> bool {
        if self.sources.next_id().index() <= first_source.index() {
            return false;
        }
        let held = self.sources.file(first_source).name();
        modules
            .iter()
            .find_map(|record| match self.module_sources.find(record.path()) {
                Some(ModuleSource::Text { name, .. }) => Some(name),
                _ => None,
            })
            .is_some_and(|name| name == held)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use rustc_hash::FxHashMap;

    use super::*;
    use crate::core::{
        BindingArena, BindingKind, ExprKind, Item, ItemKind, KernelConstants, KernelForm,
        KernelType, Leaf, Literal, Namespace,
    };

    struct UserProgram {
        name: &'static str,
        text: &'static str,
    }

    impl UserProgram {
        const GARBLED_STATE: &[u8] = &[1, 2, 3];
        const MACROS_AND_NAMES: Self = Self {
            name: "user",
            text: "\
(defmacro define-caller (name function argument)
  (template (def (unquote name) (fn () ((unquote function) (unquote argument))))))
(def map (fn (x) (+ x 1)))
(define-caller call-it map 3)
(defn main () (println \"{}\" (call-it)))",
        };
        const EMPTY: Self = Self {
            name: "user",
            text: "",
        };
        const NAMED_PRELUDE: Self = Self {
            name: "prelude",
            text: "(def answer 42)",
        };

        fn built_entry(&self) -> String {
            let (mut sources, entry) = self.sources();
            self.entry_module(Expander::new(&mut sources, Self::module_sources()), entry)
        }

        fn loaded_entry(&self) -> String {
            let (mut sources, entry) = self.sources();
            let expander =
                Expander::with_prelude(&mut sources, Self::module_sources(), Self::loaded_state());
            self.entry_module(expander, entry)
        }

        fn sources(&self) -> (SourceMap, SourceId) {
            let mut sources = SourceMap::default();
            let entry = sources.add(self.name, self.text);
            (sources, entry)
        }

        fn entry_module(&self, expander: Expander<'_>, entry: SourceId) -> String {
            let program = expander
                .expand(ModulePath::new(self.name), entry)
                .unwrap_or_else(|_| panic!("the user program expands"));
            format!("{:?}", program.entry_module())
        }

        fn decodes_no_prelude_note(&self) -> bool {
            let (mut sources, entry) = self.sources();
            let mut expander =
                Expander::with_prelude(&mut sources, Self::module_sources(), Self::loaded_state());
            expander.load_source_module(ModulePath::new(self.name), &[entry]);
            expander.tables.module_notes.decoded_no_saved_note()
        }

        fn loaded_state() -> PreludeState {
            PreludeState::load(PreludeState::build().save().leak())
                .expect("the saved prelude loads")
        }

        fn module_sources() -> ModuleSources {
            ModuleSources::new(PathBuf::new(), PathBuf::new())
        }
    }

    struct PreludePages {
        lookups: FxHashMap<String, String>,
    }

    impl PreludePages {
        const ENTRY: &str = "kernel-pages";
        const DOCUMENTED_MODULES: [&str; 18] = [
            "prelude/number",
            "prelude/logic",
            "prelude/text",
            "prelude/io",
            "prelude/vec",
            "prelude/iterator",
            "prelude/sequence",
            "prelude/option",
            "prelude/result",
            "prelude/types",
            "prelude/pair",
            "prelude/ownership",
            "prelude/map",
            "prelude/assoc",
            "prelude/place",
            "prelude/struct",
            "prelude/syntax",
            "prelude/definers",
        ];
        const ALTERNATIVE: &str = " or ";
        const OPTIONAL_PART_END: &str = ")?";
        const OPTIONAL_PART_START: &str = " (";
        const UNDOCUMENTED_TERM: &str = "undocumented-term";
        const UNDOCUMENTED_BODY_NAME: &str = "undocumented-body";

        fn of_saved_prelude(names: &[&str]) -> Self {
            Self::of_entry(Self::lookups(names))
        }

        fn of_module_exports(module: &str, names: &[&str]) -> Self {
            let import = format!("(use {module} {})\n", names.join(" "));
            Self::of_entry(import + &Self::lookups(names))
        }

        fn of_entry(text: String) -> Self {
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::ENTRY, text);
            let expander = Expander::with_prelude(
                &mut sources,
                UserProgram::module_sources(),
                UserProgram::loaded_state(),
            );
            let program = expander
                .expand(ModulePath::new(Self::ENTRY), entry)
                .unwrap_or_else(|_| panic!("the kernel lookups expand"));
            let lookups = program
                .entry_module()
                .items
                .iter()
                .filter_map(|item| Self::string_definition(&program.bindings, item))
                .collect();
            Self { lookups }
        }

        fn names_the_prelude_documents_by_type() -> Vec<(&'static str, &'static str)> {
            let state = UserProgram::loaded_state();
            let prelude = Self::module(&state, ModuleSources::PRELUDE);
            KernelType::all()
                .map(|kernel_type| (kernel_type.name(), kernel_type.shape()))
                .filter(|(name, _)| prelude.exported(name, Namespace::Value).is_none())
                .chain(KernelConstants::BOOLEANS.map(|(name, _)| (name, name)))
                .collect()
        }

        fn module<'a>(state: &'a PreludeState, path: &str) -> &'a ModuleRecord {
            let path = ModulePath::new(path);
            state
                .modules
                .iter()
                .find(|record| *record.path() == path)
                .expect("the saved state holds the module")
        }

        fn exported_names(module: &str) -> Vec<String> {
            let state = UserProgram::loaded_state();
            let exports = Self::module(&state, module).exports();
            let names: BTreeSet<String> = exports.map(|(name, _, _)| name.to_string()).collect();
            names.into_iter().collect()
        }

        fn lookups(names: &[&str]) -> String {
            let term_lookups: String = names
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    format!(
                        "(def {} (doc-header {name}))\n(def {} (doc-body {name}))\n",
                        Self::header_name(index),
                        Self::body_name(index),
                    )
                })
                .collect();
            format!("{}{term_lookups}", Self::undocumented_lookup())
        }

        fn undocumented_lookup() -> String {
            let term = Self::UNDOCUMENTED_TERM;
            let name = Self::UNDOCUMENTED_BODY_NAME;
            format!("(def {term} 0)\n(def {name} (doc-body {term}))\n")
        }

        fn header_name(index: usize) -> String {
            format!("header-{index}")
        }

        fn body_name(index: usize) -> String {
            format!("body-{index}")
        }

        fn string_definition(bindings: &BindingArena, item: &Item) -> Option<(String, String)> {
            let ItemKind::Def(binding, value) = &item.kind else {
                return None;
            };
            let ExprKind::Leaf(Leaf::Literal(Literal::String(text))) = value.kind() else {
                return None;
            };
            Some((bindings.get(*binding).name().to_string(), text.to_string()))
        }

        fn header(&self, index: usize) -> Option<&str> {
            self.lookups
                .get(&Self::header_name(index))
                .map(String::as_str)
        }

        fn documents(&self, index: usize) -> bool {
            let undocumented_body = &self.lookups[Self::UNDOCUMENTED_BODY_NAME];
            self.lookups
                .get(&Self::body_name(index))
                .is_some_and(|body| body != undocumented_body)
        }

        fn header_of_shape(shape: &str) -> String {
            let first_alternative = shape
                .split(Self::ALTERNATIVE)
                .next()
                .expect("a shape has a first alternative");
            Self::without_optional_parts(first_alternative)
        }

        fn without_optional_parts(shape: &str) -> String {
            let mut header = shape.to_string();
            while let Some(end) = header.find(Self::OPTIONAL_PART_END) {
                let start = header[..end]
                    .rfind(Self::OPTIONAL_PART_START)
                    .expect("an optional part opens after a space");
                header.replace_range(start..end + Self::OPTIONAL_PART_END.len(), "");
            }
            header
        }
    }

    #[test]
    fn each_kernel_form_page_shows_its_shape_as_header() {
        let names: Vec<&str> = KernelForm::all().map(KernelForm::name).collect();
        let pages = PreludePages::of_saved_prelude(&names);
        for (index, form) in KernelForm::all().enumerate() {
            let header = PreludePages::header_of_shape(form.shape());
            assert_eq!(
                pages.header(index),
                Some(header.as_str()),
                "{}",
                form.name()
            );
        }
    }

    #[test]
    fn an_empty_program_decodes_no_prelude_note() {
        assert!(UserProgram::EMPTY.decodes_no_prelude_note());
    }

    #[test]
    fn each_kernel_type_and_constant_has_a_page_headed_by_its_shape() {
        let entries = PreludePages::names_the_prelude_documents_by_type();
        let names: Vec<&str> = entries.iter().map(|(name, _)| *name).collect();
        let pages = PreludePages::of_saved_prelude(&names);
        for (index, (name, shape)) in entries.iter().enumerate() {
            assert_eq!(pages.header(index), Some(*shape), "{name}");
            assert!(pages.documents(index), "{name} has no docstring");
        }
    }

    #[test]
    fn each_export_of_a_documented_module_has_a_docstring() {
        for module in PreludePages::DOCUMENTED_MODULES {
            let names = PreludePages::exported_names(module);
            let lookups: Vec<&str> = names.iter().map(String::as_str).collect();
            let pages = PreludePages::of_module_exports(module, &lookups);
            for (index, name) in names.iter().enumerate() {
                assert!(pages.documents(index), "{module} {name} has no docstring");
            }
        }
    }

    #[test]
    fn user_ids_continue_after_a_loaded_prelude() {
        let program = UserProgram::MACROS_AND_NAMES;
        assert_eq!(program.loaded_entry(), program.built_entry());
    }

    #[test]
    fn an_entry_named_prelude_expands_alike_on_a_loaded_state() {
        let program = UserProgram::NAMED_PRELUDE;
        assert_eq!(program.loaded_entry(), program.built_entry());
    }

    #[test]
    fn a_saved_state_holds_the_string_view() {
        let state = UserProgram::loaded_state();
        assert!(state.tables.string_view.is_some());
    }

    #[test]
    #[should_panic(expected = "installs only after a save and a load")]
    fn a_built_state_does_not_install() {
        let (mut sources, entry) = UserProgram::EMPTY.sources();
        let expander = Expander::with_prelude(
            &mut sources,
            UserProgram::module_sources(),
            PreludeState::build(),
        );
        UserProgram::EMPTY.entry_module(expander, entry);
    }

    #[test]
    #[should_panic(expected = "only a built prelude state saves")]
    fn a_loaded_state_does_not_save() {
        UserProgram::loaded_state().save();
    }

    #[test]
    fn a_saved_state_resolves_no_prelude_local() {
        let tables = UserProgram::loaded_state().installed_tables();
        let mut locals = tables
            .bindings
            .ids()
            .filter(|binding| matches!(tables.bindings.get(*binding).kind(), BindingKind::Local));
        let local = locals.next().expect("the prelude binds locals");
        assert!(!tables.table.holds(local));
    }

    #[test]
    fn a_loaded_state_decodes_a_binding_only_when_it_is_read() {
        let bindings = UserProgram::loaded_state().installed_bindings();
        let mut ids = bindings.ids();
        let (first, second) = (
            ids.next().expect("a binding"),
            ids.next().expect("a binding"),
        );
        assert!(!bindings.is_decoded(first));
        bindings.get(first);
        assert!(bindings.is_decoded(first));
        assert!(!bindings.is_decoded(second));
    }

    #[test]
    fn a_cloned_arena_shares_the_decoded_prelude_bindings() {
        let bindings = UserProgram::loaded_state().installed_bindings();
        let first = bindings.ids().next().expect("a binding");
        let copy = bindings.clone();
        copy.get(first);
        assert!(bindings.is_decoded(first));
    }

    #[test]
    fn a_loaded_state_reads_a_note_of_a_prelude_module() {
        let state = UserProgram::loaded_state();
        let module = ModulePath::new(ModuleSources::DOCSTRINGS);
        assert!(state.module_note(&module, "docstring-syntax").is_some());
        assert!(state.module_note(&module, "no-such-note").is_none());
    }

    #[test]
    fn a_saved_state_keeps_no_unsaved_note() {
        let state = PreludeState::build();
        assert!(state.tables().module_notes.holds_no_unsaved_note());
    }

    #[test]
    fn a_saved_state_keeps_its_evaluated_globals_only_in_the_saved_bytes() {
        let state = PreludeState::build();
        assert!(state.tables().globals.holds_no_value());
    }

    #[test]
    fn a_saved_state_keeps_its_macro_origins_only_in_the_saved_bytes() {
        let state = PreludeState::build();
        let origins = &state.tables().macro_origins;
        assert!(origins.calls.holds_no_own_origin());
        assert!(origins.introduced.holds_no_own_name());
    }

    #[test]
    fn unreadable_bytes_report_an_unreadable_prelude() {
        let Err(problem) = PreludeState::load(UserProgram::GARBLED_STATE) else {
            panic!("garbled bytes do not load");
        };
        assert!(
            problem
                .to_string()
                .starts_with("the saved prelude state is unreadable: ")
        );
    }

    #[test]
    fn two_saves_of_the_prelude_state_match() {
        let first_save = PreludeState::build().save();
        let second_save = PreludeState::build().save();
        assert!(first_save == second_save, "the two saves differ");
    }

    #[test]
    fn a_second_install_into_the_same_sources_reuses_the_prelude_sources() {
        let program = UserProgram::MACROS_AND_NAMES;
        let (mut sources, entry) = program.sources();
        let first = Expander::with_prelude(
            &mut sources,
            UserProgram::module_sources(),
            UserProgram::loaded_state(),
        );
        let first_entry = program.entry_module(first, entry);
        let installed = sources.next_id();
        let second = Expander::with_prelude(
            &mut sources,
            UserProgram::module_sources(),
            UserProgram::loaded_state(),
        );
        assert_eq!(program.entry_module(second, entry), first_entry);
        assert_eq!(sources.next_id(), installed);
    }

    #[test]
    #[should_panic(expected = "the entry is the only source before the prelude")]
    fn a_source_before_the_entry_stops_the_install() {
        let program = UserProgram::MACROS_AND_NAMES;
        let mut sources = SourceMap::default();
        sources.add("earlier", "");
        let entry = sources.add(program.name, program.text);
        let expander = Expander::with_prelude(
            &mut sources,
            UserProgram::module_sources(),
            UserProgram::loaded_state(),
        );
        program.entry_module(expander, entry);
    }
}
