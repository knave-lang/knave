use super::access::Access;
use super::passing_relaxation::PassingRelaxation;
use super::template::{ParsedTemplate, PassingAspect};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ArgumentPassing<'t> {
    accesses: Vec<Access>,
    receives: bool,
    indexes: bool,
    dereferences: bool,
    called: Option<Vec<&'t str>>,
    operand: bool,
}

impl<'t> ArgumentPassing<'t> {
    pub(super) fn of(template: &'t ParsedTemplate, argument: usize) -> Self {
        Self {
            accesses: template
                .pieces()
                .iter()
                .filter_map(|piece| piece.access_for(argument))
                .collect(),
            receives: template.receives(argument),
            indexes: template.indexes(argument),
            dereferences: template.dereferences(argument),
            called: template.called_arguments(argument),
            operand: template
                .operation()
                .operator_arguments()
                .any(|operand| operand == argument),
        }
    }

    pub(super) fn relaxed(self, relaxation: PassingRelaxation) -> Self {
        match relaxation {
            PassingRelaxation::Copied => self.copied(),
            PassingRelaxation::SharedReference => self.without_receiver(),
            PassingRelaxation::Exact => self,
        }
    }

    fn without_receiver(self) -> Self {
        Self {
            receives: false,
            ..self
        }
    }

    fn copied(self) -> Self {
        Self {
            operand: false,
            ..self
        }
    }

    pub(super) fn differs_as_operand(&self, clause: &ArgumentPassing) -> bool {
        self.operand != clause.operand
    }

    pub(super) fn differing_aspect(&self, clause: &ArgumentPassing) -> Option<PassingAspect> {
        [
            (
                PassingAspect::Use,
                self.accesses.is_empty() != clause.accesses.is_empty(),
            ),
            (PassingAspect::Borrow, self.accesses != clause.accesses),
            (PassingAspect::Receiver, self.receives != clause.receives),
            (PassingAspect::Index, self.indexes != clause.indexes),
            (
                PassingAspect::Dereference,
                self.dereferences != clause.dereferences,
            ),
            (PassingAspect::Call, self.called != clause.called),
            (PassingAspect::Operand, self.operand != clause.operand),
        ]
        .into_iter()
        .find_map(|(aspect, differs)| differs.then_some(aspect))
    }
}
