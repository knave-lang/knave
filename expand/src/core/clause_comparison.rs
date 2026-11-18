use super::argument_passing::ArgumentPassing;
use super::dropped_borrows::DroppedBorrows;
use super::passing_allowance::PassingAllowance;
use super::passing_relaxation::PassingRelaxation;
use super::template::{ParsedTemplate, PassingAspect, PassingDifference};

pub struct ClauseComparison<'t> {
    template: &'t ParsedTemplate,
    clause: &'t ParsedTemplate,
    allowance: &'t PassingAllowance,
}

impl<'t> ClauseComparison<'t> {
    pub fn new(
        template: &'t ParsedTemplate,
        clause: &'t ParsedTemplate,
        allowance: &'t PassingAllowance,
    ) -> Self {
        Self {
            template,
            clause,
            allowance,
        }
    }

    pub fn dropped_borrows(&self) -> Result<DroppedBorrows, PassingDifference> {
        match self.compared_under(&self.allowance.exact()) {
            Ok(dropped) if !self.skips_the_template_call() => Ok(dropped),
            Ok(_) => self.relaxed(),
            Err(difference) if self.relaxes(difference) => self.relaxed(),
            Err(difference) => Err(difference),
        }
    }

    fn relaxed(&self) -> Result<DroppedBorrows, PassingDifference> {
        if !self.clause.operation().is_pure() {
            return Err(PassingDifference::ImpureClause);
        }
        self.compared_under(self.allowance)
    }

    fn relaxes(&self, difference: PassingDifference) -> bool {
        matches!(difference, PassingDifference::Argument(argument, PassingAspect::Operand)
            if self.allowance.copies(argument))
    }

    fn skips_the_template_call(&self) -> bool {
        self.template.operation().calls() && !self.clause.operation().calls()
    }

    fn compared_under(
        &self,
        allowance: &PassingAllowance,
    ) -> Result<DroppedBorrows, PassingDifference> {
        let mut arguments: Vec<usize> = self
            .template
            .placeholders()
            .chain(self.clause.placeholders())
            .collect();
        arguments.sort_unstable();
        arguments.dedup();
        let mut dropped = Vec::new();
        for argument in arguments {
            if self.drops_a_borrow(argument, allowance)? {
                dropped.push(argument);
            }
        }
        match self.call_difference() {
            Some(difference) => Err(difference),
            None => Ok(DroppedBorrows::new(dropped)),
        }
    }

    fn drops_a_borrow(
        &self,
        argument: usize,
        allowance: &PassingAllowance,
    ) -> Result<bool, PassingDifference> {
        let relaxation = allowance.relaxation(argument);
        let in_clause = Self::compared(self.clause, argument, relaxation);
        let in_template = Self::compared(self.template, argument, relaxation);
        let Some(aspect) = in_template.differing_aspect(&in_clause) else {
            return Ok(false);
        };
        let unborrowed = self.template.without_written_borrow(argument);
        let remaining =
            Self::compared(&unborrowed, argument, relaxation).differing_aspect(&in_clause);
        if let Some(remaining) = remaining {
            let reported = if in_template.differs_as_operand(&in_clause) {
                remaining
            } else {
                aspect
            };
            return Err(PassingDifference::Argument(argument, reported));
        }
        Ok(!allowance.passes_shared_reference(argument))
    }

    fn compared<'p>(
        template: &'p ParsedTemplate,
        argument: usize,
        relaxation: PassingRelaxation,
    ) -> ArgumentPassing<'p> {
        ArgumentPassing::of(template, argument).relaxed(relaxation)
    }

    fn call_difference(&self) -> Option<PassingDifference> {
        let calls_only_in_clause =
            self.clause.operation().calls() && !self.template.operation().calls();
        calls_only_in_clause.then_some(PassingDifference::CallOnlyInClause)
    }
}

#[cfg(test)]
mod tests {
    use super::ClauseComparison;
    use crate::core::{
        ArgumentPositions, ParsedTemplate, PassingAllowance, PassingAspect, PassingDifference,
    };

    struct ComparisonFixture;

    impl ComparisonFixture {
        fn difference(template: &str, clause: &str) -> Option<PassingDifference> {
            let template = ParsedTemplate::parse(template).expect("a valid template");
            let clause = ParsedTemplate::parse(clause).expect("a valid template");
            let allowance = PassingAllowance::new(
                ArgumentPositions::default(),
                ArgumentPositions::new(vec![0]),
            );
            ClauseComparison::new(&template, &clause, &allowance)
                .dropped_borrows()
                .err()
        }
    }

    #[test]
    fn lets_a_plain_clause_make_a_copied_argument_an_operand() {
        assert_eq!(
            ComparisonFixture::difference("crate::rt::is_even({0})", "{0} % 2 == 0"),
            None
        );
    }

    #[test]
    fn refuses_a_relaxed_clause_that_is_not_a_plain_expression() {
        assert_eq!(
            ComparisonFixture::difference(
                "crate::rt::is_even({0})",
                "{{ ({0}, _) = (7, 0); true }}"
            ),
            Some(PassingDifference::ImpureClause)
        );
    }

    #[test]
    fn compares_an_argument_that_is_not_copied_exactly() {
        assert_eq!(
            ComparisonFixture::difference("f({0}, {1})", "f({0}, {1} + 1)"),
            Some(PassingDifference::Argument(1, PassingAspect::Operand))
        );
    }
}
