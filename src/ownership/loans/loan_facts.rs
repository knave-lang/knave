use super::body_walk::BodyWalk;
use super::loan_context::LoanContext;
use super::loan_names::LoanNames;
use crate::source::SourceMap;
use crate::types::{TypedExprKind, TypedLambda, TypedProgram};

pub struct LoanFacts<'p> {
    program: &'p TypedProgram,
    context: LoanContext<'p>,
}

impl<'p> LoanFacts<'p> {
    pub fn of(program: &'p TypedProgram) -> Self {
        Self {
            program,
            context: LoanContext::of(program),
        }
    }

    pub fn report(&self, sources: &SourceMap) -> String {
        let names = LoanNames::new(&self.program.bindings, sources);
        self.functions()
            .map(|(name, lambda)| self.body_report(name.to_string(), lambda, &names))
            .collect()
    }

    fn functions(&self) -> impl Iterator<Item = (&'p str, &'p TypedLambda)> + '_ {
        self.program
            .modules
            .iter()
            .filter(|module| module.id == self.program.entry)
            .flat_map(|module| &module.definitions)
            .filter_map(|definition| match &definition.value.kind {
                TypedExprKind::Fn(lambda) => Some((
                    self.program.bindings.get(definition.binding).name(),
                    lambda.as_ref(),
                )),
                _ => None,
            })
    }

    fn body_report(
        &self,
        heading: String,
        lambda: &'p TypedLambda,
        names: &LoanNames<'_>,
    ) -> String {
        let walked = BodyWalk::new(&self.context).walk(lambda);
        let accesses: String = walked
            .accesses()
            .iter()
            .map(|access| format!("  {}\n", access.written(names)))
            .collect();
        let closures: String = walked
            .closures()
            .iter()
            .filter_map(|closure| match &closure.kind {
                TypedExprKind::Fn(lambda) => {
                    let heading = format!("closure at {}", names.position(closure.span));
                    Some(self.body_report(heading, lambda, names))
                }
                _ => None,
            })
            .collect();
        let own = if accesses.is_empty() {
            String::new()
        } else {
            format!("{heading}\n{accesses}")
        };
        own + &closures
    }
}

#[cfg(test)]
mod tests {
    use crate::ownership::loans::loan_source::LoanSource;

    #[test]
    fn reports_a_loan_held_by_a_local_across_a_change() {
        let source = LoanSource::new(
            "(defn main ()\n  (local v (vec 1 2))\n  (local r (at v 0))\n  (push! v 4)\n  (println r))",
        );
        assert_eq!(
            source.line_of("change `v`"),
            "4:10 change `v`: `v` lent at 3:16, held by `r`"
        );
    }

    #[test]
    fn heads_each_closure_by_its_position_and_leaves_out_a_function_without_accesses() {
        let source = LoanSource::new(
            "(defn quiet () 1)\n\n(defn main ()\n  (local v (vec 1))\n  (local grow (fn () (push! v 2)))\n  (grow))",
        );
        assert_eq!(
            source.report(),
            "closure at 5:15\n  5:29 change `v`: no live loan\n"
        );
    }
}
