use super::access::Access;

pub(super) struct WrittenBorrow<'t> {
    preceding: &'t str,
}

impl<'t> WrittenBorrow<'t> {
    const OPENERS: &'static str = "([{,;:|=+-*/%<>!^";
    const KEYWORDS: [&'static str; 6] = ["else", "if", "in", "match", "return", "while"];

    pub(super) fn after(preceding: &'t str) -> Self {
        Self { preceding }
    }

    pub(super) fn borrows_at(&self, position: usize) -> bool {
        let Some(before) = self.preceding.strip_suffix(Access::BORROW) else {
            return false;
        };
        let before = before.trim_end();
        match before.chars().last() {
            Some(last) if last.is_alphanumeric() => Self::ends_with_keyword(before),
            Some(last) => Self::OPENERS.contains(last),
            None => position == 0,
        }
    }

    fn ends_with_keyword(before: &str) -> bool {
        let word = before
            .rsplit(|character: char| !character.is_alphanumeric())
            .next();
        word.is_some_and(|word| Self::KEYWORDS.contains(&word))
    }
}

#[cfg(test)]
mod tests {
    use super::WrittenBorrow;

    #[test]
    fn an_ampersand_that_opens_the_template_borrows() {
        assert!(WrittenBorrow::after("&").borrows_at(0));
    }

    #[test]
    fn an_ampersand_after_a_placeholder_is_an_operator() {
        assert!(!WrittenBorrow::after("&").borrows_at(1));
        assert!(!WrittenBorrow::after(" &&").borrows_at(1));
    }

    #[test]
    fn an_ampersand_after_an_opener_or_an_operator_borrows() {
        for preceding in [
            "f(&",
            "f(a, &",
            "x = &",
            "x + &",
            "a < &",
            "x => &",
            "S { f: &",
            "|x| &",
            "match &",
            "for x in &",
        ] {
            assert!(WrittenBorrow::after(preceding).borrows_at(1), "{preceding}");
        }
    }

    #[test]
    fn an_ampersand_after_a_name_is_an_operator() {
        assert!(!WrittenBorrow::after("bits &").borrows_at(1));
    }

    #[test]
    fn text_without_an_ampersand_borrows_nothing() {
        assert!(!WrittenBorrow::after("f(").borrows_at(0));
    }
}
