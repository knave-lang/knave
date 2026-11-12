use rustc_hash::FxHashMap;

use super::format_rules::FormatRules;
use super::laid_out_token::LaidOutToken;

pub struct DocstringLayouts<'s> {
    rules: FormatRules<'s>,
    tokens: FxHashMap<String, LaidOutToken>,
}

impl<'s> DocstringLayouts<'s> {
    pub fn new(rules: FormatRules<'s>) -> Self {
        Self {
            rules,
            tokens: FxHashMap::default(),
        }
    }

    pub fn rules(&self) -> FormatRules<'s> {
        self.rules
    }

    pub fn of_token(&mut self, token: &str) -> &LaidOutToken {
        let rules = self.rules;
        self.tokens
            .entry(token.to_string())
            .or_insert_with(|| LaidOutToken::of(rules, token))
    }
}

#[cfg(test)]
mod tests {
    use super::DocstringLayouts;
    use crate::format::docstring_syntax::{DocstringSyntax, DocstringSyntaxFixture};
    use crate::format::format_rules::FormatRules;
    use crate::format::shapes::ShapeCatalog;

    struct LayoutsFixture {
        shapes: ShapeCatalog,
        syntax: DocstringSyntax,
    }

    impl LayoutsFixture {
        fn new() -> Self {
            Self {
                shapes: ShapeCatalog::default(),
                syntax: DocstringSyntaxFixture::syntax(),
            }
        }

        fn layouts(&self) -> DocstringLayouts<'_> {
            DocstringLayouts::new(FormatRules {
                shapes: &self.shapes,
                syntax: &self.syntax,
            })
        }
    }

    #[test]
    fn a_token_lays_out_its_docstring_code() {
        let fixture = LayoutsFixture::new();
        let mut layouts = fixture.layouts();
        assert_eq!(
            layouts.of_token("\"Pads.\n\n  (pad   1)\n  => 1\"").text,
            "\"Pads.\n\n  (pad 1)\n  => 1\""
        );
    }

    #[test]
    fn a_token_asked_twice_gets_the_same_layout() {
        let fixture = LayoutsFixture::new();
        let mut layouts = fixture.layouts();
        let first = layouts.of_token("\"Pads.\n\n  (pad   1)\n  => 1\"").clone();
        assert_eq!(layouts.of_token("\"Pads.\n\n  (pad   1)\n  => 1\""), &first);
    }
}
