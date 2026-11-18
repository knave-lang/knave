use super::argument_passing::ArgumentPassing;
use super::dropped_borrows::DroppedBorrows;
use super::shared_references::SharedReferences;
use super::template::{ParsedTemplate, PassingDifference};

pub struct ClauseComparison<'t> {
    template: &'t ParsedTemplate,
    clause: &'t ParsedTemplate,
    shared_references: &'t SharedReferences,
}

impl<'t> ClauseComparison<'t> {
    pub fn new(
        template: &'t ParsedTemplate,
        clause: &'t ParsedTemplate,
        shared_references: &'t SharedReferences,
    ) -> Self {
        Self {
            template,
            clause,
            shared_references,
        }
    }

    pub fn dropped_borrows(&self) -> Result<DroppedBorrows, PassingDifference> {
        let mut arguments: Vec<usize> = self
            .template
            .placeholders()
            .chain(self.clause.placeholders())
            .collect();
        arguments.sort_unstable();
        arguments.dedup();
        let mut dropped = Vec::new();
        for argument in arguments {
            if self.drops_a_borrow(argument)? {
                dropped.push(argument);
            }
        }
        match self.call_difference() {
            Some(difference) => Err(difference),
            None => Ok(DroppedBorrows::new(dropped)),
        }
    }

    fn drops_a_borrow(&self, argument: usize) -> Result<bool, PassingDifference> {
        let in_clause = self.compared(self.clause, argument);
        let in_template = self.compared(self.template, argument);
        let Some(aspect) = in_template.differing_aspect(&in_clause) else {
            return Ok(false);
        };
        let unborrowed = self.template.without_written_borrow(argument);
        let remaining = self
            .compared(&unborrowed, argument)
            .differing_aspect(&in_clause);
        if let Some(remaining) = remaining {
            let reported = if in_template.differs_as_operand(&in_clause) {
                remaining
            } else {
                aspect
            };
            return Err(PassingDifference::Argument(argument, reported));
        }
        Ok(!self.shared_references.holds(argument))
    }

    fn compared<'p>(&self, template: &'p ParsedTemplate, argument: usize) -> ArgumentPassing<'p> {
        ArgumentPassing::of(template, argument)
            .ignoring_receiver_of_shared(self.shared_references, argument)
    }

    fn call_difference(&self) -> Option<PassingDifference> {
        match (
            self.template.operation().calls(),
            self.clause.operation().calls(),
        ) {
            (false, true) => Some(PassingDifference::CallOnlyInClause),
            (true, false) => Some(PassingDifference::CallOnlyInTemplate),
            _ => None,
        }
    }
}
