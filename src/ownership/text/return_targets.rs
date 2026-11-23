use super::slot_owner::SlotOwner;
use crate::types::Type;

#[derive(Clone)]
pub struct ReturnTarget {
    pub owner: SlotOwner,
    pub ty: Type,
}

pub struct ReturnTargets {
    function: ReturnTarget,
    closures: Vec<ReturnTarget>,
}

impl ReturnTargets {
    pub fn new(function: ReturnTarget) -> Self {
        Self {
            function,
            closures: Vec::new(),
        }
    }

    pub fn enter(&mut self, closure: ReturnTarget) {
        self.closures.push(closure);
    }

    pub fn leave(&mut self) {
        self.closures.pop();
    }

    pub fn function(&self) -> &ReturnTarget {
        &self.function
    }

    pub fn innermost(&self) -> &ReturnTarget {
        self.closures.last().unwrap_or(&self.function)
    }
}
