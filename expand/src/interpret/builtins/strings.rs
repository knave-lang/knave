use super::Forms;
use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::Arguments;
use crate::interpret::problems::InterpretProblems;
use crate::interpret::{CodeList, Value};
use crate::reader::{Form, FormKind};
use crate::source::Span;

pub struct StringOperations<'a> {
    arguments: &'a Arguments,
    call_site: Span,
}

impl<'a> StringOperations<'a> {
    const LINE_BREAK: char = '\n';

    pub fn new(arguments: &'a Arguments, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn lines(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let lines = self
            .arguments
            .string(0)?
            .split(Self::LINE_BREAK)
            .map(|line| self.string_value(line.to_string()))
            .collect();
        CodeList::new(lines, self.call_site)
            .map(Value::List)
            .map_err(|deep| deep.at(self.arguments.span()))
    }

    pub fn trim_start(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let text = self.arguments.string(0)?;
        Ok(self.string_value(text.trim_start().to_string()))
    }

    pub fn trim_end(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let text = self.arguments.string(0)?;
        Ok(self.string_value(text.trim_end().to_string()))
    }

    pub fn length(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let count = self.arguments.string(0)?.chars().count();
        Ok(Forms::count(count, self.call_site))
    }

    pub fn drop(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let text = self.arguments.string(0)?;
        let count = self.arguments.index(1)?;
        let start = self.char_boundary(text, count)?;
        Ok(self.string_value(text[start..].to_string()))
    }

    pub fn lowercase(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        Ok(self.string_value(self.arguments.string(0)?.to_lowercase()))
    }

    pub fn contains(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let text = self.arguments.string(0)?;
        let part = self.arguments.string(1)?;
        Ok(Value::Bool(text.contains(part)))
    }

    fn char_boundary(&self, text: &str, count: usize) -> Result<usize, Diagnostic> {
        text.char_indices()
            .map(|(boundary, _)| boundary)
            .chain(std::iter::once(text.len()))
            .nth(count)
            .ok_or_else(|| {
                InterpretProblems::past_string_end(self.arguments.function(), self.arguments.span())
            })
    }

    fn string_value(&self, text: String) -> Value {
        Value::Form(Form::new(FormKind::String(text.into()), self.call_site))
    }
}

#[cfg(test)]
mod tests {
    use super::StringOperations;
    use crate::interpret::arguments::{Arguments, BuiltinSite};
    use crate::interpret::{StepBudget, Value, WorkMeter};
    use crate::reader::{Form, FormKind};
    use crate::source::{SourceId, Span};

    struct StringCall {
        arguments: Arguments,
    }

    impl StringCall {
        const FUNCTION: &'static str = "string-search";

        fn of(texts: &[&str]) -> Self {
            let values = texts
                .iter()
                .map(|text| Value::Form(Self::string(text)))
                .collect();
            let site = BuiltinSite {
                function: Self::FUNCTION,
                span: Self::span(),
            };
            Self {
                arguments: Arguments::new(
                    site,
                    values,
                    WorkMeter::new(StepBudget::unmetered(), Self::span()),
                ),
            }
        }

        fn span() -> Span {
            Span::new(SourceId::new(0), 0..0)
        }

        fn string(text: &str) -> Form {
            Form::new(FormKind::String(text.into()), Self::span())
        }

        fn operations(&self) -> StringOperations<'_> {
            StringOperations::new(&self.arguments, Self::span())
        }

        fn lowercase(&self) -> String {
            let value = self.operations().lowercase().expect("one string lowers");
            match value.as_form().map(Form::kind) {
                Some(FormKind::String(text)) => text.to_string(),
                _ => panic!("the lowercase text is a string"),
            }
        }

        fn contains(&self) -> bool {
            matches!(self.operations().contains(), Ok(Value::Bool(true)))
        }
    }

    #[test]
    fn lowercase_lowers_every_letter() {
        assert_eq!(StringCall::of(&["SoRT-Émile"]).lowercase(), "sort-émile");
    }

    #[test]
    fn contains_finds_a_part_inside_the_text() {
        assert!(StringCall::of(&["into-sorted-map", "sort"]).contains());
    }

    #[test]
    fn contains_refuses_a_part_of_another_case() {
        assert!(!StringCall::of(&["into-sorted-map", "SORT"]).contains());
    }
}
