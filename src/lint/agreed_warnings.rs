use rustc_hash::FxHashSet;

use super::program_warnings::ProgramWarnings;
use crate::diagnostic::{Diagnostic, DiagnosticRenderer, Diagnostics};
use crate::expand::Program;
use crate::source::SourceMap;

pub struct AgreedWarnings<'s> {
    sources: &'s SourceMap,
}

struct ProgramView {
    files: FxHashSet<String>,
    rendered: FxHashSet<String>,
}

impl<'s> AgreedWarnings<'s> {
    pub fn new(sources: &'s SourceMap) -> Self {
        Self { sources }
    }

    pub fn of(&self, programs: Vec<ProgramWarnings<'_>>) -> Diagnostics {
        let views: Vec<ProgramView> = programs.iter().map(|program| self.view(program)).collect();
        let mut kept = Diagnostics::default();
        let mut kept_texts = FxHashSet::default();
        for warning in programs.iter().flat_map(|program| &program.warnings) {
            let rendered = self.render(warning);
            let file = self.file_of(warning);
            let agreed = views
                .iter()
                .filter(|view| view.files.contains(&file))
                .all(|view| view.rendered.contains(&rendered));
            if agreed && kept_texts.insert(rendered) {
                kept.push(warning.clone());
            }
        }
        kept
    }

    fn view(&self, program: &ProgramWarnings<'_>) -> ProgramView {
        ProgramView {
            files: self.files_of(program.program),
            rendered: program
                .warnings
                .into_iter()
                .map(|warning| self.render(warning))
                .collect(),
        }
    }

    fn files_of(&self, program: &Program) -> FxHashSet<String> {
        program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .map(|item| self.sources.file(item.span.source()).name().to_string())
            .collect()
    }

    fn file_of(&self, warning: &Diagnostic) -> String {
        let source = warning.primary().span().source();
        self.sources.file(source).name().to_string()
    }

    fn render(&self, warning: &Diagnostic) -> String {
        DiagnosticRenderer::plain(self.sources).render(warning)
    }
}
