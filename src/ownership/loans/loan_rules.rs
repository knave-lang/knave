use crate::types::{
    BorrowedIndex, BorrowingTypes, CopyRules, TemplateTies, Type, TypedExpr, TypedExprKind,
    TypedProgram, WrittenBorrows,
};

pub struct LoanRules<'p> {
    program: &'p TypedProgram,
    copy: CopyRules<'p>,
}

impl<'p> LoanRules<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            copy: CopyRules::new(&program.declarations, &program.generics),
        }
    }

    pub fn moves(&self, ty: &Type) -> bool {
        !self.copy.is_copy(ty)
    }

    pub fn ties(&self, callee: &TypedExpr) -> Option<TemplateTies> {
        self.program
            .template_borrows
            .read_template(callee, None, |_, ties| ties.cloned())
    }

    pub fn copies_out(&self, value: &TypedExpr, levels: usize) -> bool {
        WrittenBorrows::under(value, levels).is_some()
            || (levels == 1 && self.indexes_in_place(value))
    }

    fn indexes_in_place(&self, value: &TypedExpr) -> bool {
        let TypedExprKind::Call(callee, _) = &value.kind else {
            return false;
        };
        self.program
            .template_borrows
            .read_template(callee, false, |parsed, _| {
                BorrowedIndex::place(parsed.text()).is_some()
            })
    }

    pub fn holds_a_borrow(&self, ty: &Type) -> bool {
        BorrowingTypes::of(self.program).keeps_borrow(ty)
    }
}
