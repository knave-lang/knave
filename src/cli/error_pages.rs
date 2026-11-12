use super::doc_folder::WrittenPage;
use super::explanation_page::ExplanationPage;
use crate::diagnostic::Explanation;
use crate::language::Language;

pub struct ErrorPages;

impl ErrorPages {
    const INDEX: &'static str = "errors";
    const INDEX_TITLE: &'static str = "Diagnostic codes";
    const TABLE_HEAD: &'static str = "| Code | Name |\n| --- | --- |\n";
    const UNWRITTEN_MARK: &'static str = "no explanation yet";

    pub fn pages() -> Vec<WrittenPage> {
        let explanations: Vec<ExplanationPage> =
            Explanation::all().map(ExplanationPage::new).collect();
        let written = explanations.iter().filter_map(Self::code_page);
        [Self::index(&explanations)]
            .into_iter()
            .chain(written)
            .collect()
    }

    fn index(explanations: &[ExplanationPage]) -> WrittenPage {
        let rows: String = explanations.iter().map(Self::index_row).collect();
        WrittenPage {
            module: Self::INDEX.to_string(),
            text: format!(
                "# {}\n\nRun `{} doc <code>` to print a code's explanation.\n\n{}{rows}",
                Self::INDEX_TITLE,
                Language::NAME,
                Self::TABLE_HEAD
            ),
        }
    }

    fn index_row(page: &ExplanationPage) -> String {
        let code = page.code();
        let name = page.display_name();
        if page.is_written() {
            format!("| [{code}]({}/{code}.md) | {name} |\n", Self::INDEX)
        } else {
            format!("| {code} | {name}, {} |\n", Self::UNWRITTEN_MARK)
        }
    }

    fn code_page(page: &ExplanationPage) -> Option<WrittenPage> {
        Some(WrittenPage {
            module: format!("{}/{}", Self::INDEX, page.code()),
            text: page.markdown()?,
        })
    }
}
