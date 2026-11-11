use super::code_book::CodeBook;
use crate::core::Keyword;
use crate::diagnostic::{Diagnostic, DiagnosticCode, Suggestion};
use crate::interpret::arguments::Arguments;
use crate::interpret::problems::InterpretProblems;
use crate::reader::{Form, FormKind};
use crate::source::Span;

pub struct FailCall<'a> {
    arguments: &'a Arguments<'a>,
}

impl<'a> FailCall<'a> {
    const MESSAGE: usize = 0;
    const CULPRIT: usize = 1;
    const HELP: usize = 2;
    const DESCRIPTION: usize = 3;
    const FEWEST_ARGUMENTS: usize = 2;
    const MOST_ARGUMENTS: usize = 4;

    pub fn new(arguments: &'a Arguments<'a>) -> Self {
        Self { arguments }
    }

    pub fn failure(&self) -> Diagnostic {
        match self.diagnostic() {
            Ok(failure) | Err(failure) => failure,
        }
    }

    fn diagnostic(&self) -> Result<Diagnostic, Diagnostic> {
        self.expect_count_in_range()?;
        let message = self.arguments.string(Self::MESSAGE)?.to_string();
        let culprit = self.arguments.form(Self::CULPRIT)?.span();
        let help = self.help()?;
        Ok(self.description()?.diagnostic(message, culprit, help))
    }

    fn expect_count_in_range(&self) -> Result<(), Diagnostic> {
        let count = self.arguments.values().len();
        self.arguments
            .expect_count(count.clamp(Self::FEWEST_ARGUMENTS, Self::MOST_ARGUMENTS))
    }

    fn description(&self) -> Result<FailureDescription, Diagnostic> {
        if self.has_argument(Self::DESCRIPTION) {
            FailureDescription::read(
                &self.arguments.list_forms(Self::DESCRIPTION)?,
                &CodeBook::MACROS,
            )
        } else {
            Ok(FailureDescription::default())
        }
    }

    fn help(&self) -> Result<Option<String>, Diagnostic> {
        if self.has_argument(Self::HELP) {
            Ok(Some(self.arguments.string(Self::HELP)?.to_string()))
        } else {
            Ok(None)
        }
    }

    fn has_argument(&self, index: usize) -> bool {
        index < self.arguments.values().len()
    }
}

pub struct FailureDescription {
    code: Option<DiagnosticCode>,
    label: Option<String>,
    earlier: Option<Earlier>,
    earlier_label: Option<EarlierLabel>,
    replacement: Option<String>,
    note: Option<String>,
}

struct Earlier {
    form: Span,
    part: Span,
}

struct EarlierLabel {
    text: String,
    part: Span,
}

impl FailureDescription {
    pub const PARTS: [&'static str; 6] = [
        Self::CODE,
        Self::EARLIER,
        Self::EARLIER_LABEL,
        Self::LABEL,
        Self::NOTE,
        Self::SUGGEST,
    ];
    const CODE: &'static str = "code";
    const EARLIER: &'static str = "earlier";
    const EARLIER_LABEL: &'static str = "earlier-label";
    const LABEL: &'static str = "label";
    const NOTE: &'static str = "note";
    const SUGGEST: &'static str = "suggest";

    pub fn read(parts: &[Form], codes: &CodeBook) -> Result<Self, Diagnostic> {
        let mut pairs = parts.chunks_exact(2);
        let lone_part = pairs.remainder().first();
        let description = pairs.try_fold(Self::default(), |description, pair| {
            description.with_part(&pair[0], &pair[1], codes)
        })?;
        match lone_part {
            Some(part) => Err(Self::malformed_part(part)),
            None => description.expect_earlier_label(),
        }
    }

    pub fn code(&self) -> Option<&DiagnosticCode> {
        self.code.as_ref()
    }

    fn expect_earlier_label(self) -> Result<Self, Diagnostic> {
        match (self.earlier, self.earlier_label) {
            (Some(earlier), None) => Err(InterpretProblems::missing_earlier_label(earlier.part)),
            (None, Some(earlier_label)) => {
                Err(InterpretProblems::stray_earlier_label(earlier_label.part))
            }
            (earlier, earlier_label) => Ok(Self {
                earlier,
                earlier_label,
                ..self
            }),
        }
    }

    fn with_part(self, part: &Form, value: &Form, codes: &CodeBook) -> Result<Self, Diagnostic> {
        match Self::part_name(part) {
            Some(Self::CODE) => self.with_code(value, codes),
            Some(Self::EARLIER) => Ok(self.with_earlier(part, value)),
            Some(Self::EARLIER_LABEL) => self.with_earlier_label(part, value),
            Some(Self::LABEL) => self.with_label(value),
            Some(Self::NOTE) => self.with_note(value),
            Some(Self::SUGGEST) => self.with_replacement(value),
            _ => Err(Self::malformed_part(part)),
        }
    }

    fn with_code(self, value: &Form, codes: &CodeBook) -> Result<Self, Diagnostic> {
        Ok(Self {
            code: Some(codes.code_named(Self::symbol_name(value)?, value.span())?),
            ..self
        })
    }

    fn with_earlier(self, part: &Form, value: &Form) -> Self {
        Self {
            earlier: Some(Earlier {
                form: value.span(),
                part: part.span(),
            }),
            ..self
        }
    }

    fn with_earlier_label(self, part: &Form, value: &Form) -> Result<Self, Diagnostic> {
        Ok(Self {
            earlier_label: Some(EarlierLabel {
                text: Self::label_text(value)?,
                part: part.span(),
            }),
            ..self
        })
    }

    fn with_label(self, value: &Form) -> Result<Self, Diagnostic> {
        Ok(Self {
            label: Some(Self::label_text(value)?),
            ..self
        })
    }

    fn with_note(self, value: &Form) -> Result<Self, Diagnostic> {
        Ok(Self {
            note: Some(Self::label_text(value)?),
            ..self
        })
    }

    fn with_replacement(self, value: &Form) -> Result<Self, Diagnostic> {
        Ok(Self {
            replacement: Some(Self::symbol_name(value)?.to_string()),
            ..self
        })
    }

    fn part_name(part: &Form) -> Option<&str> {
        part.as_symbol()
            .and_then(|symbol| Keyword::field_name(symbol.name()))
    }

    fn symbol_name(value: &Form) -> Result<&str, Diagnostic> {
        value
            .as_symbol()
            .map(|symbol| symbol.name())
            .ok_or_else(|| Self::malformed_part(value))
    }

    fn label_text(value: &Form) -> Result<String, Diagnostic> {
        match value.kind() {
            FormKind::String(text) => Ok(text.to_string()),
            _ => Err(Self::malformed_part(value)),
        }
    }

    fn malformed_part(part: &Form) -> Diagnostic {
        InterpretProblems::malformed_failure_part(part.span(), &Self::PARTS)
    }

    pub fn diagnostic(
        mut self,
        message: String,
        culprit: Span,
        help: Option<String>,
    ) -> Diagnostic {
        let replacement = self.replacement.take();
        let note = self.note.take();
        let labelled = self.labelled(message, culprit);
        let failure = match note {
            Some(note) => labelled.with_note(note),
            None => labelled,
        };
        match (help, replacement) {
            (Some(help), Some(replacement)) => {
                failure.with_suggestion(help, Suggestion::new(culprit, replacement))
            }
            (Some(help), None) => failure.with_help(help),
            (None, _) => failure,
        }
    }

    fn labelled(self, message: String, culprit: Span) -> Diagnostic {
        let earlier = self
            .earlier
            .zip(self.earlier_label)
            .map(|(earlier, label)| (earlier.form, label.text));
        let code = self.code.unwrap_or(DiagnosticCode::MACRO_FAILED);
        InterpretProblems::failed(code, message, culprit, self.label, earlier)
    }
}

impl Default for FailureDescription {
    fn default() -> Self {
        Self {
            code: None,
            label: None,
            earlier: None,
            earlier_label: None,
            replacement: None,
            note: None,
        }
    }
}
