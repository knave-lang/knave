use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::Expander;
use super::context::DefinitionContext;
use super::expansion_limit::ExpansionLimit;
use super::form_budget::FormBudget;
use super::problems::ExpandProblems;
use super::returned_arguments::ReturnedArguments;
use super::shape::{Shape, WrittenDefiner};
use crate::core::{BindingId, BindingKind, DefinitionForm, Expr, KernelForm, Lambda, Namespace};
use crate::diagnostic::Diagnostic;
use crate::interpret::{Closure, Env, Interpreter, PassedLimit, Value};
use crate::modules::ModuleSources;
use crate::reader::Form;
use crate::scope::{Phase, Scope};
use crate::source::Span;

#[derive(serde::Serialize, serde::Deserialize)]
enum MacroState {
    Pending(Form),
    Ready(Rc<Closure>),
    Failed,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MacroOrigin {
    pub name: Rc<str>,
    pub defined: Span,
    pub call_site: Span,
    pub through_prelude: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MacroTable {
    states: FxHashMap<BindingId, MacroState>,
}

impl MacroTable {
    pub fn add_pending(&mut self, binding: BindingId, definition: Form) {
        self.states.insert(binding, MacroState::Pending(definition));
    }

    pub fn take_ready(&mut self) -> Vec<(BindingId, Rc<Closure>)> {
        let mut ready = Vec::new();
        for (binding, state) in std::mem::take(&mut self.states) {
            match state {
                MacroState::Ready(closure) => ready.push((binding, closure)),
                unready => {
                    self.states.insert(binding, unready);
                }
            }
        }
        ready.sort_by_key(|(binding, _)| *binding);
        ready
    }

    #[cfg(test)]
    pub fn holds_ready(&self, binding: BindingId) -> bool {
        matches!(self.states.get(&binding), Some(MacroState::Ready(_)))
    }

    fn pending(&self) -> Vec<(BindingId, Form)> {
        let mut pending: Vec<(BindingId, Form)> = self
            .states
            .iter()
            .filter_map(|(binding, state)| match state {
                MacroState::Pending(definition) => Some((*binding, definition.clone())),
                _ => None,
            })
            .collect();
        pending.sort_by_key(|(binding, _)| *binding);
        pending
    }
}

impl Expander<'_> {
    const MAX_EXPANSION_DEPTH: usize = 256;
    const MACRO_PARAMETERS: usize = 1;

    pub(super) fn expand_head(
        &mut self,
        form: Form,
        mut context: Option<&mut DefinitionContext>,
    ) -> Result<Form, Diagnostic> {
        self.expand_head_with(form, |expander, binding, form| {
            expander.apply_macro(binding, form, context.as_deref_mut())
        })
    }

    pub(super) fn expand_top_level_head(
        &mut self,
        form: Form,
        context: &mut DefinitionContext,
    ) -> Result<Form, Diagnostic> {
        self.expand_head_with(form, |expander, binding, form| {
            expander.apply_definer(binding, form, context)
        })
    }

    fn expand_head_with(
        &mut self,
        form: Form,
        mut apply: impl FnMut(&mut Self, BindingId, Form) -> Result<Form, Diagnostic>,
    ) -> Result<Form, Diagnostic> {
        let mut form = form;
        let mut expansions = 0;
        while let Some(binding) = self.macro_head(&form) {
            if expansions == Self::MAX_EXPANSION_DEPTH {
                return Err(ExpandProblems::expansion_too_deep(form.span()));
            }
            form = apply(self, binding, form)?;
            expansions += 1;
        }
        Ok(form)
    }

    pub(super) fn lower_expansion(&mut self, expanded: &Form) -> Result<Expr, Diagnostic> {
        self.expansion_depth += 1;
        let lowered = self.lower_expr(expanded);
        self.expansion_depth -= 1;
        lowered
    }

    pub(super) fn macro_head(&self, form: &Form) -> Option<BindingId> {
        let head = form.as_list()?.first()?.as_symbol()?;
        let binding = self.try_resolve(head, Namespace::Value)?;
        (matches!(self.kind(binding), BindingKind::Macro(_))).then_some(binding)
    }

    pub(super) fn apply_macro(
        &mut self,
        binding: BindingId,
        form: Form,
        context: Option<&mut DefinitionContext>,
    ) -> Result<Form, Diagnostic> {
        let call_site = form.span();
        let expanded = self.run_macro(binding, form, context)?;
        Ok(self.place_expansion(binding, call_site, expanded))
    }

    pub(super) fn value_expansion(
        &mut self,
        binding: BindingId,
        form: Form,
    ) -> Result<Option<Form>, Diagnostic> {
        let call_site = form.span();
        match self.trial_expansion(binding, form) {
            Ok(expanded) => Ok(Some(self.place_expansion(binding, call_site, expanded))),
            Err(failure) if ExpansionLimit::stopped(&failure) => Err(failure),
            Err(_) => Ok(None),
        }
    }

    fn place_expansion(&mut self, binding: BindingId, call_site: Span, mut form: Form) -> Form {
        let origin = self.macro_origin(binding, call_site);
        self.record_introduced_names(&form, &origin);
        form.relocate_outside(call_site);
        self.tables.macro_origins.record(call_site, origin);
        form
    }

    pub(super) fn discarded_expansion(
        &mut self,
        binding: BindingId,
        form: Form,
    ) -> Result<Form, Diagnostic> {
        let checkpoint = self.tables.module_notes.checkpoint();
        let budget = self.form_budgets.current().clone();
        let steps = budget.steps();
        let limit = steps.limit();
        let expansion = self.trial_expansion(binding, form);
        self.tables.module_notes.roll_back(checkpoint);
        *self.form_budgets.current() = budget;
        steps.set_limit(limit);
        expansion
    }

    fn trial_expansion(&mut self, binding: BindingId, form: Form) -> Result<Form, Diagnostic> {
        let diagnostics = self.diagnostics.len();
        let recovered = self.recovered_diagnostics.len();
        let declared = self.declared_definitions.len();
        let budget_before = self.form_budgets.current().clone();
        let result = self.run_macro(binding, form, None);
        self.diagnostics.truncate(diagnostics);
        self.recovered_diagnostics.truncate(recovered);
        self.declared_definitions.truncate(declared);
        if result.is_err() {
            *self.form_budgets.current() = budget_before;
        }
        result
    }

    fn run_macro(
        &mut self,
        binding: BindingId,
        form: Form,
        context: Option<&mut DefinitionContext>,
    ) -> Result<Form, Diagnostic> {
        let checkpoint = self.tables.module_notes.checkpoint();
        let expansion = self.macro_expansion(binding, form, context);
        if expansion.is_err() {
            self.tables.module_notes.roll_back(checkpoint);
        }
        expansion
    }

    fn macro_expansion(
        &mut self,
        binding: BindingId,
        form: Form,
        context: Option<&mut DefinitionContext>,
    ) -> Result<Form, Diagnostic> {
        let call_site = form.span();
        if self.expansion_depth >= Self::MAX_EXPANSION_DEPTH {
            return Err(ExpandProblems::expansion_too_deep(call_site));
        }
        if self.form_budgets.current().is_exhausted() {
            return Err(self.expansion_too_large(binding, call_site));
        }
        let closure = self.macro_closure(binding, call_site)?;
        let (input, introduction) = self.prepare_input(form, context);
        let arguments = ReturnedArguments::of_call(&input);
        self.expansion_depth += 1;
        self.introductions.push(introduction);
        let steps = self.form_budgets.current().steps();
        let result = Interpreter::new(self, call_site, steps).call(
            &Value::Closure(closure),
            vec![Value::Form(input)],
            call_site,
        );
        self.introductions.pop();
        self.expansion_depth -= 1;
        let sources = &*self.sources;
        let output = result.map_err(|failure| {
            let failure = ExpansionLimit::at_user_call(failure, call_site, sources);
            Self::point_at_call(failure, call_site)
        })?;
        let mut form = self.output_form(binding, &output, call_site)?;
        let counted_forms = self.counted_definition_forms(arguments, &form);
        self.introduce_within_budget(binding, &mut form, introduction, counted_forms, call_site)?;
        Ok(form)
    }

    fn counted_definition_forms(&self, arguments: ReturnedArguments, expansion: &Form) -> usize {
        if self.is_kernel_form(expansion, KernelForm::Definition(DefinitionForm::Def)) {
            arguments.into_form_count(expansion)
        } else {
            0
        }
    }

    fn output_form(
        &mut self,
        binding: BindingId,
        output: &Value,
        call_site: Span,
    ) -> Result<Form, Diagnostic> {
        if !output.is_code() {
            return Err(ExpandProblems::macro_result(output.describe(), call_site));
        }
        match self.form_budgets.current().admitted_form(output) {
            Ok(form) => Ok(form),
            Err(PassedLimit::FormCount) => Err(self.exhaust_budget(binding, call_site)),
            Err(PassedLimit::Nesting) => Err(self.output_nests_too_deep(binding, call_site)),
        }
    }

    fn introduce_within_budget(
        &mut self,
        binding: BindingId,
        form: &mut Form,
        introduction: Scope,
        counted_forms: usize,
        call_site: Span,
    ) -> Result<(), Diagnostic> {
        if self
            .form_budgets
            .current()
            .flip_scope(form, introduction, counted_forms)
        {
            return Ok(());
        }
        Err(self.exhaust_budget(binding, call_site))
    }

    fn exhaust_budget(&mut self, binding: BindingId, call_site: Span) -> Diagnostic {
        self.form_budgets.current().exhaust();
        self.expansion_too_large(binding, call_site)
    }

    fn output_nests_too_deep(&self, binding: BindingId, call_site: Span) -> Diagnostic {
        let name = self.tables.bindings.get(binding).name();
        ExpandProblems::expansion_nests_too_deep(call_site, Form::NESTING_LIMIT, Some(name))
    }

    fn expansion_too_large(&self, binding: BindingId, call_site: Span) -> Diagnostic {
        let name = self.tables.bindings.get(binding).name();
        ExpandProblems::expansion_too_large(name, FormBudget::EXPANSION_LIMIT, call_site)
    }

    fn macro_origin(&self, binding: BindingId, call_site: Span) -> MacroOrigin {
        let info = self.tables.bindings.get(binding);
        MacroOrigin {
            name: info.shared_name(),
            defined: info.span().unwrap_or(call_site),
            call_site,
            through_prelude: self.is_prelude_macro(binding),
        }
    }

    fn prepare_input(
        &mut self,
        mut form: Form,
        context: Option<&mut DefinitionContext>,
    ) -> (Form, Scope) {
        let introduction = self.tables.scopes.fresh();
        self.tables.introduction_scopes.insert(introduction);
        form.flip_scope(introduction);
        if let Some(context) = context {
            let use_site = self.tables.scopes.fresh();
            self.tables.use_site_scopes.insert(use_site);
            form.add_scope(use_site);
            context.record_fresh_use_site(use_site);
        }
        (form, introduction)
    }

    fn is_prelude_macro(&self, binding: BindingId) -> bool {
        match self.kind(binding) {
            BindingKind::Macro(module) => {
                ModuleSources::is_prelude(self.modules.get(module).path())
            }
            _ => false,
        }
    }

    pub(super) fn macro_origin_at(&self, span: Span) -> Option<MacroOrigin> {
        self.tables.macro_origins.enclosing(span)
    }

    pub(super) fn definition_shape<'f>(
        &self,
        definition: DefinitionForm,
        form: &'f Form,
    ) -> Shape<'f> {
        let definer = self
            .tables
            .macro_origins
            .get(form.span())
            .map(|origin| WrittenDefiner {
                name: origin.name,
                through_prelude: origin.through_prelude,
            });
        Shape::expanded_from(KernelForm::Definition(definition), form, definer)
    }

    pub(super) fn introduced_name(&self, call_site: Span, name: &str) -> Option<Span> {
        self.tables.macro_origins.introduced(call_site, name)
    }

    fn record_introduced_names(&mut self, form: &Form, origin: &MacroOrigin) {
        if let Some(symbol) = form.as_symbol()
            && !origin.call_site.contains(form.span())
        {
            self.tables.macro_origins.record_introduced(
                origin.call_site,
                symbol.name(),
                form.span(),
            );
        }
        if let Some(items) = form.as_list() {
            for item in items {
                self.record_introduced_names(item, origin);
            }
        }
    }

    fn point_at_call(diagnostic: Diagnostic, call_site: Span) -> Diagnostic {
        let already_labels_the_call = diagnostic.labels().any(|label| label.span() == call_site);
        if call_site.contains(diagnostic.primary().span())
            || already_labels_the_call
            || ExpansionLimit::ran_too_many_steps(&diagnostic)
        {
            diagnostic
        } else {
            diagnostic.with_label(ExpandProblems::macro_call(call_site))
        }
    }

    fn macro_closure(&mut self, binding: BindingId, span: Span) -> Result<Rc<Closure>, Diagnostic> {
        let name = self.tables.bindings.get(binding).name().to_string();
        match self.tables.macros.states.remove(&binding) {
            Some(MacroState::Ready(closure)) => {
                self.tables
                    .macros
                    .states
                    .insert(binding, MacroState::Ready(Rc::clone(&closure)));
                Ok(closure)
            }
            Some(MacroState::Pending(definition)) => {
                self.within_top_level_form(|expander| expander.compile_macro(binding, &definition))
            }
            Some(MacroState::Failed) => {
                self.tables
                    .macros
                    .states
                    .insert(binding, MacroState::Failed);
                Err(ExpandProblems::broken_macro(&name, span))
            }
            None => self
                .decode_saved_macro(binding)
                .ok_or_else(|| ExpandProblems::compile_time_cycle(&name, span)),
        }
    }

    fn decode_saved_macro(&mut self, binding: BindingId) -> Option<Rc<Closure>> {
        let closure = Rc::new(self.saved_macros.value_of(binding)?);
        let ready = MacroState::Ready(Rc::clone(&closure));
        self.tables.macros.states.insert(binding, ready);
        Some(closure)
    }

    pub(super) fn compile_pending_macros(&mut self) {
        for (binding, definition) in self.tables.macros.pending() {
            let compiled =
                self.within_top_level_form(|expander| expander.compile_macro(binding, &definition));
            if let Err(diagnostic) = compiled {
                self.diagnostics.push(diagnostic);
            }
        }
    }

    fn compile_macro(
        &mut self,
        binding: BindingId,
        definition: &Form,
    ) -> Result<Rc<Closure>, Diagnostic> {
        let compiled = self.lower_macro_lambda(definition).and_then(|lambda| {
            self.check_compile_time(&lambda)?;
            Ok(Rc::new(Closure {
                lambda: Rc::new(lambda),
                env: Env::default(),
            }))
        });
        let state = match &compiled {
            Ok(closure) => MacroState::Ready(Rc::clone(closure)),
            Err(_) => MacroState::Failed,
        };
        self.tables.macros.states.insert(binding, state);
        compiled
    }

    fn lower_macro_lambda(&mut self, definition: &Form) -> Result<Lambda, Diagnostic> {
        let shape = Shape::new(KernelForm::Macro, definition);
        let items = shape.at_least(2)?;
        if shape.list(&items[1])?.len() != Self::MACRO_PARAMETERS {
            return Err(shape.malformed(&items[1]));
        }
        let outer_phase = std::mem::replace(&mut self.phase, Phase::CompileTime);
        let lambda = self.lower_lambda(&shape, &items[1], &items[2..]);
        self.phase = outer_phase;
        lambda
    }
}

#[cfg(test)]
mod tests {
    use super::MacroTable;
    use crate::core::BindingId;
    use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
    use crate::expand::Expander;
    use crate::reader::{Form, FormKind};
    use crate::source::{SourceId, Span};

    struct MacroTableFixture;

    impl MacroTableFixture {
        fn with_pending(binding: BindingId) -> MacroTable {
            let span = Span::new(SourceId::new(0), 0..0);
            let mut table = MacroTable::default();
            table.add_pending(binding, Form::new(FormKind::List(Vec::new()), span));
            table
        }
    }

    #[test]
    fn taking_the_ready_macros_leaves_a_pending_one() {
        let binding = BindingId::from_index(0);
        let mut table = MacroTableFixture::with_pending(binding);
        assert!(table.take_ready().is_empty());
        assert_eq!(table.pending().len(), 1);
    }

    struct CallFixture;

    impl CallFixture {
        fn span(range: std::ops::Range<usize>) -> Span {
            Span::new(SourceId::new(0), range)
        }

        fn error_at(range: std::ops::Range<usize>) -> Diagnostic {
            Diagnostic::error(
                DiagnosticCode::UNBOUND_NAME,
                "an error",
                Label::new(Self::span(range), "here"),
            )
        }
    }

    #[test]
    fn an_error_outside_its_call_gets_a_label_marking_the_call() {
        let pointed =
            Expander::point_at_call(CallFixture::error_at(20..21), CallFixture::span(0..5));
        let call = pointed.labels().last().expect("the call is labeled");
        assert!(call.marks_a_macro_call());
        assert_eq!(call.span(), CallFixture::span(0..5));
    }

    #[test]
    fn an_error_inside_its_call_gets_no_label() {
        let pointed = Expander::point_at_call(CallFixture::error_at(1..2), CallFixture::span(0..5));
        assert_eq!(pointed.labels().count(), 1);
    }
}
