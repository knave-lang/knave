use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::bound_names::BoundNames;
use super::rule_call::RuleCall;
use super::rule_global::RuleGlobal;
use super::rule_names::RuleNames;
use super::rule_program::RuleProgram;
use crate::core::{BindingArena, BindingId, BindingKind};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::expand::MacroCallLookup;
use crate::interpret::{
    DefinitionQueries, ExpansionCallbacks, GlobalValues, Host, Interpreter, ModuleNotes,
    ModuleQueries, NameComparison, SourceQueries, StepBudget, Value, Warnings,
};
use crate::reader::{Form, LiteralForms, Symbol};
use crate::source::{SourceMap, Span};

pub struct RuleHost<'h> {
    rules: &'h RuleProgram,
    program: ProgramFacts<'h>,
    values: FxHashMap<BindingId, Value>,
    warnings: Diagnostics,
}

pub struct ProgramFacts<'h> {
    pub names: &'h BoundNames,
    pub bindings: &'h BindingArena,
    pub origins: &'h dyn MacroCallLookup,
    pub sources: &'h SourceMap,
}

impl<'h> RuleHost<'h> {
    pub fn new(rules: &'h RuleProgram, program: ProgramFacts<'h>) -> Self {
        Self {
            rules,
            program,
            values: FxHashMap::default(),
            warnings: Diagnostics::default(),
        }
    }

    pub fn into_warnings(self) -> Diagnostics {
        self.warnings
    }

    pub fn rule_names(&mut self, span: Span) -> Result<Vec<Form>, Diagnostic> {
        let names = self.global_value(self.rules.entries().names, span)?;
        RuleNames::of(&names, span)
    }

    pub fn check_rule(&mut self, call: RuleCall) -> Result<(), Diagnostic> {
        let span = call.span;
        let budget = StepBudget::full();
        budget.set_limit(call.step_limit);
        let check = self.global_value(self.rules.entries().check, span)?;
        Interpreter::new(self, span, budget)
            .call(&check, vec![Value::Form(call.rule), call.table], span)
            .map(|_| ())
    }

    fn evaluate(&mut self, binding: BindingId, span: Span) -> Result<Value, Diagnostic> {
        let rules = self.rules;
        let value = match rules.global(binding, span)? {
            RuleGlobal::Builtin(builtin) => Value::Builtin(builtin),
            RuleGlobal::Definition(definition) => {
                Interpreter::new(self, span, StepBudget::full()).evaluate(definition)?
            }
        };
        self.values.insert(binding, value.clone());
        Ok(value)
    }

    fn binding_kind(&self, symbol: &Symbol) -> Option<BindingKind> {
        let binding = self.program.names.binding_of(symbol)?;
        Some(self.program.bindings.get(binding).kind())
    }
}

impl Host for RuleHost<'_> {
    fn global_values(&mut self) -> Option<&mut dyn GlobalValues> {
        Some(self)
    }

    fn module_queries(&mut self) -> Option<&mut dyn ModuleQueries> {
        None
    }

    fn definition_queries(&mut self) -> Option<&mut dyn DefinitionQueries> {
        None
    }

    fn module_notes(&mut self) -> Option<&mut dyn ModuleNotes> {
        None
    }

    fn name_comparison(&mut self) -> Option<&mut dyn NameComparison> {
        Some(self)
    }

    fn expansion_callbacks(&mut self) -> Option<&mut dyn ExpansionCallbacks> {
        None
    }

    fn source_queries(&mut self) -> Option<&mut dyn SourceQueries> {
        Some(self)
    }

    fn warnings(&mut self) -> Option<&mut dyn Warnings> {
        Some(self)
    }
}

impl GlobalValues for RuleHost<'_> {
    fn global_value(&mut self, binding: BindingId, span: Span) -> Result<Value, Diagnostic> {
        match self.values.get(&binding) {
            Some(value) => Ok(value.clone()),
            None => self.evaluate(binding, span),
        }
    }
}

impl NameComparison for RuleHost<'_> {
    fn same_binding(&self, left: &Symbol, right: &Symbol) -> bool {
        self.program
            .names
            .binding_of(left)
            .is_some_and(|binding| self.program.names.binding_of(right) == Some(binding))
    }

    fn is_pattern_head(&self, head: &Symbol) -> bool {
        matches!(
            self.binding_kind(head),
            Some(BindingKind::Struct(_) | BindingKind::Variant(..))
        )
    }
}

impl SourceQueries for RuleHost<'_> {
    fn form_location(&self, form: &Form) -> String {
        let span = form.span();
        let file = self.program.sources.file(span.source());
        let position = self.program.sources.start_position(span);
        format!("{}:{position}", file.name())
    }

    fn in_tasks_directory(&self, _form: &Form) -> bool {
        false
    }

    fn introducing_macro(&self, form: &Form) -> Option<Rc<str>> {
        let origin = self.program.origins.introducing_macro(form.span())?;
        Some(origin.name)
    }

    fn read_literal_form(
        &self,
        literal: Span,
        value: &str,
        character: usize,
    ) -> Option<(Form, usize)> {
        let file = self.program.sources.file(literal.source());
        LiteralForms::new(file, literal.range(), value).read_at(character)
    }
}

impl Warnings for RuleHost<'_> {
    fn report_warning(&mut self, warning: Diagnostic) {
        self.warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::{ProgramFacts, RuleHost};
    use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind, ModuleId};
    use crate::diagnostic::{Diagnostic, DiagnosticCode, Label};
    use crate::interpret::{Host, NameComparison, SourceQueries, Warnings};
    use crate::lint::binding_symbols::BindingSymbols;
    use crate::lint::bound_names::BoundNames;
    use crate::lint::expanded_source::ExpandedSource;
    use crate::lint::rule_program::RuleProgram;
    use crate::reader::{Form, FormKind, Symbol};

    struct HostFixture {
        rules: RuleProgram,
        source: ExpandedSource,
        arena: BindingArena,
        names: BoundNames,
        made: Vec<Symbol>,
    }

    impl HostFixture {
        const TEXT: &str = "(defrule with-scratch ((body) (let (scratch 0) body)))\n(defn area (width) (with-scratch width))\n(def phrase \"(a b)\")\n";
        const CALL: &str = "(with-scratch width)";
        const LITERAL: &str = "\"(a b)\"";

        fn new() -> Self {
            let mut source = ExpandedSource::expand(Self::TEXT);
            let mut arena = BindingArena::default();
            let total = Self::add(&mut arena, BindingKind::Local);
            let other_total = Self::add(&mut arena, BindingKind::Local);
            let point = Self::add(&mut arena, BindingKind::Struct(ModuleId::new(0)));
            let mut symbols = BindingSymbols::new(source.supply());
            let made = [total, total, other_total, point]
                .iter()
                .map(|binding| symbols.symbol(*binding, &arena))
                .collect();
            let names = symbols.into_names();
            Self {
                rules: RuleProgram::expand_from_sources().expect("the lint rules expand"),
                source,
                arena,
                names,
                made,
            }
        }

        fn add(arena: &mut BindingArena, kind: BindingKind) -> BindingId {
            arena.add(BindingInfo::new("total".into(), kind, None))
        }

        fn host(&self) -> RuleHost<'_> {
            let facts = ProgramFacts {
                names: &self.names,
                bindings: &self.arena,
                origins: self.source.program().call_origins.as_ref(),
                sources: self.source.sources(),
            };
            RuleHost::new(&self.rules, facts)
        }

        fn form_at(&self, text: &str) -> Form {
            Form::new(
                FormKind::Symbol(Symbol::new("form")),
                self.source.span_of(text),
            )
        }

        fn warning(&self) -> Diagnostic {
            let span = self.source.span_of(Self::CALL);
            Diagnostic::warning(
                DiagnosticCode::UNUSED_BINDING,
                "unread",
                Label::new(span, "here"),
            )
        }
    }

    #[test]
    fn offers_only_the_groups_a_rule_needs() {
        let fixture = HostFixture::new();
        let mut host = fixture.host();
        assert!(host.global_values().is_some());
        assert!(host.name_comparison().is_some());
        assert!(host.source_queries().is_some());
        assert!(host.warnings().is_some());
        assert!(host.module_queries().is_none());
        assert!(host.definition_queries().is_none());
        assert!(host.module_notes().is_none());
        assert!(host.expansion_callbacks().is_none());
    }

    #[test]
    fn two_symbols_of_one_binding_are_the_same_binding() {
        let fixture = HostFixture::new();
        assert!(
            fixture
                .host()
                .same_binding(&fixture.made[0], &fixture.made[1])
        );
    }

    #[test]
    fn symbols_of_two_bindings_of_one_name_differ() {
        let fixture = HostFixture::new();
        assert!(
            !fixture
                .host()
                .same_binding(&fixture.made[0], &fixture.made[2])
        );
    }

    #[test]
    fn a_kernel_head_is_no_binding() {
        let fixture = HostFixture::new();
        let head = Symbol::new("local");
        assert!(!fixture.host().same_binding(&head, &head));
    }

    #[test]
    fn a_struct_heads_a_pattern_and_a_local_does_not() {
        let fixture = HostFixture::new();
        assert!(fixture.host().is_pattern_head(&fixture.made[3]));
        assert!(!fixture.host().is_pattern_head(&fixture.made[0]));
    }

    #[test]
    fn locates_a_form_by_its_file_line_and_column() {
        let fixture = HostFixture::new();
        let location = fixture
            .host()
            .form_location(&fixture.form_at(HostFixture::CALL));
        assert_eq!(location, "fixture:2:20");
    }

    #[test]
    fn places_no_form_in_a_tasks_directory() {
        let fixture = HostFixture::new();
        assert!(
            !fixture
                .host()
                .in_tasks_directory(&fixture.form_at(HostFixture::CALL))
        );
    }

    #[test]
    fn names_the_macro_whose_call_introduced_a_form() {
        let fixture = HostFixture::new();
        let introducing = fixture
            .host()
            .introducing_macro(&fixture.form_at(HostFixture::CALL));
        assert_eq!(introducing.as_deref(), Some("with-scratch"));
    }

    #[test]
    fn names_no_macro_for_a_written_form() {
        let fixture = HostFixture::new();
        let written = fixture.form_at("(width)");
        assert_eq!(fixture.host().introducing_macro(&written), None);
    }

    #[test]
    fn reads_a_form_inside_a_string_literal() {
        let fixture = HostFixture::new();
        let literal = fixture.source.span_of(HostFixture::LITERAL);
        let read = fixture.host().read_literal_form(literal, "(a b)", 0);
        assert_eq!(
            read.map(|(form, _)| form.to_string()).as_deref(),
            Some("(a b)")
        );
    }

    #[test]
    fn keeps_each_warning_a_rule_reports() {
        let fixture = HostFixture::new();
        let mut host = fixture.host();
        host.report_warning(fixture.warning());
        assert_eq!(host.into_warnings().len(), 1);
    }

    #[test]
    fn evaluates_the_rule_names_of_the_lint_module() {
        let fixture = HostFixture::new();
        let names = fixture
            .host()
            .rule_names(fixture.source.span_of(HostFixture::CALL))
            .expect("the rule names evaluate");
        let written: Vec<String> = names.iter().map(Form::to_string).collect();
        assert_eq!(written, ["unused-binding"]);
    }
}
