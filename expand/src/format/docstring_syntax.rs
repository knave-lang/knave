use super::markdown_fence::MarkdownFence;
use crate::core::{Keyword, ModulePath};
use crate::expand::PreludeState;
use crate::modules::ModuleSources;
use crate::reader::{Form, FormKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocstringSyntax {
    arrow: String,
    fence_marks: Vec<char>,
    fence_length: usize,
}

impl DocstringSyntax {
    const NOTE: &'static str = "docstring-syntax";
    const ARROW: &'static str = "arrow";
    const FENCE_MARKS: &'static str = "fence-marks";
    const FENCE_LENGTH: &'static str = "fence-length";

    pub fn of_prelude(state: &PreludeState) -> Self {
        let module = ModulePath::new(ModuleSources::DOCSTRINGS);
        let note = state
            .module_note(&module, Self::NOTE)
            .and_then(<[Form]>::first)
            .and_then(Form::as_list)
            .expect("the prelude notes its docstring syntax");
        Self {
            arrow: Self::text(Self::field(note, Self::ARROW)),
            fence_marks: Self::field(note, Self::FENCE_MARKS)
                .as_list()
                .expect("the fence marks are a list")
                .iter()
                .filter_map(|mark| Self::text(mark).chars().next())
                .collect(),
            fence_length: Self::count(Self::field(note, Self::FENCE_LENGTH)),
        }
    }

    pub fn is_arrow_line(&self, line: &str) -> bool {
        line.trim_start().starts_with(&self.arrow)
    }

    pub fn opening_fence(&self, line: &str) -> Option<MarkdownFence> {
        let text = line.trim_start();
        let mark = self
            .fence_marks
            .iter()
            .copied()
            .find(|mark| text.starts_with(*mark))?;
        let fence = MarkdownFence::of_run(text, mark);
        (fence.length() >= self.fence_length).then_some(fence)
    }

    fn field<'n>(note: &'n [Form], name: &str) -> &'n Form {
        note.chunks(2)
            .find_map(|pair| match pair {
                [key, value] => {
                    let key = key.as_symbol()?.name();
                    (Keyword::field_name(key) == Some(name)).then_some(value)
                }
                _ => None,
            })
            .expect("the docstring syntax note holds each field")
    }

    fn text(form: &Form) -> String {
        match form.kind() {
            FormKind::String(text) => text.to_string(),
            _ => panic!("a docstring syntax field holds text"),
        }
    }

    fn count(form: &Form) -> usize {
        match form.kind() {
            FormKind::Number(number) => number.text().parse().ok(),
            _ => None,
        }
        .expect("the fence length is a count")
    }
}

#[cfg(test)]
use std::sync::OnceLock;

#[cfg(test)]
pub struct DocstringSyntaxFixture;

#[cfg(test)]
impl DocstringSyntaxFixture {
    pub fn syntax() -> DocstringSyntax {
        static SYNTAX: OnceLock<DocstringSyntax> = OnceLock::new();
        SYNTAX
            .get_or_init(|| {
                DocstringSyntax::of_prelude(
                    &PreludeState::load(PreludeState::build().save().leak())
                        .expect("the saved prelude loads"),
                )
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::DocstringSyntaxFixture;

    #[test]
    fn an_arrow_line_starts_with_the_arrow_and_its_space() {
        let syntax = DocstringSyntaxFixture::syntax();
        assert_eq!(
            (
                syntax.is_arrow_line("  => 6"),
                syntax.is_arrow_line("=>"),
                syntax.is_arrow_line("=>6")
            ),
            (true, false, false)
        );
    }

    #[test]
    fn a_fence_opens_with_three_backticks_or_tildes() {
        let syntax = DocstringSyntaxFixture::syntax();
        assert_eq!(
            (
                syntax.opening_fence("```").is_some(),
                syntax.opening_fence("  ~~~~ knave").is_some(),
                syntax.opening_fence("``").is_some(),
                syntax.opening_fence("---").is_some()
            ),
            (true, true, false, false)
        );
    }
}
