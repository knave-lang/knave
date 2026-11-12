use crate::form_reducer::FormReducer;
use crate::known_classes::KnownClasses;

pub struct ReductionPlan {
    reducer: FormReducer,
    classes: &'static KnownClasses,
    count: usize,
}

impl ReductionPlan {
    pub fn new(reducer: FormReducer, classes: &'static KnownClasses, count: usize) -> Self {
        Self {
            reducer,
            classes,
            count,
        }
    }

    pub fn reducer(&self) -> &FormReducer {
        &self.reducer
    }

    pub fn classes(&self) -> &'static KnownClasses {
        self.classes
    }

    pub fn count(&self) -> usize {
        self.count
    }
}
