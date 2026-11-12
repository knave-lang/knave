use rustc_hash::FxHashMap;

use super::unreadable_span::UnreadableSpan;

#[derive(Default)]
pub struct DocstringMemo {
    tokens: FxHashMap<String, LaidOutToken>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaidOutToken {
    pub text: String,
    pub unreadable: Vec<UnreadableSpan>,
}

impl DocstringMemo {
    pub fn laid_out(
        &mut self,
        token: &str,
        lay_out: impl FnOnce() -> LaidOutToken,
    ) -> &LaidOutToken {
        self.tokens.entry(token.to_string()).or_insert_with(lay_out)
    }
}

#[cfg(test)]
mod tests {
    use super::{DocstringMemo, LaidOutToken};

    struct MemoFixture;

    impl MemoFixture {
        fn token(text: &str) -> LaidOutToken {
            LaidOutToken {
                text: text.to_string(),
                unreadable: Vec::new(),
            }
        }
    }

    #[test]
    fn a_token_laid_out_once_is_not_laid_out_again() {
        let mut memo = DocstringMemo::default();
        memo.laid_out("\"a\"", || MemoFixture::token("\"first\""));
        let again = memo.laid_out("\"a\"", || MemoFixture::token("\"second\""));
        assert_eq!(again.text, "\"first\"");
    }

    #[test]
    fn another_token_is_laid_out_on_its_own() {
        let mut memo = DocstringMemo::default();
        memo.laid_out("\"a\"", || MemoFixture::token("\"first\""));
        let other = memo.laid_out("\"b\"", || MemoFixture::token("\"second\""));
        assert_eq!(other.text, "\"second\"");
    }
}
