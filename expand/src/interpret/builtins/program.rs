use super::Forms;
use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::Arguments;
use crate::interpret::list_size::ListSize;
use crate::interpret::{BuildLimit, ModuleQueries, PlannedList, SourceQueries, Value, WorkMeter};
use crate::reader::{Form, FormKind, Number, NumberValue, Symbol};
use crate::source::Span;

pub struct ProgramOperations<'a> {
    arguments: &'a Arguments<'a>,
    call_site: Span,
}

impl<'a> ProgramOperations<'a> {
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

    pub fn pinned_at(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let place = self.arguments.form(0)?.span();
        let pinned = Span::new(place.source().pinned(), place.range());
        Ok(Value::Form(self.arguments.form(1)?.located_at(pinned)))
    }

    pub fn in_tasks_directory(&self, sources: &dyn SourceQueries) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Bool(
            sources.in_tasks_directory(&self.arguments.form(0)?),
        ))
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

    pub fn located(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let place = self.arguments.form(0)?.span();
        Ok(Value::Form(self.arguments.form(1)?.located_at(place)))
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
