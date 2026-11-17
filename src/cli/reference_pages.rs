use super::doc_folder::WrittenPage;

pub struct ReferencePages;

impl ReferencePages {
    const INDEX: &'static str = "reference";
    const INDEX_TEXT: &'static str = include_str!("../../docs/reference/README.md");
    const PAGES: [(&'static str, &'static str); 1] =
        [("bindings", include_str!("../../docs/reference/bindings.md"))];

    pub fn pages() -> Vec<WrittenPage> {
        let index = Self::page(Self::INDEX.to_string(), Self::INDEX_TEXT);
        let pages = Self::PAGES
            .iter()
            .map(|(name, text)| Self::page(format!("{}/{name}", Self::INDEX), text));
        std::iter::once(index).chain(pages).collect()
    }

    fn page(module: String, text: &str) -> WrittenPage {
        let text = text.to_string();
        WrittenPage { module, text }
    }
}
