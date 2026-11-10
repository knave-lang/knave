use std::fmt::{self, Write};

use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::Arguments;
use crate::interpret::build_limit::BuiltValue;
use crate::interpret::{BuildLimit, LimitedText, WorkMeter};
use crate::reader::FormKind;

pub struct JoinedParts<'a> {
    arguments: &'a Arguments<'a>,
}

impl<'a> JoinedParts<'a> {
    pub fn new(arguments: &'a Arguments<'a>) -> Self {
        Self { arguments }
    }

    pub fn from(&self, start: usize) -> Result<String, Diagnostic> {
        let mut text = LimitedText::default();
        for index in start..self.arguments.values().len() {
            let written_before = text.built_size();
            let part = self.write_part(&mut text, index)?;
            let written = text.built_size() - written_before;
            self.arguments.charge(WorkMeter::text_work(written))?;
            if part.is_err() {
                break;
            }
        }
        Ok(BuildLimit::of(self.arguments).check(text)?.into_text())
    }

    fn write_part(&self, text: &mut LimitedText, index: usize) -> Result<fmt::Result, Diagnostic> {
        let form = self.arguments.form(index)?;
        Ok(match form.kind() {
            FormKind::Symbol(symbol) => text.write_str(symbol.name()),
            FormKind::String(part) => text.write_str(part),
            FormKind::Number(number) => text.write_str(number.text()),
            FormKind::List(_) => write!(text, "{form}"),
        })
    }
}
