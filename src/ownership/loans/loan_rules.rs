use crate::types::{CopyRules, Pipelines, TemplateTies, Type, TypedExpr, TypedProgram};

pub struct LoanRules<'p> {
    program: &'p TypedProgram,
    copy: CopyRules<'p>,
    pipelines: Pipelines<'p>,
}

impl<'p> LoanRules<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            copy: CopyRules::new(&program.declarations, &program.generics),
            pipelines: Pipelines::new(program.declarations.types().rust_types()),
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
        self.program.type_holds_borrow(ty) || self.pipelines.is_pipeline(ty)
    }

    pub fn adapts(&self, parameter: &Type, result: &Type) -> bool {
        self.pipelines.is_pipeline(parameter) && self.pipelines.is_pipeline(result)
    }
}
