use super::docstring_line::DocstringLine;
use super::docstring_part::DocstringPart;
use super::docstring_regions::DocstringRegions;
use super::docstring_syntax::DocstringSyntax;
use super::import_order::ImportList;
use super::shapes::ShapeCatalog;
use crate::reader::{Form, FormKind, TokenKind, TokenScan};
use crate::stack::DeepRecursion;

pub struct MeaningComparison<'s> {
    shapes: &'s ShapeCatalog,
    syntax: &'s DocstringSyntax,
}

struct Docstring<'t> {
    syntax: &'t DocstringSyntax,
    text: &'t str,
}

#[derive(Debug, PartialEq, Eq)]
enum ComparedPart {
    Prose(String),
    Code(Vec<String>),
}

impl<'s> MeaningComparison<'s> {
    pub fn new(shapes: &'s ShapeCatalog, syntax: &'s DocstringSyntax) -> Self {
        Self { shapes, syntax }
    }

    pub fn same_forms(&self, written: &[Form], formatted: &[Form]) -> bool {
        written.len() == formatted.len()
            && written
                .iter()
                .zip(formatted)
                .all(|(left, right)| self.same_top_level_form(left, right))
    }

    fn same_top_level_form(&self, left: &Form, right: &Form) -> bool {
        match (left.as_list(), right.as_list()) {
            (Some(left), Some(right)) => {
                DeepRecursion::guard(|| self.same_top_level_lists(left, right))
            }
            _ => self.same_form(left, right, false),
        }
    }

    fn same_top_level_lists(&self, left: &[Form], right: &[Form]) -> bool {
        let head = left
            .first()
            .and_then(Form::as_symbol)
            .map(|head| head.name());
        let import_start = head
            .and_then(ImportList::names_start)
            .filter(|start| *start <= left.len());
        if left.len() != right.len() {
            false
        } else if head.is_some_and(ImportList::splices) {
            left.iter()
                .zip(right)
                .all(|(left, right)| self.same_top_level_form(left, right))
        } else if let Some(start) = import_start {
            self.same_in_order(&left[..start], &right[..start], None)
                && self.same_as_sets(&left[start..], &right[start..])
        } else {
            self.same_lists(left, right)
        }
    }

    pub fn same_comments(&self, written: &str, formatted: &str) -> bool {
        Self::comment_texts(written) == Self::comment_texts(formatted)
    }

    fn comment_texts(text: &str) -> Vec<&str> {
        TokenScan::new(text)
            .filter(|token| token.kind == TokenKind::Comment)
            .map(|token| &text[token.range])
            .collect()
    }

    fn same_form(&self, left: &Form, right: &Form, is_docstring: bool) -> bool {
        match (left.kind(), right.kind()) {
            (FormKind::Symbol(left), FormKind::Symbol(right)) => left.name() == right.name(),
            (FormKind::Number(left), FormKind::Number(right)) => left.text() == right.text(),
            (FormKind::String(left), FormKind::String(right)) if is_docstring => {
                self.docstring(left).parts() == self.docstring(right).parts()
            }
            (FormKind::String(left), FormKind::String(right)) => left == right,
            (FormKind::List(left), FormKind::List(right)) => {
                DeepRecursion::guard(|| self.same_lists(left, right))
            }
            _ => false,
        }
    }

    fn docstring<'t>(&'t self, text: &'t str) -> Docstring<'t> {
        Docstring {
            syntax: self.syntax,
            text,
        }
    }

    fn same_lists(&self, left: &[Form], right: &[Form]) -> bool {
        left.len() == right.len() && self.same_in_order(left, right, self.docstring_position(left))
    }

    fn same_in_order(&self, left: &[Form], right: &[Form], docstring: Option<usize>) -> bool {
        left.iter()
            .zip(right)
            .enumerate()
            .all(|(index, (left, right))| self.same_form(left, right, docstring == Some(index)))
    }

    fn same_as_sets(&self, left: &[Form], right: &[Form]) -> bool {
        let sorted = |forms: &[Form]| {
            let mut sorted: Vec<Form> = forms.to_vec();
            sorted.sort_by_key(Form::to_string);
            sorted
        };
        self.same_in_order(&sorted(left), &sorted(right), None)
    }

    fn docstring_position(&self, items: &[Form]) -> Option<usize> {
        let head = items.first()?.as_symbol()?;
        let position = self.shapes.docstring_position(head.name(), items.len())?;
        let is_string = matches!(items.get(position)?.kind(), FormKind::String(_));
        is_string.then_some(position)
    }
}

impl Docstring<'_> {
    fn parts(&self) -> Vec<ComparedPart> {
        let lines = DocstringLine::split(&self.dedented());
        DocstringRegions::parts_of(self.syntax, &lines)
            .into_iter()
            .flat_map(|part| match part {
                DocstringPart::Prose(range) => lines[range]
                    .iter()
                    .map(|line| ComparedPart::Prose(line.text().to_string()))
                    .collect(),
                DocstringPart::Example(range) | DocstringPart::Fence(range) => {
                    vec![ComparedPart::Code(Self::tokens(&Self::joined(
                        &lines[range],
                    )))]
                }
            })
            .collect()
    }

    fn joined(lines: &[DocstringLine]) -> String {
        lines
            .iter()
            .map(DocstringLine::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn tokens(code: &str) -> Vec<String> {
        TokenScan::new(code)
            .map(|token| code[token.range].to_string())
            .collect()
    }

    fn dedented(&self) -> String {
        let mut lines = self.text.split('\n');
        let first = lines.next().unwrap_or_default();
        let later: Vec<&str> = lines.collect();
        let shared = Self::shared_indentation(&later);
        let mut dedented = first.to_string();
        for line in later {
            dedented.push('\n');
            dedented.push_str(Self::without_indentation(line, shared));
        }
        dedented
    }

    fn shared_indentation(lines: &[&str]) -> usize {
        lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| Self::indentation(line))
            .min()
            .unwrap_or(0)
    }

    fn without_indentation(line: &str, shared: usize) -> &str {
        &line[Self::indentation(line).min(shared)..]
    }

    fn indentation(line: &str) -> usize {
        line.len() - line.trim_start_matches(' ').len()
    }
}

#[cfg(test)]
mod tests {
    use super::{ComparedPart, Docstring};
    use crate::format::docstring_syntax::{DocstringSyntax, DocstringSyntaxFixture};

    struct DocstringFixture {
        syntax: DocstringSyntax,
    }

    impl DocstringFixture {
        fn new() -> Self {
            Self {
                syntax: DocstringSyntaxFixture::syntax(),
            }
        }

        fn docstring<'t>(&'t self, text: &'t str) -> Docstring<'t> {
            Docstring {
                syntax: &self.syntax,
                text,
            }
        }
    }

    #[test]
    fn dedenting_removes_the_indentation_that_later_lines_share() {
        let fixture = DocstringFixture::new();
        let docstring = fixture.docstring("Doubles.\n\n    Works on\n      any number.");
        assert_eq!(docstring.dedented(), "Doubles.\n\nWorks on\n  any number.");
    }

    #[test]
    fn laid_out_code_keeps_the_parts_of_a_docstring() {
        let fixture = DocstringFixture::new();
        let written = fixture.docstring("Pads.\n\n  (pad   1)\n  => 1");
        let laid_out = fixture.docstring("Pads.\n\n  (pad 1)\n  => 1");
        assert_eq!(written.parts(), laid_out.parts());
    }

    #[test]
    fn moved_prose_changes_the_parts_of_a_docstring() {
        let fixture = DocstringFixture::new();
        let written = fixture.docstring("Pads.\n\n  Some   prose.");
        let laid_out = fixture.docstring("Pads.\n\n  Some prose.");
        assert_ne!(written.parts(), laid_out.parts());
    }

    #[test]
    fn changed_code_changes_the_parts_of_a_docstring() {
        let fixture = DocstringFixture::new();
        let written = fixture.docstring("Pads.\n\n  (pad 1)\n  => 1");
        let laid_out = fixture.docstring("Pads.\n\n  (pad 2)\n  => 1");
        assert_ne!(written.parts(), laid_out.parts());
        assert!(matches!(written.parts()[2], ComparedPart::Code(_)));
    }
}
