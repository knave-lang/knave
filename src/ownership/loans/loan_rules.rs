use crate::types::{BorrowingTypes, CopyRules, TemplateTies, Type, TypedExpr, TypedProgram};

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

    pub fn holds_a_borrow(&self, ty: &Type) -> bool {
        BorrowingTypes::of(self.program).keeps_borrow(ty)
    }
}
