pub(super) struct WrittenBorrow<'t> {
    preceding: &'t str,
}

impl<'t> WrittenBorrow<'t> {
    const BORROW: char = '&';
    const OPENERS: &'static str = "([{,;=";

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
