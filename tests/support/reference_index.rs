use std::fs;

use super::ReferencePage;

pub struct ReferenceIndex {
    text: String,
}

impl ReferenceIndex {
    pub const PAGE_NAME: &str = "README";
    const PATH: &str = "docs/reference/README.md";
    const LINK_START: &str = "](reference/";
    const LINK_END: &str = ".md)";

    pub fn written() -> Self {
        Self::new(&fs::read_to_string(Self::PATH).expect("the reference index is readable"))
    }

    pub fn new(text: &str) -> Self {
        let text = text.to_string();
        Self { text }
    }

    pub fn link_mismatches(&self, pages: &[ReferencePage]) -> Vec<String> {
        let names: Vec<&str> = pages
            .iter()
            .map(ReferencePage::name)
            .filter(|name| *name != Self::PAGE_NAME)
            .collect();
        let links: Vec<&str> = self.linked_pages().collect();
        let unlinked = names
            .iter()
            .filter(|name| !links.contains(name))
            .map(|name| format!("the index has no link to `reference/{name}.md`"));
        let pageless = links
            .iter()
            .filter(|link| !names.contains(link))
            .map(|link| format!("the index links `reference/{link}.md`, which is no page"));
        unlinked.chain(pageless).collect()
    }

    fn linked_pages(&self) -> impl Iterator<Item = &str> {
        self.text
            .split(Self::LINK_START)
            .skip(1)
            .filter_map(|rest| rest.split_once(Self::LINK_END))
            .map(|(name, _)| name)
    }
}
