use super::need::Need;
use super::slot_owner::SlotOwner;
use super::type_path::TypePath;
use crate::core::BindingId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    pub owner: SlotOwner,
    pub path: TypePath,
}

impl Slot {
    pub fn new(owner: SlotOwner, path: TypePath) -> Self {
        Self { owner, path }
    }

    pub fn top(owner: SlotOwner) -> Self {
        Self::new(owner, TypePath::default())
    }

    pub fn is_stored_place(&self) -> bool {
        matches!(self.owner, SlotOwner::Field(..))
    }

    pub fn need(&self) -> Option<Need> {
        match self.owner {
            SlotOwner::OwnedArgument(_) => Some(Need::Owned),
            SlotOwner::ClonedArgument(_) => Some(Need::OwnedOrMixed),
            SlotOwner::WrittenView(..) => Some(Need::View),
            _ => None,
        }
    }

    pub fn is_variable(&self) -> bool {
        matches!(self.owner, SlotOwner::Variable(..))
    }

    pub fn parameter(&self) -> Option<BindingId> {
        match self.owner {
            SlotOwner::Parameter(binding) if self.path.is_top() => Some(binding),
            _ => None,
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self.owner, SlotOwner::Local(_)) && self.path.is_top()
    }
}
