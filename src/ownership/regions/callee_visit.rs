use super::callee_parameters::CalleeParameters;

#[derive(Default)]
pub(super) struct CalleeVisit {
    pub parameters: CalleeParameters,
    pub earliest_reached: Option<usize>,
}

impl CalleeVisit {
    pub(super) fn settled(parameters: CalleeParameters) -> Self {
        Self {
            parameters,
            earliest_reached: None,
        }
    }
}
