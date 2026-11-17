pub(super) struct WrittenBorrow<'t> {
    preceding: &'t str,
}

impl<'t> WrittenBorrow<'t> {
    const BORROW: char = '&';
    const OPENERS: &'static str = "([{,;=+-*/%<>!^";

    pub(super) fn after(preceding: &'t str) -> Self {
        Self { preceding }
    }

    pub(super) fn borrows_at(&self, position: usize) -> bool {
        let Some(before) = self.preceding.strip_suffix(Self::BORROW) else {
            return false;
        };
        match before.trim_end().chars().last() {
            Some(last) => Self::OPENERS.contains(last),
            None => position == 0,
        }
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
        for preceding in ["f(&", "f(a, &", "x = &", "x + &", "a < &", "x => &"] {
            assert!(WrittenBorrow::after(preceding).borrows_at(1), "{preceding}");
        }
    }

    #[test]
    fn text_without_an_ampersand_borrows_nothing() {
        assert!(!WrittenBorrow::after("f(").borrows_at(0));
    }
}
