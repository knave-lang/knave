use crate::types::{PositionTie, Type, TypedExpr, TypedProgram};

pub struct ResultSource<'e> {
    tie: PositionTie,
    argument: &'e TypedExpr,
}

impl<'e> ResultSource<'e> {
    pub fn kept_by_closure(parameter: &Type, argument: &'e TypedExpr) -> Self {
        let tie = if parameter.is_reference() {
            PositionTie::Lent
        } else {
            PositionTie::Held
        };
        Self { tie, argument }
    }

    pub fn kept_by_call(tie: PositionTie, argument: &'e TypedExpr, program: &TypedProgram) -> Self {
        let lends_itself = !program.type_holds_borrow(argument.ty.referent());
        let tie = match tie {
            PositionTie::Lent if !lends_itself => PositionTie::Held,
            tie => tie,
        };
        Self { tie, argument }
    }

    pub fn tie(&self) -> PositionTie {
        self.tie
    }

    pub fn argument(&self) -> &'e TypedExpr {
        self.argument
    }
}
