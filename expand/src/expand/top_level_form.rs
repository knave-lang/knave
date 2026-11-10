use super::Expander;
use super::budget_arena::BudgetId;
use crate::diagnostic::{Diagnostic, DistinctDiagnostics};

impl Expander<'_> {
    pub(super) fn within_top_level_form<T>(
        &mut self,
        expand: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let budget = self.form_budgets.open();
        self.within_source_form(budget, expand)
    }

    pub(super) fn within_source_form<T>(
        &mut self,
        budget: BudgetId,
        expand: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let first_recovered = self.recovered_diagnostics.len();
        let expanded = self.spending(budget, expand);
        let recovered = self.recovered_diagnostics.split_off(first_recovered);
        match expanded {
            Ok(value) => {
                self.recover_distinct(recovered);
                Ok(value)
            }
            Err(error) => Err(self.recover_distinct_beside(error, recovered)),
        }
    }

    pub(super) fn within_source_run(&mut self, budget: BudgetId, lower: impl FnOnce(&mut Self)) {
        let first_recovered = self.recovered_diagnostics.len();
        self.spending(budget, lower);
        let recovered = self.recovered_diagnostics.split_off(first_recovered);
        self.recover_distinct(recovered);
    }

    pub(super) fn spending<T>(
        &mut self,
        budget: BudgetId,
        expand: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let outer = self.form_budgets.spending();
        self.form_budgets.spend_from(budget);
        let expanded = expand(self);
        self.form_budgets.spend_from(outer);
        expanded
    }

    fn recover_distinct(&mut self, recovered: Vec<Diagnostic>) {
        let distinct = DistinctDiagnostics::of(recovered);
        self.recovered_diagnostics
            .extend(distinct.into_diagnostics());
    }

    fn recover_distinct_beside(
        &mut self,
        error: Diagnostic,
        recovered: Vec<Diagnostic>,
    ) -> Diagnostic {
        let with_error = std::iter::once(error).chain(recovered);
        let mut distinct = DistinctDiagnostics::of(with_error).into_diagnostics();
        let most_helpful_error = distinct.remove(0);
        self.recovered_diagnostics.extend(distinct);
        most_helpful_error
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Expander;
    use crate::core::ModulePath;
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct ExpressionFixture {
        sources: SourceMap,
    }

    impl ExpressionFixture {
        const ENTRY: &str = "entry";
        const SPINNING_MACRO: &str = "(use code number-gt number-sub)

(defmacro spin-steps (count)
  (local spin
    (fn (left)
      (if (number-gt left 0) (recur (number-sub left 1)) left)))
  (spin count))";
        const LOWERED_LIMIT: &str = "(with-macro-steps 20000 0)";
        const SPENDS_PAST_THE_LOWERED_LIMIT: &str = "(spin-steps 20000)";

        fn new() -> Self {
            Self {
                sources: SourceMap::default(),
            }
        }

        fn expands_both(&mut self, first: &str, second: &str) -> bool {
            let entry = self.sources.add(Self::ENTRY, Self::SPINNING_MACRO);
            let mut session = Expander::new(
                &mut self.sources,
                ModuleSources::new(PathBuf::new(), PathBuf::new()),
            )
            .open(ModulePath::new(Self::ENTRY), &[entry])
            .unwrap_or_else(|_| panic!("the entry expands"));
            [first, second].iter().all(|text| {
                let source = session.add_source(Self::ENTRY, text);
                let form = session
                    .read(source)
                    .unwrap_or_else(|_| panic!("the expression reads"))
                    .remove(0);
                session
                    .expression_in(&ModulePath::new(Self::ENTRY), form)
                    .is_ok()
            })
        }
    }

    #[test]
    fn each_expression_spends_from_its_own_step_budget() {
        assert!(ExpressionFixture::new().expands_both(
            ExpressionFixture::LOWERED_LIMIT,
            ExpressionFixture::SPENDS_PAST_THE_LOWERED_LIMIT,
        ));
    }
}
