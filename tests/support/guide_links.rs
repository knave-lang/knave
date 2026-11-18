use std::fs;
use std::path::Path;

use knave::cli::ReferencePages;

use super::ReferencePage;

pub struct GuideLinks {
    text: String,
}

struct GuideLink<'g> {
    target: &'g str,
    page: &'g str,
    anchor: Option<&'g str>,
}

impl GuideLinks {
    const GUIDE: &str = "docs/language.md";
    const LINK_END: char = ')';
    const PAGE_EXTENSION: &str = ".md";
    const ANCHOR_START: char = '#';

    pub fn written() -> Self {
        Self::new(&fs::read_to_string(Self::GUIDE).expect("the language guide is readable"))
    }

    pub fn new(text: &str) -> Self {
        let text = text.to_string();
        Self { text }
    }

    pub fn broken(&self, pages: &[ReferencePage]) -> Vec<String> {
        let link_start = Self::link_start();
        self.text
            .split(link_start.as_str())
            .skip(1)
            .filter_map(|rest| rest.split(Self::LINK_END).next())
            .map(GuideLink::parse)
            .filter(|link| !link.resolves_in(pages))
            .map(|link| {
                format!(
                    "{} links to `{}`, which names no heading of a reference page",
                    Self::GUIDE,
                    link.target
                )
            })
            .collect()
    }

    fn link_start() -> String {
        let guide_folder = Path::new(Self::GUIDE)
            .parent()
            .expect("the guide is in a folder");
        let relative = Path::new(ReferencePages::FOLDER)
            .strip_prefix(guide_folder)
            .expect("the reference sits under the guide's folder");
        format!("]({}/", relative.display())
    }
}

impl<'g> GuideLink<'g> {
    fn parse(target: &'g str) -> Self {
        let (file, anchor) = match target.split_once(GuideLinks::ANCHOR_START) {
            Some((file, anchor)) => (file, Some(anchor)),
            None => (target, None),
        };
        let page = file
            .strip_suffix(GuideLinks::PAGE_EXTENSION)
            .unwrap_or(file);
        Self {
            target,
            page,
            anchor,
        }
    }

    fn resolves_in(&self, pages: &[ReferencePage]) -> bool {
        pages
            .iter()
            .filter(|page| page.name() == self.page)
            .any(|page| {
                self.anchor
                    .is_none_or(|anchor| page.has_heading_anchor(anchor))
            })
    }
}
