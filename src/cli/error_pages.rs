use super::doc_folder::WrittenPage;
use super::exit_status::ExitStatus;
use super::explanation_lookup::ExplanationLookup;
use super::explanation_page::ExplanationPage;
use super::terminal::Terminal;
use crate::diagnostic::Explanation;
use crate::language::Language;

pub struct ErrorPages;

impl ErrorPages {
    const INDEX: &'static str = "errors";
    const INDEX_TITLE: &'static str = "Diagnostic codes";
    const TABLE_HEAD: &'static str = "| Code | Name |\n| --- | --- |\n";

    pub fn pages(terminal: &mut Terminal) -> Result<Vec<WrittenPage>, ExitStatus> {
        Ok(Self::of(Self::explanations(terminal)?))
    }

    fn of(mut explanations: Vec<ExplanationPage>) -> Vec<WrittenPage> {
        explanations.sort_by(|left, right| left.code().cmp(right.code()));
        let written = explanations.iter().map(Self::code_page);
        [Self::index(&explanations)]
            .into_iter()
            .chain(written)
            .collect()
    }

    fn explanations(terminal: &mut Terminal) -> Result<Vec<ExplanationPage>, ExitStatus> {
        let mut lookup = ExplanationLookup::new(terminal);
        Explanation::all()
            .map(|explanation| {
                let text = lookup.text(&explanation)?;
                Ok(ExplanationPage::new(explanation, text))
            })
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
        format!("| [{code}]({}/{code}.md) | {name} |\n", Self::INDEX)
    }

    fn code_page(page: &ExplanationPage) -> WrittenPage {
        WrittenPage {
            module: format!("{}/{}", Self::INDEX, page.code()),
            text: page.markdown(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorPages;
    use crate::cli::explanation_page::ExplanationPage;
    use crate::diagnostic::Explanation;

    const EXPLANATION_TEXT: &str = "An explanation.";

    #[test]
    fn the_index_lists_the_codes_in_order() {
        let explained = |explanation| ExplanationPage::new(explanation, EXPLANATION_TEXT.into());
        let pages = ErrorPages::of(Explanation::all().map(explained).collect());
        let codes: Vec<&str> = pages[0]
            .text
            .lines()
            .filter_map(|row| row.strip_prefix("| "))
            .filter_map(|row| row.split([' ', ']']).next())
            .map(|code| code.trim_start_matches('['))
            .filter(|code| code.starts_with('L'))
            .collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        assert!(codes.len() > 1);
        assert_eq!(codes, sorted);
    }
}
