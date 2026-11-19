use super::flow::Flow;
use super::listed_section::ListedSection;
use super::listed_slot::ListedSlot;
use super::slot_listing::SlotListing;
use super::text_decision_table::TextDecisionTable;
use super::text_program::TextProgram;
use crate::source::{SourceMap, Span};

pub struct TextReport<'r, 'p> {
    program: &'r TextProgram<'p>,
    table: &'r TextDecisionTable<'r>,
    refused: Vec<Span>,
}

impl<'r, 'p> TextReport<'r, 'p> {
    pub fn new(
        program: &'r TextProgram<'p>,
        table: &'r TextDecisionTable<'r>,
        refusals: &[&Flow],
    ) -> Self {
        let mut refused: Vec<Span> = refusals.iter().map(|flow| flow.span).collect();
        refused.sort_by_key(|span| (span.source().index(), span.range().start));
        refused.dedup();
        Self {
            program,
            table,
            refused,
        }
    }

    pub fn written(&self, listing: &SlotListing, sources: &SourceMap) -> String {
        let sections: String = listing
            .sections()
            .iter()
            .map(|section| self.section(section))
            .collect();
        sections + &self.refusals(sources)
    }

    fn section(&self, section: &ListedSection) -> String {
        let lines: String = section
            .slots
            .iter()
            .map(|listed| format!("  {}\n", self.line(listed)))
            .collect();
        match &section.heading {
            Some(heading) if !lines.is_empty() => format!("{heading}\n{lines}"),
            _ => String::new(),
        }
    }

    fn line(&self, listed: &ListedSlot) -> String {
        let decided = self.table.of_slot(&listed.slot);
        let label = listed.slot.label(self.program.names());
        if decided == listed.written {
            format!("{label}: {decided}")
        } else {
            format!("{label}: {decided}, written {}", listed.written)
        }
    }

    fn refusals(&self, sources: &SourceMap) -> String {
        let lines: String = self
            .refused
            .iter()
            .map(|span| {
                let file = sources.file(span.source()).name();
                format!("  {file}:{}\n", sources.start_position(*span))
            })
            .collect();
        if lines.is_empty() {
            String::new()
        } else {
            format!("needs owned text\n{lines}")
        }
    }
}
