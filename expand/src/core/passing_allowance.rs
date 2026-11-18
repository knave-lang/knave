use super::ArgumentPositions;
use super::passing_relaxation::PassingRelaxation;

pub struct PassingAllowance {
    shared_references: ArgumentPositions,
    copied: ArgumentPositions,
}

impl PassingAllowance {
    pub fn new(shared_references: ArgumentPositions, copied: ArgumentPositions) -> Self {
        Self {
            shared_references,
            copied,
        }
    }

    pub fn exact(&self) -> Self {
        Self {
            shared_references: self.shared_references.clone(),
            copied: ArgumentPositions::default(),
        }
    }

    pub fn copies(&self, argument: usize) -> bool {
        self.copied.holds(argument)
    }

    pub fn passes_shared_reference(&self, argument: usize) -> bool {
        self.shared_references.holds(argument)
    }

    pub fn relaxation(&self, argument: usize) -> PassingRelaxation {
        if self.copied.holds(argument) {
            PassingRelaxation::Copied
        } else if self.shared_references.holds(argument) {
            PassingRelaxation::SharedReference
        } else {
            PassingRelaxation::Exact
        }
    }
}
