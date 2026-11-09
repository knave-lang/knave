use super::Forms;
use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::Arguments;
use crate::interpret::{CompileTimeHost, Value};
use crate::language::Language;
use crate::reader::{Form, FormKind, Number, NumberValue, Symbol};
use crate::source::Span;

pub struct ProgramOperations<'a> {
    arguments: &'a Arguments,
    call_site: Span,
}

impl<'a> ProgramOperations<'a> {
    pub fn new(arguments: &'a Arguments, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn form_location(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let location = host.form_location(&self.arguments.form(0)?);
        Ok(Value::Form(Form::new(
            FormKind::String(location.into()),
            self.call_site,
        )))
    }

    pub fn project_file_name(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        let name = FormKind::String(Language::project_file_name().into());
        Ok(Value::Form(Form::new(name, self.call_site)))
    }

    pub fn project_modules(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        Ok(self.symbols(host.project_modules()))
    }

    pub fn program_modules(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        Ok(self.symbols(host.program_modules()))
    }

    pub fn module_exports(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let path = self.arguments.form(0)?;
        let exports = host.module_exports(self.arguments.symbol(0)?, path.span())?;
        Ok(self.symbols(exports))
    }

    pub fn module_definitions(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let path = self.arguments.form(0)?;
        let definitions = host.module_definitions(self.arguments.symbol(0)?, path.span())?;
        Ok(self.symbols(definitions))
    }

    pub fn current_module(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        let module = FormKind::Symbol(host.current_module());
        Ok(Value::Form(Form::new(module, self.call_site)))
    }

    pub fn module_symbol(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let path = self.arguments.form(0)?;
        let module = host.module_symbol(self.arguments.symbol(0)?, path.span())?;
        Ok(Value::Form(Form::new(
            FormKind::Symbol(module),
            self.call_site,
        )))
    }

    pub fn read_form(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(3)?;
        let context = self.arguments.symbol(0)?;
        let literal = self.arguments.form(1)?.span();
        let value = self.arguments.string(1)?;
        let read = host.read_literal_form(literal, value, self.arguments.index(2)?);
        Ok(Forms::list(
            read.map_or_else(Vec::new, |(mut form, end)| {
                form.set_scopes(context.scopes());
                vec![form, self.count_form(end)]
            }),
            self.call_site,
        ))
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

    fn symbols(&self, symbols: Vec<Symbol>) -> Value {
        let forms = symbols
            .into_iter()
            .map(|symbol| Form::new(FormKind::Symbol(symbol), self.call_site))
            .collect();
        Forms::list(forms, self.call_site)
    }
}
