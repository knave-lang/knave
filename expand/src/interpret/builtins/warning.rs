use super::code_book::CodeBook;
use super::failure::FailureDescription;
use crate::diagnostic::{CodeStage, Diagnostic, RuleName};
use crate::interpret::arguments::Arguments;
use crate::interpret::warning_problems::WarningProblems;
use crate::interpret::{Value, Warnings};
use crate::reader::FormKind;

pub struct WarnCall<'a> {
    arguments: &'a Arguments<'a>,
}

impl<'a> WarnCall<'a> {
    const MESSAGE: usize = 0;
    const CULPRIT: usize = 1;
    const HELP: usize = 2;
    const FEWEST_ARGUMENTS: usize = 3;
    const MOST_ARGUMENTS: usize = 4;

    pub fn new(arguments: &'a Arguments<'a>) -> Self {
        Self { arguments }
    }

    pub fn report(&self, warnings: &mut dyn Warnings) -> Result<Value, Diagnostic> {
        warnings.report_warning(self.warning()?);
        Ok(Value::Unit)
    }

    fn warning(&self) -> Result<Diagnostic, Diagnostic> {
        self.expect_count_in_range()?;
        let message = self.arguments.string(Self::MESSAGE)?.to_string();
        let culprit = self.arguments.form(Self::CULPRIT)?.span();
        let help = self.help()?;
        let description = self.description()?;
        let rule = self.rule(&description)?;
        let failure = description.diagnostic(message, culprit, help);
        Ok(failure.into_warning(rule))
    }

    fn expect_count_in_range(&self) -> Result<(), Diagnostic> {
        let count = self.arguments.values().len();
        self.arguments
            .expect_count(count.clamp(Self::FEWEST_ARGUMENTS, Self::MOST_ARGUMENTS))
    }

    fn description(&self) -> Result<FailureDescription, Diagnostic> {
        let position = self.description_position();
        match self.arguments.value(position) {
            Value::Form(description) if matches!(description.kind(), FormKind::String(_)) => {
                Err(WarningProblems::missing_lint(description.span()))
            }
            _ => FailureDescription::read(&self.arguments.list_forms(position)?, &CodeBook::LINTS),
        }
    }

    fn rule(&self, description: &FailureDescription) -> Result<RuleName, Diagnostic> {
        match description
            .code()
            .and_then(|code| CodeStage::LINTS.name_of(code))
        {
            Some(name) => Ok(RuleName::new(&name)),
            None => Err(WarningProblems::missing_lint(
                self.arguments.form(self.description_position())?.span(),
            )),
        }
    }

    fn help(&self) -> Result<Option<String>, Diagnostic> {
        if self.description_position() == Self::HELP {
            Ok(None)
        } else {
            Ok(Some(self.arguments.string(Self::HELP)?.to_string()))
        }
    }

    fn description_position(&self) -> usize {
        self.arguments.values().len() - 1
    }
}
