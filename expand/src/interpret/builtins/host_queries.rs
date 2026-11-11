use std::rc::Rc;

use super::Forms;
use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::Arguments;
use crate::interpret::list_size::ListSize;
use crate::interpret::{BuildLimit, ModuleQueries, PlannedList, SourceQueries, Value, WorkMeter};
use crate::reader::{Form, FormKind, Number, NumberValue, Symbol};
use crate::source::Span;

pub struct SourceOperations<'a> {
    arguments: &'a Arguments<'a>,
    call_site: Span,
}

impl<'a> SourceOperations<'a> {
    pub fn new(arguments: &'a Arguments<'a>, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn form_location(&self, sources: &dyn SourceQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let location = sources.form_location(&self.arguments.form(0)?);
        Ok(Value::Form(Form::new(
            FormKind::String(location.into()),
            self.call_site,
        )))
    }

    pub fn in_tasks_directory(&self, sources: &dyn SourceQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Bool(
            sources.in_tasks_directory(&self.arguments.form(0)?),
        ))
    }

    pub fn introducing_macro(&self, sources: &dyn SourceQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let name = sources.introducing_macro(&self.arguments.form(0)?);
        let items = name
            .into_iter()
            .map(|name| self.symbol_form(name))
            .collect();
        Forms::list(items, self.call_site, self.arguments)
    }

    pub fn read_form(&self, sources: &dyn SourceQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(3)?;
        let context = self.arguments.symbol(0)?;
        let literal = self.arguments.form(1)?.span();
        let value = self.arguments.string(1)?;
        let read = sources.read_literal_form(literal, value, self.arguments.index(2)?);
        let items = read.map_or_else(Vec::new, |(mut form, end)| {
            form.set_scopes(context.scopes());
            vec![form, self.count_form(end)]
        });
        self.admit_read(&items)?;
        Forms::list(items, self.call_site, self.arguments)
    }

    fn admit_read(&self, items: &[Form]) -> Result<(), Diagnostic> {
        let size = ListSize::of_counts(items.iter().map(Form::form_count));
        let limit = BuildLimit::of(self.arguments.function(), self.arguments.span());
        limit.check(PlannedList(size))?;
        self.arguments
            .charge(WorkMeter::built_forms_work(size - ListSize::EMPTY))
    }

    fn symbol_form(&self, name: Rc<str>) -> Form {
        Form::new(FormKind::Symbol(Symbol::new(name)), self.call_site)
    }

    fn count_form(&self, count: usize) -> Form {
        let number = Number::new(count.to_string(), NumberValue::Integer(count as i128));
        Form::new(FormKind::Number(number), self.call_site)
    }
}

pub struct ModuleOperations<'a> {
    arguments: &'a Arguments<'a>,
    call_site: Span,
}

impl<'a> ModuleOperations<'a> {
    pub fn new(arguments: &'a Arguments<'a>, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn project_modules(&self, modules: &dyn ModuleQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        self.symbols(modules.project_modules())
    }

    pub fn program_modules(&self, modules: &dyn ModuleQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        self.symbols(modules.program_modules())
    }

    pub fn module_exports(&self, modules: &dyn ModuleQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let path = self.arguments.form(0)?;
        let exports = modules.module_exports(self.arguments.symbol(0)?, path.span())?;
        self.symbols(exports)
    }

    pub fn module_definitions(&self, modules: &dyn ModuleQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let path = self.arguments.form(0)?;
        let definitions = modules.module_definitions(self.arguments.symbol(0)?, path.span())?;
        self.symbols(definitions)
    }

    pub fn current_module(&self, modules: &dyn ModuleQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        let module = FormKind::Symbol(modules.current_module());
        Ok(Value::Form(Form::new(module, self.call_site)))
    }

    pub fn named(
        &self,
        find: impl FnOnce(&Symbol, Span) -> Result<Symbol, Diagnostic>,
    ) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let path = self.arguments.form(0)?;
        let module = find(self.arguments.symbol(0)?, path.span())?;
        Ok(Value::Form(Form::new(
            FormKind::Symbol(module),
            self.call_site,
        )))
    }

    fn symbols(&self, symbols: Vec<Symbol>) -> Result<Value, Diagnostic> {
        let forms = symbols
            .into_iter()
            .map(|symbol| Form::new(FormKind::Symbol(symbol), self.call_site))
            .collect();
        Forms::list(forms, self.call_site, self.arguments)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::SourceOperations;
    use crate::interpret::arguments::{Arguments, BuiltinSite};
    use crate::interpret::{SourceQueries, StepBudget, Value, WorkMeter};
    use crate::reader::{Form, FormKind, Symbol};
    use crate::source::{SourceId, Span};

    struct KnownOrigin {
        introducing: Option<Rc<str>>,
    }

    impl SourceQueries for KnownOrigin {
        fn form_location(&self, _form: &Form) -> String {
            String::new()
        }

        fn in_tasks_directory(&self, _form: &Form) -> bool {
            false
        }

        fn introducing_macro(&self, _form: &Form) -> Option<Rc<str>> {
            self.introducing.clone()
        }

        fn read_literal_form(
            &self,
            _literal: Span,
            _value: &str,
            _character: usize,
        ) -> Option<(Form, usize)> {
            None
        }
    }

    struct SourcesFixture {
        span: Span,
        meter: WorkMeter,
    }

    impl SourcesFixture {
        fn new() -> Self {
            let span = Span::new(SourceId::new(0), 0..0);
            Self {
                span,
                meter: WorkMeter::new(StepBudget::full(), span),
            }
        }

        fn introducing_macro(&self, introducing: Option<&str>) -> String {
            let binder = Form::new(FormKind::Symbol(Symbol::new("hidden")), self.span);
            let site = BuiltinSite {
                function: "introducing-macro",
                span: self.span,
            };
            let arguments = Arguments::new(site, vec![Value::Form(binder)], &self.meter);
            let sources = KnownOrigin {
                introducing: introducing.map(Rc::from),
            };
            match SourceOperations::new(&arguments, self.span).introducing_macro(&sources) {
                Ok(Value::Form(names)) => names.to_string(),
                other => panic!("expected a list of names, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_form_that_a_macro_introduced_names_that_macro() {
        let fixture = SourcesFixture::new();
        assert_eq!(
            fixture.introducing_macro(Some("hide-a-name")),
            "(hide-a-name)"
        );
    }

    #[test]
    fn a_written_form_names_no_macro() {
        assert_eq!(SourcesFixture::new().introducing_macro(None), "()");
    }
}
