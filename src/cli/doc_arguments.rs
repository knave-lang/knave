use std::convert::Infallible;
use std::path::PathBuf;

use clap::Args;

use super::doc::{DocCommand, DocEnvironment, DocPage};
use super::doc_write::{CompilerPages, DocScope, DocWriteCommand};
use super::exit_status::ExitStatus;
use super::explanation_lookup::ExplanationLookup;
use super::explanation_page::ExplanationPage;
use super::term_parser::{SurplusParser, TermParser};
use super::terminal::Terminal;
use crate::diagnostic::Explanation;

#[derive(Args)]
pub struct DocArguments {
    #[arg(
        allow_hyphen_values = true,
        value_parser = TermParser,
        required_unless_present_any = ["separated_term", "write"]
    )]
    term: Option<String>,
    #[arg(hide = true, value_parser = SurplusParser)]
    surplus: Vec<Infallible>,
    #[arg(last = true, hide = true, value_name = "TERM", conflicts_with = "term")]
    separated_term: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long, help = "Print the inferred type above the page")]
    types: bool,
    #[arg(
        long,
        conflicts_with_all = ["term", "separated_term", "file", "types"],
        help = "Write a Markdown page per module of the project into doc/"
    )]
    write: bool,
    #[arg(
        long,
        requires = "write",
        value_name = "DIRECTORY",
        help = "The folder that --write writes into [default: doc/ at the project root]"
    )]
    output: Option<PathBuf>,
    #[arg(
        long,
        requires = "write",
        help = "Write the pages of the prelude's modules instead of the project's"
    )]
    prelude: bool,
    #[arg(
        long,
        requires = "write",
        help = "Write the pages of the diagnostic codes instead of the project's"
    )]
    errors: bool,
    #[arg(
        long,
        requires = "write",
        help = "Write the pages of the language reference instead of the project's"
    )]
    reference: bool,
}

impl DocArguments {
    pub fn run(self, terminal: &mut Terminal) -> ExitStatus {
        if self.write {
            let scope = self.scope();
            return DocWriteCommand::new(self.output, scope).run(terminal);
        }
        let page = self.page();
        let term = self
            .term
            .or(self.separated_term)
            .expect("clap requires one term");
        match Explanation::of_code(&term) {
            Some(explanation) => Self::explain(explanation, terminal),
            None => DocCommand::new(DocEnvironment::new(self.file), term, page).run(terminal),
        }
    }

    fn explain(explanation: Explanation, terminal: &mut Terminal) -> ExitStatus {
        ExplanationLookup::new(terminal)
            .text(&explanation)
            .map(|text| ExplanationPage::new(explanation, text).print())
            .unwrap_or_else(|status| status)
    }

    fn scope(&self) -> DocScope {
        let chosen = self.chosen_compiler_pages();
        if chosen.is_empty() {
            DocScope::Project
        } else {
            DocScope::Compiler(chosen)
        }
    }

    fn chosen_compiler_pages(&self) -> Vec<CompilerPages> {
        [
            (self.prelude, CompilerPages::Prelude),
            (self.errors, CompilerPages::Errors),
            (self.reference, CompilerPages::Reference),
        ]
        .into_iter()
        .filter_map(|(chosen, pages)| chosen.then_some(pages))
        .collect()
    }

    fn page(&self) -> DocPage {
        if self.types {
            DocPage::Typed
        } else {
            DocPage::Written
        }
    }
}
