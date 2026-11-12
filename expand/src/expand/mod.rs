mod binder_form;
mod bindings;
mod budget_arena;
mod classified_form;
mod compile_time_check;
mod context;
mod declared_trait;
mod defining_call;
mod docstring_notes;
mod documented_terms;
mod duplicate_definitions;
mod enclosing_index;
mod evaluated_forms;
mod expansion_limit;
mod expression;
mod failed_definition;
mod field_keyword;
mod form_budget;
mod generated_entry;
mod globals;
mod host;
mod input_terms;
mod introduced;
mod macro_call;
mod macro_function;
mod macro_origin;
mod macro_origins;
#[cfg(test)]
mod macro_value_limits;
mod misplaced_definition;
mod module_body;
mod module_loading;
mod module_notes;
mod module_survey;
mod names;
mod noted_term;
mod noted_terms;
mod notes_module;
mod prelude_state;
mod problem_mark;
mod problem_report;
mod problems;
mod program_entry;
mod recorded_inputs;
mod recovered_errors;
mod recur_check;
mod rest_of_body;
mod result_printer;
mod returned_arguments;
mod rust_clauses;
mod rust_type;
mod saved_definitions;
mod saved_index;
mod saved_notes;
mod saved_values;
mod session;
mod set_aside_input;
mod set_aside_problems;
mod shape;
mod sourced_items;
mod spliced_forms;
mod string_view;
mod structural_path_claim;
mod structural_path_claims;
mod structural_traits;
mod top_level_form;
mod try_check;
mod type_expr;
mod type_note;
mod type_parameters;

use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{
    BindingArena, BindingId, BindingKind, CoreModule, ModuleId, ModulePath, Namespace, References,
};
use crate::diagnostic::Diagnostics;
use crate::interpret::{Closure, Interpreter, Value};
use crate::modules::{ModuleRegistry, ModuleSources};
use crate::reader::Form;
use crate::scope::{BindingTable, Phase, Scope, ScopeSupply, Scopes};
use crate::source::{SourceId, SourceMap, Span};
use bindings::Bindings;
use budget_arena::BudgetArena;
pub use classified_form::ClassifiedForm;
use docstring_notes::DocstringNotes;
use documented_terms::DocumentedTerms;
use duplicate_definitions::DuplicateDefinitions;
use evaluated_forms::EvaluatedForms;
pub use generated_entry::GeneratedEntry;
use globals::{GlobalTable, LoweredGlobals};
pub use input_terms::InputTerms;
use macro_call::MacroTable;
pub use macro_origin::MacroOrigin;
pub use macro_origins::MacroOrigins;
use module_notes::{ModuleNotes, NotesCheckpoint};
pub use module_survey::ModuleSurvey;
use noted_term::NotedTerm;
use noted_terms::NotedTerms;
use notes_module::NotesModule;
pub use prelude_state::{PreludeState, UnreadablePrelude};
use problem_report::ProblemReport;
pub use problems::ExpandProblems;
pub use program_entry::ProgramEntry;
use recorded_inputs::RecordedInputs;
pub use recorded_inputs::{BoundName, LoadedModule};
use recovered_errors::RecoveredErrors;
pub use result_printer::{GeneratedFunction, ResultPrinter};
pub use saved_definitions::PendingDefinitions;
use saved_definitions::{DefinitionSaver, SavedDefinitions, SavedTables};
use saved_values::SavedValues;
pub use session::ExpansionSession;
pub use set_aside_input::SetAsideInput;
pub use set_aside_problems::SetAsideProblems;
use string_view::StringViewSearch;
use structural_path_claims::StructuralPathClaims;
use structural_traits::StructuralTraits;
use type_note::TypeNote;

pub struct Program {
    pub modules: Vec<CoreModule>,
    pub entry: ProgramEntry,
    pub bindings: BindingArena,
    pub scopes: ScopeSupply,
    pub macro_origins: Rc<MacroOrigins>,
    pub macro_helpers: FxHashSet<BindingId>,
    pub string_view: Option<BindingId>,
    pub checked_roots: Vec<BindingId>,
    pub pending_definitions: PendingDefinitions,
    pub warnings: Diagnostics,
    pub expansion: ExpansionWork,
}

#[derive(Clone, Copy, Default)]
pub struct ExpansionWork {
    pub lowered_forms: usize,
}

impl Program {
    pub(super) const WRITTEN_ENTRY_FUNCTION: &str = "main";

    pub fn entry_module(&self) -> &CoreModule {
        self.modules
            .iter()
            .find(|module| module.id == self.entry.module)
            .expect("the entry module is expanded")
    }

    pub fn prelude_macro_helpers(&self) -> FxHashSet<BindingId> {
        let prelude: FxHashSet<ModuleId> = self
            .modules
            .iter()
            .filter(|module| ModuleSources::is_prelude(&module.path))
            .map(|module| module.id)
            .collect();
        let is_prelude_global = |binding: BindingId| {
            matches!(self.bindings.get(binding).kind(),
                BindingKind::Global(module) if prelude.contains(&module))
        };
        self.macro_helpers
            .iter()
            .copied()
            .filter(|binding| self.bindings.is_saved(*binding) || is_prelude_global(*binding))
            .collect()
    }

    pub fn check_also(&mut self, binding: BindingId) {
        self.checked_roots.push(binding);
        self.pending_definitions
            .restore(&mut self.modules, vec![binding]);
    }

    fn restore_reached(&mut self) {
        let roots = self
            .modules
            .iter()
            .flat_map(CoreModule::definitions)
            .flat_map(|(_, value)| References::of(value, &self.bindings).into_globals())
            .collect();
        self.pending_definitions.restore(&mut self.modules, roots);
    }

    fn written_entry_function(&self) -> Option<BindingId> {
        self.entry_module()
            .definitions()
            .map(|(binding, _)| binding)
            .find(|binding| self.bindings.get(*binding).name() == Self::WRITTEN_ENTRY_FUNCTION)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct TraitMethods {
    pub rust_name: Rc<str>,
    pub self_variable: crate::core::BindingId,
    pub associated: Vec<(crate::core::BindingId, Rc<str>)>,
    pub methods: Vec<(crate::core::BindingId, crate::core::TypeExpr)>,
    pub defaults: rustc_hash::FxHashSet<crate::core::BindingId>,
}

pub struct Expander<'s> {
    sources: &'s mut SourceMap,
    module_sources: ModuleSources,
    modules: ModuleRegistry,
    tables: ExpansionTables,
    current_module: Option<ModuleId>,
    problems: ProblemReport,
    reported_hidden_modules: Vec<std::path::PathBuf>,
    library_modules: Vec<(ModulePath, Rc<str>)>,
    phase: Phase,
    expansion_depth: usize,
    nesting_depth: usize,
    form_budgets: BudgetArena,
    recovered: RecoveredErrors,
    type_note: TypeNote,
    saved_prelude: Option<PreludeState>,
    saved_definitions: SavedDefinitions,
    saved_macros: SavedValues<Closure>,
    saved_globals: SavedValues<Value>,
    expansion: ExpansionWork,
    evaluated_forms: EvaluatedForms,
    recorded_inputs: RecordedInputs,
    declared_definitions: Vec<failed_definition::DeclaredName>,
    failed_body_definitions: Vec<BindingId>,
    introductions: Vec<Scope>,
    duplicate_definitions: DuplicateDefinitions,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct ExpansionTables {
    scopes: Scopes,
    table: BindingTable,
    bindings: BindingArena,
    macros: MacroTable,
    macro_origins: MacroOrigins,
    global_macros: Vec<(crate::core::BindingId, Form)>,
    trait_paths: FxHashMap<crate::core::BindingId, Rc<str>>,
    structural_path_claims: StructuralPathClaims,
    enum_variants: FxHashMap<crate::core::BindingId, Vec<crate::core::BindingId>>,
    variant_fields: FxHashMap<crate::core::BindingId, Vec<Rc<str>>>,
    trait_methods: FxHashMap<crate::core::BindingId, TraitMethods>,
    globals: GlobalTable,
    module_notes: ModuleNotes,
    expanded: FxHashMap<ModuleId, CoreModule>,
    order: Vec<ModuleId>,
    string_view: Option<BindingId>,
}

impl ExpansionTables {
    fn forget_locals(&mut self) {
        let bindings = &self.bindings;
        self.table
            .forget(|binding| matches!(bindings.get(binding).kind(), BindingKind::Local));
    }

    fn find_string_view(&self) -> Option<BindingId> {
        let modules = self
            .order
            .iter()
            .filter_map(|id| self.expanded.get(id))
            .filter(|module| ModuleSources::is_embedded_prelude(&module.path));
        StringViewSearch::in_modules(modules)
    }

    fn save_definitions(&mut self) -> SavedTables {
        let mut saver = DefinitionSaver::new(&self.bindings, self.string_view);
        let modules = self.expanded.values_mut();
        for module in modules.filter(|module| ModuleSources::is_embedded_prelude(&module.path)) {
            saver.take_from(module);
        }
        saver.finish()
    }

    fn take_lowered_globals(&mut self) -> LoweredGlobals {
        let definitions = self.expanded.values().flat_map(CoreModule::definitions);
        self.globals.take_lowered(definitions)
    }

    fn restore_lowered_globals(&mut self, lowered: &LoweredGlobals) {
        let definitions = self.expanded.values().flat_map(CoreModule::definitions);
        self.globals.restore_lowered(lowered, definitions);
    }
}

impl<'s> Expander<'s> {
    pub fn new(sources: &'s mut SourceMap, module_sources: ModuleSources) -> Self {
        Self {
            sources,
            module_sources,
            modules: ModuleRegistry::default(),
            tables: ExpansionTables::default(),
            current_module: None,
            problems: ProblemReport::default(),
            reported_hidden_modules: Vec::new(),
            library_modules: Vec::new(),
            phase: Phase::Runtime,
            expansion_depth: 0,
            nesting_depth: 0,
            form_budgets: BudgetArena::default(),
            recovered: RecoveredErrors::default(),
            type_note: TypeNote::default(),
            saved_prelude: None,
            saved_definitions: SavedDefinitions::default(),
            saved_macros: SavedValues::default(),
            saved_globals: SavedValues::default(),
            expansion: ExpansionWork::default(),
            evaluated_forms: EvaluatedForms::default(),
            recorded_inputs: RecordedInputs::default(),
            declared_definitions: Vec::new(),
            failed_body_definitions: Vec::new(),
            introductions: Vec::new(),
            duplicate_definitions: DuplicateDefinitions::default(),
        }
    }

    pub fn evaluating(self, spans: impl IntoIterator<Item = Span>) -> Self {
        Self {
            evaluated_forms: EvaluatedForms::new(spans),
            ..self
        }
    }

    #[cfg(test)]
    pub(crate) fn take_diagnostics_with_code(
        &mut self,
        code: &crate::diagnostic::DiagnosticCode,
    ) -> Vec<crate::diagnostic::Diagnostic> {
        self.take_diagnostics()
            .in_source_order()
            .into_iter()
            .filter(|diagnostic| diagnostic.code() == Some(code))
            .cloned()
            .collect()
    }

    pub fn with_prelude(
        sources: &'s mut SourceMap,
        module_sources: ModuleSources,
        prelude: PreludeState,
    ) -> Self {
        Self {
            saved_prelude: Some(prelude),
            ..Self::new(sources, module_sources)
        }
    }

    #[inline(never)]
    pub fn expand(mut self, path: ModulePath, entry: SourceId) -> Result<Program, Diagnostics> {
        let entry = self.load_source_module(path, &[entry]);
        self.into_program(entry)
    }

    fn into_program(mut self, entry: ModuleId) -> Result<Program, Diagnostics> {
        let diagnostics = self.take_diagnostics();
        self.tables.macro_origins.forget_introduced_names();
        let string_view = self
            .tables
            .string_view
            .or_else(|| self.tables.find_string_view());
        let mut program = Program {
            modules: self.take_modules(),
            entry: ProgramEntry {
                module: entry,
                function: None,
            },
            bindings: self.tables.bindings,
            scopes: self.tables.scopes.into_supply(),
            macro_helpers: self.tables.globals.macro_helpers(),
            string_view,
            checked_roots: Vec::new(),
            pending_definitions: PendingDefinitions::new(self.saved_definitions),
            macro_origins: Rc::new(self.tables.macro_origins),
            warnings: self.problems.take_warnings(),
            expansion: self.expansion,
        };
        program.restore_reached();
        program.entry.function = program.written_entry_function();
        diagnostics.into_result(program)
    }

    fn take_diagnostics(&mut self) -> Diagnostics {
        self.recovered.hand_over(&mut self.problems);
        self.problems.take_errors()
    }

    fn structural_traits(&mut self) -> StructuralTraits<'_> {
        StructuralTraits::new(
            &self.tables.bindings,
            &mut self.tables.structural_path_claims,
            &mut self.problems,
        )
    }

    fn check_documented_terms(&mut self, module: ModuleId, since: NotesCheckpoint) {
        for undefined in self.documented_terms(module, since).undefined() {
            self.problems.report(undefined);
        }
    }

    fn documented_terms(
        &mut self,
        module: ModuleId,
        since: NotesCheckpoint,
    ) -> DocumentedTerms<'_> {
        let noted = self.noted_terms(module, since);
        DocumentedTerms::new(self.bindings(), &self.duplicate_definitions, noted)
    }

    fn bindings(&self) -> Bindings<'_> {
        Bindings::new(&self.tables.table, &self.tables.bindings, self.phase)
    }

    fn noted_terms(&mut self, module: ModuleId, since: NotesCheckpoint) -> NotedTerms {
        let is_prelude = ModuleSources::is_prelude(self.modules.get(module).path());
        if is_prelude || self.tables.module_notes.checkpoint() == since {
            return NotedTerms::new(module, Vec::new());
        }
        let Some(notes) = self.docstring_notes() else {
            return NotedTerms::new(module, Vec::new());
        };
        let terms = notes
            .keys()
            .into_iter()
            .flat_map(|key| self.tables.module_notes.added_since(since, module, key))
            .collect::<Vec<Form>>()
            .into_iter()
            .filter_map(|note| self.noted_term(notes.noted_term(), note))
            .collect();
        NotedTerms::new(module, terms)
    }

    fn docstring_notes(&mut self) -> Option<DocstringNotes> {
        let notes_module = NotesModule::new(&self.modules, self.bindings()).find()?;
        let key = self.exported_value(notes_module, DocstringNotes::DOCSTRINGS_KEY)?;
        let documented_terms_key =
            self.exported_value(notes_module, DocstringNotes::DOCUMENTED_TERMS_KEY)?;
        let noted_term = self.exported_value(notes_module, DocstringNotes::NOTED_TERM)?;
        DocstringNotes::of(&key, &documented_terms_key, noted_term)
    }

    fn exported_value(&mut self, module: ModuleId, name: &str) -> Option<Value> {
        let binding = self.modules.get(module).exported(name, Namespace::Value)?;
        let span = self.tables.bindings.get(binding).span()?;
        self.evaluate_global(binding, span).ok()
    }

    fn noted_term(&mut self, accessor: &Value, note: Form) -> Option<NotedTerm> {
        let span = note.span();
        let steps = self.form_budgets.current().steps();
        let term = Interpreter::new(self, span, steps)
            .call(accessor, vec![Value::Form(note)], span)
            .ok()?;
        let term = term.as_form()?;
        Some(NotedTerm {
            term: term.as_symbol()?.clone(),
            span: term.span(),
        })
    }

    fn take_modules(&mut self) -> Vec<CoreModule> {
        self.tables
            .order
            .iter()
            .filter_map(|id| self.tables.expanded.remove(id))
            .collect()
    }
}
