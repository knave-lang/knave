use super::slot_owner::SlotOwner;
use super::text_names::TextNames;
use super::type_path::TypePath;
use crate::core::BindingId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    owner: SlotOwner,
    path: TypePath,
}

impl From<SlotOwner> for Slot {
    fn from(owner: SlotOwner) -> Self {
        Self::new(owner, TypePath::default())
    }
}

impl Slot {
    pub fn new(owner: SlotOwner, path: TypePath) -> Self {
        Self { owner, path }
    }

    pub fn nested(&self, path: &TypePath) -> Self {
        Self::new(self.owner.clone(), self.path.joined(path))
    }

    pub fn label(&self, names: TextNames<'_>) -> String {
        format!("{}{}", self.owner_label(names), self.path)
    }

    pub fn is_stored_place(&self) -> bool {
        matches!(self.owner, SlotOwner::Field(..) | SlotOwner::Written(_))
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

    fn owner_label(&self, names: TextNames<'_>) -> String {
        match &self.owner {
            SlotOwner::Parameter(binding) => names.name(*binding).to_string(),
            SlotOwner::Result(_) => "result".to_string(),
            SlotOwner::Local(binding) => format!("local {}", names.name(*binding)),
            SlotOwner::Field(_, name) => name.to_string(),
            SlotOwner::Variable(..)
            | SlotOwner::Value(_)
            | SlotOwner::Argument(_)
            | SlotOwner::Written(_) => String::new(),
        }
    }
}
