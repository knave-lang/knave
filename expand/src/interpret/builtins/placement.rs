use crate::diagnostic::Diagnostic;
use crate::interpret::Value;
use crate::interpret::arguments::Arguments;
use crate::source::Span;

pub struct PlacementOperations<'a> {
    arguments: &'a Arguments<'a>,
}

impl<'a> PlacementOperations<'a> {
    pub fn new(arguments: &'a Arguments<'a>) -> Self {
        Self { arguments }
    }

    pub fn pinned_at(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let place = self.arguments.form(0)?.span();
        let pinned = Span::new(place.source().pinned(), place.range());
        Ok(Value::Form(self.arguments.form(1)?.located_at(pinned)))
    }

    pub fn located(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(2)?;
        let place = self.arguments.form(0)?.span();
        Ok(Value::Form(self.arguments.form(1)?.located_at(place)))
    }
}
