use crate::types::{PositionTie, Type, TypedExpr, TypedProgram};

pub struct ResultSource<'e> {
    pub tie: PositionTie,
    pub argument: &'e TypedExpr,
}

pub struct PassedArgument<'t, 'e> {
    pub parameter: &'t Type,
    pub argument: &'e TypedExpr,
}

impl<'e> ResultSource<'e> {
    pub fn kept_by_closure(passed: PassedArgument<'_, 'e>) -> Self {
        let tie = if passed.parameter.is_reference() {
            PositionTie::Lent
        } else {
            PositionTie::Held
        };
        Self {
            tie,
            argument: passed.argument,
        }
    }

    pub fn kept_by_call(
        tie: PositionTie,
        passed: PassedArgument<'_, 'e>,
        program: &TypedProgram,
    ) -> Self {
        let lends_itself = passed.parameter.is_reference()
            && !program.type_holds_borrow(passed.argument.ty.referent());
        let tie = match tie {
            PositionTie::Lent if !lends_itself => PositionTie::Held,
            tie => tie,
        };
        Self {
            tie,
            argument: passed.argument,
        }
    }
}
