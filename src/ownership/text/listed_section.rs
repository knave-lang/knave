use super::listed_slot::ListedSlot;

pub struct ListedSection {
    pub heading: Option<String>,
    pub slots: Vec<ListedSlot>,
}
