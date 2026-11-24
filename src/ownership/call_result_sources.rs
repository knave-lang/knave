use super::recorded_ties::RecordedTies;
use super::result_source::ResultSource;
use crate::types::{DefinitionTable, TypedExpr, TypedExprKind, TypedProgram};

pub struct CallResultSources<'p> {
    program: &'p TypedProgram,
    ties: RecordedTies<'p, 'p>,
}

impl<'p> CallResultSources<'p> {
    pub fn new(program: &'p TypedProgram, definitions: &'p DefinitionTable<'p>) -> Self {
        Self {
            program,
            ties: RecordedTies::new(program, definitions),
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
