use super::docstring_code::DocstringCode;
use super::docstring_lines::DocstringLines;
use super::format_rules::FormatRules;
use super::unreadable_span::UnreadableSpan;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaidOutToken {
    pub text: String,
    pub unreadable: Vec<UnreadableSpan>,
}

impl LaidOutToken {
    pub fn of(rules: FormatRules<'_>, token: &str) -> Self {
        let Some(lines) = DocstringLines::of_token(token) else {
            return Self {
                text: token.to_string(),
                unreadable: Vec::new(),
            };
        };
        let laid_out = DocstringCode::of_docstrings(rules).lay_out(lines.lines());
        Self {
            text: DocstringLines::token_of(&laid_out.lines),
            unreadable: laid_out
                .unreadable
                .iter()
                .map(|code| code.spanning(lines.written_range(code.line)))
                .collect(),
        }
    }
}
