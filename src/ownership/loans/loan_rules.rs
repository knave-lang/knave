use crate::types::{CopyRules, TemplateBorrows, Type, TypedExpr, TypedProgram};

pub struct LoanRules<'p> {
    program: &'p TypedProgram,
    templates: &'p TemplateBorrows,
    copy: CopyRules<'p>,
}

impl<'p> LoanRules<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            templates: &program.template_borrows,
            copy: CopyRules::new(&program.declarations, &program.generics),
        }
    }

    pub fn moves(&self, value: &TypedExpr) -> bool {
        !self.copy.is_copy(&value.ty)
    }

    pub fn is_template(&self, callee: &TypedExpr) -> bool {
        self.templates.read_template(callee, false, |_| true)
    }

    pub fn holds_a_borrow(&self, ty: &Type) -> bool {
        self.program.type_holds_borrow(ty)
    }
}
