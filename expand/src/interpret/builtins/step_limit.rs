use crate::diagnostic::Diagnostic;
use crate::interpret::Value;
use crate::interpret::arguments::Arguments;
use crate::reader::{FormKind, NumberValue};

pub struct StepLimitSetting<'a> {
    arguments: &'a Arguments,
}

impl<'a> StepLimitSetting<'a> {
    pub fn new(arguments: &'a Arguments) -> Self {
        Self { arguments }
    }

    pub fn apply(&self, set_limit: impl FnOnce(u64)) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let count = self.step_count();
        if let Some(limit) = count {
            set_limit(limit);
        }
        Ok(Value::Bool(count.is_some()))
    }

    fn step_count(&self) -> Option<u64> {
        match self.arguments.value(0).as_form()?.kind() {
            FormKind::Number(number) => Self::whole_count(number.value()),
            _ => None,
        }
    }

    fn whole_count(value: &NumberValue) -> Option<u64> {
        match value {
            NumberValue::Integer(integer) => u64::try_from(*integer).ok(),
            NumberValue::Float(_) => None,
        }
    }
}
