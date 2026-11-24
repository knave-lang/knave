use super::Type;
use super::typed::TypedProgram;

pub struct LendingTypes<'p> {
    program: &'p TypedProgram,
}

impl<'p> LendingTypes<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn lends_borrow(&self, ty: &Type) -> bool {
        ty.is_reference() || self.program.type_holds_borrow(ty)
    }
}
