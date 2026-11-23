use super::Type;
use super::typed::TypedProgram;

pub struct RecordedClosures<'p> {
    program: &'p TypedProgram,
}

impl<'p> RecordedClosures<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self { program }
    }

    pub fn includes(&self, closure_type: &Type) -> bool {
        matches!(
            closure_type,
            Type::Function(_, _, result) if self.program.function_type_returns_borrow(result)
        )
    }
}
