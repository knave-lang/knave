use crate::diagnostic::Diagnostic;
use crate::interpret::Value;
use crate::interpret::arguments::Arguments;
use crate::language::Language;
use crate::reader::{Form, FormKind};
use crate::source::Span;

pub struct LanguageNames<'a> {
    arguments: &'a Arguments<'a>,
    call_site: Span,
}

impl<'a> LanguageNames<'a> {
    pub fn new(arguments: &'a Arguments<'a>, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn project_file_name(&self) -> Result<Value, Diagnostic> {
        self.text(&Language::project_file_name())
    }

    pub fn language_name(&self) -> Result<Value, Diagnostic> {
        self.text(Language::NAME)
    }

    fn text(&self, name: &str) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(0)?;
        let name = FormKind::String(name.into());
        Ok(Value::Form(Form::new(name, self.call_site)))
    }
}
