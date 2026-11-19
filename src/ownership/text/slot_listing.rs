use super::slot::{ListedSlot, Slot, SlotOwner};
use super::text_position::TextPositions;
use crate::types::Type;

pub struct ListedSection {
    pub heading: Option<String>,
    pub slots: Vec<ListedSlot>,
}

#[derive(Default)]
pub struct SlotListing {
    sections: Vec<ListedSection>,
}

impl SlotListing {
    pub fn open(&mut self, heading: Option<String>) {
        self.sections.push(ListedSection {
            heading,
            slots: Vec::new(),
        });
    }

    pub fn list(&mut self, owner: &SlotOwner, ty: &Type) {
        let section = self
            .sections
            .last_mut()
            .expect("a slot is listed inside a section");
        section.slots.extend(
            TextPositions::of(ty)
                .into_iter()
                .map(|position| ListedSlot {
                    slot: Slot::new(owner.clone(), position.path),
                    written: position.written,
                }),
        );
    }

    pub fn sections(&self) -> &[ListedSection] {
        &self.sections
    }
}
