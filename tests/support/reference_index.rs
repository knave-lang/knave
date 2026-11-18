use std::fs;

use knave::cli::ReferencePages;

use super::ReferencePage;

pub struct ReferenceIndex {
    text: String,
}

impl ReferenceIndex {
    const LINK_END: &str = ".md)";

    pub fn written() -> Self {
        let path = format!(
            "{}/{}.md",
            ReferencePages::FOLDER,
            ReferencePages::INDEX_SOURCE
        );
        Self::new(&fs::read_to_string(path).expect("the reference index is readable"))
    }

    pub fn new(text: &str) -> Self {
        let text = text.to_string();
        Self { text }
    }

    pub fn link_mismatches(&self, pages: &[ReferencePage]) -> Vec<String> {
        let names: Vec<&str> = pages
            .iter()
            .map(ReferencePage::name)
            .filter(|name| *name != ReferencePages::INDEX_SOURCE)
            .collect();
        let links = self.linked_pages();
        let unlinked = names
            .iter()
            .filter(|name| !links.contains(name))
            .map(|name| {
                format!(
                    "the index has no link to `{}`",
                    ReferencePages::written_file(name)
                )
            });
        let pageless = links
            .iter()
            .filter(|link| !names.contains(link))
            .map(|link| {
                format!(
                    "the index links `{}`, which is no page",
                    ReferencePages::written_file(link)
                )
            });
        unlinked.chain(pageless).collect()
    }

    fn linked_pages(&self) -> Vec<&str> {
        let link_start = format!("]({}/", ReferencePages::WRITTEN_FOLDER);
        self.text
            .split(link_start.as_str())
            .skip(1)
            .filter_map(|rest| rest.split_once(Self::LINK_END))
            .map(|(name, _)| name)
            .collect()
    }
}
