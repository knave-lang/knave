use super::result_source::ResultSource;
use super::result_ties::ResultTies;
use crate::types::{TypedArgument, TypedExpr, TypedExprKind, TypedProgram};

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
        match &call.kind {
            TypedExprKind::Call(callee, arguments) => self.kept_by(callee, arguments),
            TypedExprKind::Assign(assignment) => {
                self.kept_by(&assignment.template, &assignment.arguments)
            }
            _ => None,
        }
    }

    fn kept_by<'e>(
        &self,
        callee: &TypedExpr,
        arguments: &'e [TypedArgument],
    ) -> Option<Vec<ResultSource<'e>>> {
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
