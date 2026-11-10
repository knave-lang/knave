use super::problems::InterpretProblems;
use super::work_meter::WorkMeter;
use super::{CodeList, Value};
use crate::diagnostic::Diagnostic;
use crate::reader::{Form, FormKind, NumberValue, Symbol};
use crate::source::Span;

pub struct BuiltinSite {
    pub function: &'static str,
    pub span: Span,
}

pub struct Arguments<'m> {
    site: BuiltinSite,
    values: Vec<Value>,
    meter: &'m WorkMeter,
}

impl<'m> Arguments<'m> {
    pub fn new(site: BuiltinSite, values: Vec<Value>, meter: &'m WorkMeter) -> Self {
        Self {
            site,
            values,
            meter,
        }
    }

    pub fn charge(&self, work: usize) -> Result<(), Diagnostic> {
        self.meter.charge(work)
    }

    pub fn span(&self) -> Span {
        self.site.span
    }

    pub fn function(&self) -> &'static str {
        self.site.function
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }

    pub fn expect_count(&self, expected: usize) -> Result<(), Diagnostic> {
        if self.values.len() == expected {
            Ok(())
        } else {
            Err(InterpretProblems::builtin_argument_count(
                self.site.function,
                expected,
                self.values.len(),
                self.site.span,
            ))
        }
    }

    pub fn value(&self, index: usize) -> &Value {
        &self.values[index]
    }

    pub fn code(&self, index: usize) -> Result<Value, Diagnostic> {
        let value = self.value(index);
        if value.is_code() {
            Ok(value.clone())
        } else {
            Err(self.wrong(index, "code"))
        }
    }

    pub fn form(&self, index: usize) -> Result<Form, Diagnostic> {
        let form = self.values[index]
            .to_form()
            .map_err(|large| large.at(self.site.span))?
            .ok_or_else(|| self.wrong(index, "code"))?;
        if matches!(self.values[index], Value::List(_)) {
            self.charge(form.form_count())?;
        }
        Ok(form)
    }

    pub fn list(&self, index: usize) -> Result<CodeList, Diagnostic> {
        self.code(index)?
            .as_code_list()
            .ok_or_else(|| self.wrong(index, "a list"))
    }

    pub fn list_forms(&self, index: usize) -> Result<Vec<Form>, Diagnostic> {
        let forms = self
            .list(index)?
            .forms()
            .map_err(|large| large.at(self.site.span))?;
        self.charge(forms.iter().map(Form::form_count).sum())?;
        Ok(forms)
    }

    pub fn symbol(&self, index: usize) -> Result<&Symbol, Diagnostic> {
        let symbol = self
            .atom(index, "a symbol")?
            .as_symbol()
            .ok_or_else(|| self.wrong(index, "a symbol"))?;
        self.charge(WorkMeter::text_work(symbol.name().len()))?;
        Ok(symbol)
    }

    pub fn symbol_forms(&self, index: usize) -> Result<Vec<Form>, Diagnostic> {
        let forms = self.list_forms(index)?;
        if forms.iter().all(|form| form.as_symbol().is_some()) {
            Ok(forms)
        } else {
            Err(self.wrong(index, "a list of symbols"))
        }
    }

    pub fn keyword_name(&self, index: usize) -> Result<&str, Diagnostic> {
        let name = self.values[index]
            .keyword_name()
            .ok_or_else(|| self.wrong(index, "a keyword"))?;
        self.charge(WorkMeter::text_work(name.len()))?;
        Ok(name)
    }

    pub fn keyword_part(
        &self,
        index: usize,
        read: fn(&Value) -> Option<Form>,
    ) -> Result<Form, Diagnostic> {
        let part = read(&self.values[index]).ok_or_else(|| self.wrong(index, "a keyword"))?;
        self.charge(WorkMeter::text_work(
            part.as_symbol().map_or(0, |symbol| symbol.name().len()),
        ))?;
        Ok(part)
    }

    pub fn field_keyword(&self, index: usize) -> Result<Form, Diagnostic> {
        let keyword = self.values[index]
            .field_keyword()
            .ok_or_else(|| self.wrong(index, "a field name"))?;
        self.charge(WorkMeter::text_work(
            keyword.as_symbol().map_or(0, |symbol| symbol.name().len()),
        ))?;
        Ok(keyword)
    }

    pub fn string(&self, index: usize) -> Result<&str, Diagnostic> {
        match self.atom(index, "a string")?.kind() {
            FormKind::String(text) => {
                self.charge(WorkMeter::text_work(text.len()))?;
                Ok(text)
            }
            _ => Err(self.wrong(index, "a string")),
        }
    }

    pub fn integer(&self, index: usize) -> Result<i128, Diagnostic> {
        match self.atom(index, "an integer")?.kind() {
            FormKind::Number(number) => match number.value() {
                NumberValue::Integer(value) => Ok(*value),
                NumberValue::Float(_) => Err(self.wrong(index, "an integer")),
            },
            _ => Err(self.wrong(index, "an integer")),
        }
    }

    pub fn index(&self, index: usize) -> Result<usize, Diagnostic> {
        usize::try_from(self.integer(index)?).map_err(|_| self.wrong(index, "a count"))
    }

    pub fn boolean(&self, index: usize) -> Result<bool, Diagnostic> {
        match self.values[index] {
            Value::Bool(value) => Ok(value),
            _ => Err(self.wrong(index, "a boolean")),
        }
    }

    fn atom(&self, index: usize, expected: &str) -> Result<&Form, Diagnostic> {
        match &self.values[index] {
            Value::Form(form) => Ok(form),
            Value::List(_) => Err(self.wrong(index, expected)),
            _ => Err(self.wrong(index, "code")),
        }
    }

    fn wrong(&self, index: usize, expected: &str) -> Diagnostic {
        let position = index + 1;
        InterpretProblems::wrong_argument(
            self.site.function,
            position,
            expected,
            &self.values[index],
            self.site.span,
        )
    }
}
