use super::expanded_page::ExpandedPage;
use crate::core::{Expr, ModulePath};
use crate::diagnostic::Diagnostics;
use crate::expand::ExpansionSession;
use crate::modules::{ModuleSources, PublicPrelude};
use crate::reader::Form;
use crate::source::SourceId;

pub struct LintPages<'a, 's> {
    session: &'a mut ExpansionSession<'s>,
}

impl<'a, 's> LintPages<'a, 's> {
    const CALL_SOURCE: &'static str = "<lint page>";
    const RULE_MACRO: &'static str = "rule-page-part";
    const EXPLANATION_MACRO: &'static str = "lint-explanation";

    pub fn new(session: &'a mut ExpansionSession<'s>) -> Self {
        Self { session }
    }

    pub fn term_page(
        &mut self,
        term: &Form,
        source: SourceId,
    ) -> Result<ExpandedPage, Diagnostics> {
        match self.unbound_rule_page(term)? {
            Some(rule_page) => Ok(rule_page),
            None => ExpandedPage::expand(self.session, term, source),
        }
    }

    fn unbound_rule_page(&mut self, term: &Form) -> Result<Option<ExpandedPage>, Diagnostics> {
        let unbound = self.session.binding(term).is_none();
        match term.as_symbol().filter(|_| unbound) {
            Some(name) => {
                self.page(|part| format!("({} {part} {})", Self::RULE_MACRO, name.name()))
            }
            None => Ok(None),
        }
    }

    pub fn documented_explanation(&mut self, name: &str) -> Result<Option<String>, Diagnostics> {
        let paths = PublicPrelude::paths().join(" ");
        let call = format!("({} {name} ({paths}))", Self::EXPLANATION_MACRO);
        let explanation = self.expand(&call)?;
        Ok(ExpandedPage::literal(&explanation)
            .filter(|text| !text.is_empty())
            .map(str::to_string))
    }

    fn page(&mut self, call: impl Fn(&str) -> String) -> Result<Option<ExpandedPage>, Diagnostics> {
        let header = self.expand(&call(ExpandedPage::HEADER_MACRO))?;
        if ExpandedPage::literal(&header).is_some_and(str::is_empty) {
            return Ok(None);
        }
        let body = self.expand(&call(ExpandedPage::BODY_MACRO))?;
        Ok(Some(ExpandedPage::new(header, body)))
    }

    fn expand(&mut self, call: &str) -> Result<Expr, Diagnostics> {
        let source = self.session.add_source(Self::CALL_SOURCE, call);
        let form = self.session.read(source)?.remove(0);
        self.session
            .expression_in(&ModulePath::new(ModuleSources::DOCUMENTATION), form)
    }
}
