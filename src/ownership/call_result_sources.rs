use super::callee_borrows::CalleeBorrows;
use super::result_source::ResultSource;
use crate::types::{PassedCall, TypedExpr, TypedProgram};

pub struct CallResultSources<'p> {
    program: &'p TypedProgram,
    callees: CalleeBorrows<'p>,
}

impl<'p> CallResultSources<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            callees: CalleeBorrows::new(program),
        }
    }

    pub fn of<'e>(&self, call: &'e TypedExpr) -> Option<Vec<ResultSource<'e>>> {
        let PassedCall { callee, arguments } = PassedCall::of(call)?;
        let ties = self.callees.result_ties(callee)?;
        let sources = arguments
            .iter()
            .enumerate()
            .filter_map(|(position, argument)| {
                let tie = ties.tie_at(position)?;
                Some(ResultSource::kept_by_call(
                    tie,
                    &argument.value,
                    self.program,
                ))
            })
            .collect();
        Some(sources)
    }
}
