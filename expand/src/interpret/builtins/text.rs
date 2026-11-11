use std::rc::Rc;

use super::Forms;
use super::failure::FailCall;
use super::joined_parts::JoinedParts;
use crate::core::Declaration;
use crate::diagnostic::{Diagnostic, SimilarName};
use crate::interpret::arguments::Arguments;
use crate::interpret::{CompileTimeHost, StepBudget, Value, WorkMeter};
use crate::reader::{Form, FormKind, Symbol};
use crate::source::Span;

pub struct TextOperations<'a> {
    arguments: &'a Arguments<'a>,
    call_site: Span,
}

impl<'a> TextOperations<'a> {
    const ESCAPES: [(char, &'static str); 6] = [
        ('"', "\\\""),
        ('\\', "\\\\"),
        ('\n', "\\n"),
        ('\r', "\\r"),
        ('\t', "\\t"),
        ('\0', "\\0"),
    ];
    const STRING_BRACES: [(char, &'static str); 2] = [('{', "{{"), ('}', "}}")];
    const FORMAT_BRACES: [(char, &'static str); 2] = [('{', "{{{{"), ('}', "}}}}")];
    const FEWEST_SYMBOL_ARGUMENTS: usize = 1;

    pub fn new(arguments: &'a Arguments<'a>, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn is_kind(&self, matches: fn(&FormKind) -> bool) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let is_match = self
            .arguments
            .value(0)
            .as_form()
            .is_some_and(|form| matches(form.kind()));
        Ok(Value::Bool(is_match))
    }

    pub fn is_list(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Bool(self.arguments.value(0).is_list()))
    }

    pub fn symbol_name(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let name = self.arguments.symbol(0)?.name().to_string();
        self.string_value(name)
    }

    pub fn is_keyword(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Bool(
            self.arguments.value(0).keyword_name().is_some(),
        ))
    }

    pub fn keyword_name(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let name = self.arguments.keyword_name(0)?.to_string();
        self.string_value(name)
    }

    pub fn keyword_field(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Form(
            self.arguments.keyword_part(0, Value::keyword_field)?,
        ))
    }

    pub fn field_keyword(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Form(self.arguments.field_keyword(0)?))
    }

    pub fn keyword_trim_end(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Form(
            self.arguments.keyword_part(0, Value::keyword_trim_end)?,
        ))
    }

    pub fn make_symbol(&self) -> Result<Value, Diagnostic> {
        self.expect_symbol_arguments()?;
        let context = self.arguments.symbol(0)?;
        let name = JoinedParts::new(self.arguments).from(1)?;
        let symbol = Symbol::with_scopes(name, context.scopes().clone());
        Ok(Value::Form(Form::new(
            FormKind::Symbol(symbol),
            self.call_site,
        )))
    }

    pub fn make_string(&self) -> Result<Value, Diagnostic> {
        self.string_value(JoinedParts::new(self.arguments).from(0)?)
    }

    pub fn same_binding(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let left = self.arguments.symbol(0)?;
        let right = self.arguments.symbol(1)?;
        Ok(Value::Bool(host.same_binding(left, right)))
    }

    pub fn is_pattern_head(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let head = self.arguments.symbol(0)?;
        Ok(Value::Bool(host.is_pattern_head(head)))
    }

    pub fn definition_symbol(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.expect_symbol_arguments()?;
        let term = self.arguments.form(0)?;
        let context = self.arguments.symbol(0)?;
        let symbol = host.definition_symbol(
            context,
            JoinedParts::new(self.arguments).from(1)?,
            term.span(),
        )?;
        Ok(self.symbol_value(symbol))
    }

    pub fn type_definition_symbol(
        &self,
        host: &mut dyn CompileTimeHost,
    ) -> Result<Value, Diagnostic> {
        self.expect_symbol_arguments()?;
        let term = self.arguments.symbol(0)?;
        let symbol = host.type_definition_symbol(term, JoinedParts::new(self.arguments).from(1)?);
        Ok(self.symbol_value(symbol))
    }

    pub fn definition_call(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let term = self.arguments.form(0)?;
        let call = host.definition_call(self.arguments.symbol(0)?, term.span())?;
        Forms::list(call.into_iter().collect(), self.call_site, self.arguments)
    }

    pub fn type_definition_call(
        &self,
        host: &mut dyn CompileTimeHost,
    ) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let call = host.type_definition_call(self.arguments.symbol(0)?);
        Forms::list(call.into_iter().collect(), self.call_site, self.arguments)
    }

    pub fn expand_call(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        self.arguments.charge(StepBudget::EXPAND_CALL_WORK)?;
        let expansion = host.expand_call(self.arguments.form(0)?)?;
        self.arguments
            .charge(expansion.as_ref().map_or(0, Form::form_count))?;
        Forms::list(
            expansion.into_iter().collect(),
            self.call_site,
            self.arguments,
        )
    }

    fn expect_symbol_arguments(&self) -> Result<(), Diagnostic> {
        let count = self.arguments.values().len();
        self.arguments
            .expect_count(count.max(Self::FEWEST_SYMBOL_ARGUMENTS))
    }

    fn symbol_value(&self, symbol: Symbol) -> Value {
        Value::Form(Form::new(FormKind::Symbol(symbol), self.call_site))
    }

    pub fn add_module_note(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let note = self.arguments.form(1)?;
        self.arguments.charge(note.form_count())?;
        host.add_module_note(self.arguments.symbol(0)?.name(), note)
            .map_err(|too_large| too_large.at(self.arguments.function(), self.arguments.span()))?;
        Ok(Value::Unit)
    }

    pub fn module_notes(&self, host: &mut dyn CompileTimeHost) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let context = self.arguments.symbol(0)?;
        let notes = host.module_notes(context, self.arguments.symbol(1)?.name());
        Forms::list(notes, self.call_site, self.arguments)
    }

    pub fn declare_definition(
        &self,
        host: &mut dyn CompileTimeHost,
        declaration: Declaration,
    ) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        host.declare_definition(self.arguments.form(0)?, declaration);
        Ok(Value::Unit)
    }

    pub fn closest_name(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let name = self.arguments.symbol(0)?.name();
        let candidates = self.arguments.symbol_forms(1)?;
        let names: Vec<Rc<str>> = candidates
            .iter()
            .filter_map(Form::as_symbol)
            .map(|symbol| Rc::from(symbol.name()))
            .collect();
        let candidate_bytes: usize = names.iter().map(|candidate| candidate.len()).sum();
        self.arguments
            .charge(name.len().saturating_mul(candidate_bytes))?;
        let closest = SimilarName::closest(name, &names);
        let found = candidates
            .iter()
            .filter(|candidate| candidate.as_symbol().map(Symbol::name) == closest.as_deref())
            .take(1)
            .cloned()
            .collect();
        Forms::list(found, self.call_site, self.arguments)
    }

    pub fn sorted_names(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let mut names = self.arguments.symbol_forms(0)?;
        self.arguments.charge(WorkMeter::text_work(
            names
                .iter()
                .filter_map(Form::as_symbol)
                .map(|symbol| symbol.name().len())
                .sum(),
        ))?;
        names.sort_by(|left, right| {
            let left = left.as_symbol().map(Symbol::name);
            left.cmp(&right.as_symbol().map(Symbol::name))
        });
        names.dedup_by(|later, earlier| {
            later.as_symbol().map(Symbol::name) == earlier.as_symbol().map(Symbol::name)
        });
        Forms::list(names, self.call_site, self.arguments)
    }

    pub fn fail(&self) -> Result<Value, Diagnostic> {
        Err(FailCall::new(self.arguments).failure())
    }

    pub fn value_eq(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let left = self.arguments.value(0);
        let right = self.arguments.value(1);
        let equal = match (self.form_of(left)?, self.form_of(right)?) {
            (Some(left), Some(right)) => self.forms_equal(&left, &right)?,
            _ => Self::plain_values_equal(left, right),
        };
        Ok(Value::Bool(equal))
    }

    fn form_of(&self, value: &Value) -> Result<Option<Form>, Diagnostic> {
        let form = value
            .to_form()
            .map_err(|large| large.at(self.arguments.span()))?;
        if let (Value::List(_), Some(built)) = (value, &form) {
            self.arguments.charge(built.form_count())?;
        }
        Ok(form)
    }

    fn plain_values_equal(left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Bool(left), Value::Bool(right)) => left == right,
            (Value::Unit, Value::Unit) => true,
            _ => false,
        }
    }

    pub fn not(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(Value::Bool(!self.arguments.boolean(0)?))
    }

    pub fn escape_string(&self) -> Result<Value, Diagnostic> {
        self.escape(&Self::STRING_BRACES)
    }

    pub fn escape_char(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let text = self.arguments.string(0)?;
        let mut escaped = String::new();
        for character in text.chars().flat_map(char::escape_debug) {
            match character {
                '{' => escaped.push_str("{{"),
                '}' => escaped.push_str("}}"),
                _ => escaped.push(character),
            }
        }
        self.string_value(escaped)
    }

    pub fn escape_format(&self) -> Result<Value, Diagnostic> {
        self.escape(&Self::FORMAT_BRACES)
    }

    pub fn string_strip_suffix(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let text = self.arguments.string(0)?;
        let suffix = self.arguments.string(1)?;
        self.string_value(text.strip_suffix(suffix).unwrap_or(text).to_string())
    }

    pub fn string_strip_prefix(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let text = self.arguments.string(0)?;
        let prefix = self.arguments.string(1)?;
        self.string_value(text.strip_prefix(prefix).unwrap_or(text).to_string())
    }

    fn escape(&self, braces: &[(char, &'static str); 2]) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let text = self.arguments.string(0)?;
        let mut escaped = String::new();
        for character in text.chars() {
            match Self::ESCAPES
                .iter()
                .chain(braces)
                .find(|(from, _)| *from == character)
            {
                Some((_, escape)) => escaped.push_str(escape),
                None if character.is_control() => {
                    escaped.push_str(&format!("\\u{{{{{:x}}}}}", u32::from(character)))
                }
                None => escaped.push(character),
            }
        }
        self.string_value(escaped)
    }

    fn forms_equal(&self, left: &Form, right: &Form) -> Result<bool, Diagnostic> {
        let (equal, text_bytes) = match (left.kind(), right.kind()) {
            (FormKind::List(left), FormKind::List(right)) => return self.lists_equal(left, right),
            (FormKind::Symbol(left), FormKind::Symbol(right)) => {
                (left.name() == right.name(), left.name().len())
            }
            (FormKind::String(left), FormKind::String(right)) => (left == right, left.len()),
            (FormKind::Number(left), FormKind::Number(right)) => (left.value() == right.value(), 0),
            _ => (false, 0),
        };
        self.arguments
            .charge(1 + WorkMeter::text_work(text_bytes))?;
        Ok(equal)
    }

    fn lists_equal(&self, left: &[Form], right: &[Form]) -> Result<bool, Diagnostic> {
        self.arguments.charge(1)?;
        if left.len() != right.len() {
            return Ok(false);
        }
        for (left, right) in left.iter().zip(right) {
            if !self.forms_equal(left, right)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn string_value(&self, text: String) -> Result<Value, Diagnostic> {
        Forms::string(text, self.call_site, self.arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::TextOperations;
    use crate::interpret::arguments::{Arguments, BuiltinSite};
    use crate::interpret::{StepBudget, Value, WorkMeter};
    use crate::reader::{Form, FormKind, Number, NumberValue, Symbol};
    use crate::scope::{ScopeAllocator, ScopeSet};
    use crate::source::{SourceId, Span};

    struct SymbolComparison {
        scopes: ScopeAllocator,
    }

    impl SymbolComparison {
        const FUNCTION: &'static str = "value-eq";

        fn new() -> Self {
            Self {
                scopes: ScopeAllocator::default(),
            }
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn arguments(values: Vec<Value>) -> Arguments<'static> {
            let site = BuiltinSite {
                function: Self::FUNCTION,
                span: Self::span(),
            };
            Arguments::new(
                site,
                values,
                Box::leak(Box::new(WorkMeter::new(
                    StepBudget::unmetered(),
                    Self::span(),
                ))),
            )
        }

        fn form(kind: FormKind) -> Value {
            Value::Form(Form::new(kind, Self::span()))
        }

        fn scoped_symbol(&mut self, name: &str) -> Value {
            let mut scopes = ScopeSet::default();
            scopes.add(self.scopes.fresh());
            let symbol = Symbol::with_scopes(name, scopes);
            Value::Form(Form::new(FormKind::Symbol(symbol), Self::span()))
        }

        fn equal(left: Value, right: Value) -> bool {
            let arguments = Self::arguments(vec![left, right]);
            let result = TextOperations::new(&arguments, Self::span()).value_eq();
            matches!(result, Ok(Value::Bool(true)))
        }

        fn name(symbol: &Value) -> Value {
            let arguments = Self::arguments(vec![symbol.clone()]);
            TextOperations::new(&arguments, Self::span())
                .symbol_name()
                .expect("a symbol has a name")
        }

        fn agrees_with_names(&mut self, left: &str, right: &str) -> (bool, bool) {
            let left = self.scoped_symbol(left);
            let right = self.scoped_symbol(right);
            let by_names = Self::equal(Self::name(&left), Self::name(&right));
            (Self::equal(left, right), by_names)
        }
    }

    #[test]
    fn numbers_of_one_value_are_equal() {
        let one =
            SymbolComparison::form(FormKind::Number(Number::new("1", NumberValue::Integer(1))));
        assert!(SymbolComparison::equal(one.clone(), one));
    }

    #[test]
    fn numbers_of_different_values_differ() {
        let one =
            SymbolComparison::form(FormKind::Number(Number::new("1", NumberValue::Integer(1))));
        let two =
            SymbolComparison::form(FormKind::Number(Number::new("2", NumberValue::Integer(2))));
        assert!(!SymbolComparison::equal(one, two));
    }

    #[test]
    fn lists_of_different_lengths_differ() {
        let leaf = Form::new(FormKind::Symbol(Symbol::new("a")), SymbolComparison::span());
        let short = SymbolComparison::form(FormKind::List(vec![leaf.clone()]));
        let long = SymbolComparison::form(FormKind::List(vec![leaf.clone(), leaf]));
        assert!(!SymbolComparison::equal(short, long));
    }

    #[test]
    fn lists_that_differ_in_one_item_differ() {
        let first = Form::new(FormKind::Symbol(Symbol::new("a")), SymbolComparison::span());
        let second = Form::new(FormKind::Symbol(Symbol::new("b")), SymbolComparison::span());
        let left = SymbolComparison::form(FormKind::List(vec![first.clone(), first.clone()]));
        let right = SymbolComparison::form(FormKind::List(vec![first, second]));
        assert!(!SymbolComparison::equal(left, right));
    }

    #[test]
    fn forms_of_different_kinds_differ() {
        let symbol = SymbolComparison::form(FormKind::Symbol(Symbol::new("a")));
        let text = SymbolComparison::form(FormKind::String("a".into()));
        assert!(!SymbolComparison::equal(symbol, text));
    }

    #[test]
    fn symbols_of_one_name_in_different_scopes_are_equal_as_their_names_are() {
        let (by_symbols, by_names) = SymbolComparison::new().agrees_with_names("item", "item");
        assert!(by_symbols);
        assert_eq!(by_symbols, by_names);
    }

    #[test]
    fn symbols_of_different_names_differ_as_their_names_do() {
        let (by_symbols, by_names) = SymbolComparison::new().agrees_with_names("item", "items");
        assert!(!by_symbols);
        assert_eq!(by_symbols, by_names);
    }
}
