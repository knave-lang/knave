use super::representation::Representation;
use super::slot::Slot;

#[derive(Clone, Debug)]
pub struct ListedSlot {
    pub slot: Slot,
    pub written: Representation,
}
