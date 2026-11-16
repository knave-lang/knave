use crate::core::ParsedTemplate;

pub struct BorrowedIndex;

impl BorrowedIndex {
    pub fn place(template: &str) -> Option<&str> {
        let place = template.strip_prefix('&')?;
        ParsedTemplate::parse(place)
            .ok()?
            .is_single_index_place()
            .then_some(place)
    }
}
