use super::pipelines::Pipelines;
use super::ty::Type;
use super::typed::TypedProgram;

pub struct BorrowingTypes<'p> {
    program: &'p TypedProgram,
    pipelines: Pipelines<'p>,
}

impl<'p> BorrowingTypes<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            pipelines: Pipelines::new(program.declarations.types().rust_types()),
        }
    }

    pub fn keeps_borrow(&self, ty: &Type) -> bool {
        self.program.type_holds_borrow(ty) || self.pipelines.is_pipeline(ty)
    }
}
