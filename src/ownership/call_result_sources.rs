use super::result_source::ResultSource;
use super::result_ties::ResultTies;
use crate::types::{TypedExpr, TypedExprKind, TypedProgram};

pub struct CallResultSources<'p> {
    program: &'p TypedProgram,
    ties: ResultTies<'p>,
}

impl<'p> CallResultSources<'p> {
    pub fn new(program: &'p TypedProgram) -> Self {
        Self {
            program,
            ties: ResultTies::new(program),
        }
    }

    pub fn of<'e>(&self, call: &'e TypedExpr) -> Option<Vec<ResultSource<'e>>> {
        let TypedExprKind::Call(callee, arguments) = &call.kind else {
            return None;
        };
        let ties = self.ties.of(callee)?;
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
