use crate::types::{CopyRules, TemplateTies, Type, TypedExpr, TypedProgram};

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
            .read_template(callee, None, |template| template.ties())
    }

    pub fn holds_a_borrow(&self, ty: &Type) -> bool {
        self.program.type_holds_borrow(ty)
    }
}
