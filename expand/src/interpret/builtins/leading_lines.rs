use rustc_hash::FxHashSet;

use crate::diagnostic::Diagnostic;
use crate::interpret::arguments::Arguments;
use crate::interpret::problems::InterpretProblems;
use crate::interpret::{ListNesting, Value};
use crate::reader::{Form, FormKind};
use crate::source::Span;

pub struct DistinctLeadingLines<'a> {
    arguments: &'a Arguments,
    call_site: Span,
}

impl<'a> DistinctLeadingLines<'a> {
    const TEXTS_POSITION: usize = 1;

    pub fn new(arguments: &'a Arguments, call_site: Span) -> Self {
        Self {
            arguments,
            call_site,
        }
    }

    pub fn kept_texts(&self) -> Result<Value, Diagnostic> {
        self.arguments.expect_count(1)?;
        let forms = self.arguments.list_forms(0)?;
        let mut leading_lines = LeadingLines::default();
        let mut kept = Vec::with_capacity(forms.len());
        for form in &forms {
            if leading_lines.admit(self.text_of(form)?) {
                kept.push(form.clone());
            }
        }
        ListNesting::form_list(kept, self.call_site).map_err(|deep| deep.at(self.arguments.span()))
    }

    fn text_of<'f>(&self, form: &'f Form) -> Result<&'f str, Diagnostic> {
        match form.kind() {
            FormKind::String(text) => Ok(text),
            _ => Err(InterpretProblems::wrong_argument(
                self.arguments.function(),
                Self::TEXTS_POSITION,
                "a list of strings",
                &Value::Form(form.clone()),
                form.span(),
            )),
        }
    }
}

#[derive(Default)]
struct LeadingLines<'t> {
    seen: FxHashSet<&'t str>,
}

impl<'t> LeadingLines<'t> {
    const LINE_BREAK: char = '\n';

    fn admit(&mut self, text: &'t str) -> bool {
        if self.seen.contains(text) {
            return false;
        }
        self.seen.insert(text);
        let leading_lines = text
            .match_indices(Self::LINE_BREAK)
            .map(|(line_end, _)| &text[..line_end]);
        self.seen.extend(leading_lines);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::LeadingLines;

    #[test]
    fn admits_a_new_text() {
        assert!(LeadingLines::default().admit("area"));
    }

    #[test]
    fn refuses_a_repeated_text() {
        let mut lines = LeadingLines::default();
        lines.admit("area");
        assert!(!lines.admit("area"));
    }

    #[test]
    fn refuses_the_leading_line_of_an_admitted_text() {
        let mut lines = LeadingLines::default();
        lines.admit("area\n    Measures.");
        assert!(!lines.admit("area"));
    }

    #[test]
    fn refuses_every_leading_run_of_lines() {
        let mut lines = LeadingLines::default();
        lines.admit("one\ntwo\nthree");
        assert!(!lines.admit("one\ntwo"));
    }

    #[test]
    fn admits_a_text_that_only_starts_like_an_admitted_one() {
        let mut lines = LeadingLines::default();
        lines.admit("area\n    Measures.");
        assert!(lines.admit("are"));
    }

    #[test]
    fn admits_a_longer_text_after_its_leading_line() {
        let mut lines = LeadingLines::default();
        lines.admit("area");
        assert!(lines.admit("area\n    Measures."));
    }
}
