use crate::types::{CopyRules, Type, TypedExpr, TypedProgram};

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

    pub fn moves(&self, value: &TypedExpr) -> bool {
        !self.copy.is_copy(&value.ty)
    }

    pub fn is_template(&self, callee: &TypedExpr) -> bool {
        self.program
            .template_borrows
            .read_template(callee, false, |_| true)
    }

    pub fn holds_a_borrow(&self, ty: &Type) -> bool {
        self.program.type_holds_borrow(ty)
    }
}
