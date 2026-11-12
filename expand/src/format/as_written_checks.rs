use super::docstring_line::DocstringLine;
use super::docstring_syntax::DocstringSyntax;
use crate::reader::{TokenKind, TokenScan};

pub struct AsWrittenChecks<'s> {
    syntax: &'s DocstringSyntax,
}

impl<'s> AsWrittenChecks<'s> {
    pub fn new(syntax: &'s DocstringSyntax) -> Self {
        Self { syntax }
    }

    pub fn holds_a_multi_line_string(&self, lines: &[DocstringLine]) -> bool {
        let text = lines
            .iter()
            .map(DocstringLine::text)
            .collect::<Vec<_>>()
            .join("\n");
        TokenScan::new(&text)
            .any(|token| token.kind == TokenKind::String && text[token.range].contains('\n'))
    }

    pub fn writes_a_fence_line(&self, laid_out: &[String]) -> bool {
        laid_out
            .iter()
            .any(|line| self.syntax.opening_fence(line).is_some())
    }

    pub fn writes_an_arrow_line(&self, laid_out: &[String]) -> bool {
        laid_out.iter().any(|line| self.syntax.is_arrow_line(line))
    }
}

#[cfg(test)]
mod tests {
    use super::AsWrittenChecks;
    use crate::format::docstring_line::DocstringLine;
    use crate::format::docstring_syntax::DocstringSyntaxFixture;

    struct ChecksFixture;

    impl ChecksFixture {
        fn laid_out(text: &str) -> Vec<String> {
            text.split('\n').map(str::to_string).collect()
        }
    }

    #[test]
    fn a_string_over_two_lines_is_a_multi_line_string() {
        let syntax = DocstringSyntaxFixture::syntax();
        let checks = AsWrittenChecks::new(&syntax);
        assert_eq!(
            (
                checks.holds_a_multi_line_string(&DocstringLine::split("(f \"a\nb\")")),
                checks.holds_a_multi_line_string(&DocstringLine::split("(f \"a\")\n(g 1)"))
            ),
            (true, false)
        );
    }

    #[test]
    fn a_laid_out_line_that_opens_a_fence_writes_a_fence_line() {
        let syntax = DocstringSyntaxFixture::syntax();
        let checks = AsWrittenChecks::new(&syntax);
        assert_eq!(
            (
                checks.writes_a_fence_line(&ChecksFixture::laid_out("(f\n  ~~~)")),
                checks.writes_a_fence_line(&ChecksFixture::laid_out("(f 1)"))
            ),
            (true, false)
        );
    }

    #[test]
    fn a_laid_out_line_that_starts_with_the_arrow_writes_an_arrow_line() {
        let syntax = DocstringSyntaxFixture::syntax();
        let checks = AsWrittenChecks::new(&syntax);
        assert_eq!(
            (
                checks.writes_an_arrow_line(&ChecksFixture::laid_out("(f a\n   => b)")),
                checks.writes_an_arrow_line(&ChecksFixture::laid_out("(f a => b)"))
            ),
            (true, false)
        );
    }
}
