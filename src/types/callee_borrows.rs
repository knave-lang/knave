use super::templates::TemplateBorrows;
use super::typed::TypedExpr;

#[derive(Clone, Copy)]
pub(super) struct CalleeBorrows<'a> {
    callee: Option<&'a TypedExpr>,
    templates: &'a TemplateBorrows,
}

impl<'a> CalleeBorrows<'a> {
    pub(super) fn new(callee: Option<&'a TypedExpr>, templates: &'a TemplateBorrows) -> Self {
        Self { callee, templates }
    }

    pub(super) fn borrows_at(&self, position: usize) -> bool {
        self.callee
            .is_some_and(|callee| self.templates.borrows(callee, position))
    }
}

#[cfg(test)]
mod tests {
    use crate::types::passing_fixture::PassingFixture;

    #[test]
    fn a_template_borrows_a_marked_position() {
        let fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        assert!(fixture.callee_borrows(Some(&callee)).borrows_at(1));
    }

    #[test]
    fn a_template_does_not_borrow_an_unmarked_position() {
        let fixture = PassingFixture::new();
        let callee = PassingFixture::template_call(PassingFixture::BORROWING_SECOND);
        assert!(!fixture.callee_borrows(Some(&callee)).borrows_at(0));
    }
}
