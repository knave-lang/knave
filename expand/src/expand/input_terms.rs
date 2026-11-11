use std::rc::Rc;

use super::recorded_inputs::BoundName;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputTerms {
    pub bound: Vec<BoundName>,
    pub documented: Vec<Rc<str>>,
}

impl InputTerms {
    pub fn binding(bound: Vec<BoundName>) -> Self {
        Self {
            bound,
            documented: Vec::new(),
        }
    }
}
