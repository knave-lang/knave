use crate::diagnostic::Diagnostic;
use crate::interpret::Value;
use crate::interpret::arguments::Arguments;

pub struct StepLimitSetting<'a> {
    arguments: &'a Arguments,
}

impl<'a> StepLimitSetting<'a> {
    pub fn new(arguments: &'a Arguments) -> Self {
        Self { arguments }
    }

    pub fn apply(&self, set_limit: impl FnOnce(u64)) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        set_limit(self.arguments.index(0)? as u64);
        Ok(Value::Unit)
    }
}
