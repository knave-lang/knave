use std::fs;
use std::path::{Path, PathBuf};

use knave::check::ProgramChecker;
use knave::core::ModulePath;
use knave::diagnostic::{DiagnosticRenderer, Diagnostics};
use knave::expand::{Expander, Program};
use knave::lint::{Linter, RuleProgram};
use knave::modules::ModuleSources;
use knave::ownership::EnforcedOwnership;
use knave::source::{SourceId, SourceMap};

use super::Fixture;

pub struct LintFixture {
    sources: SourceMap,
    entry: SourceId,
    path: PathBuf,
}

impl LintFixture {
    pub fn load(path: &Path) -> Self {
        let text = fs::read_to_string(path).expect("fixture is readable");
        Self::of_text(path, text)
    }

    pub fn of_text(path: &Path, text: String) -> Self {
        let mut sources = SourceMap::default();
        let entry = sources.add(Fixture::display_name(path), text);
        Self {
            sources,
            entry,
            path: path.to_path_buf(),
        }
    }

    pub fn report(mut self) -> String {
        let diagnostics = match self.expand() {
            Ok(program) => self.checked_warnings(&program),
            Err(errors) => errors,
        };
        DiagnosticRenderer::plain(&self.sources).render_all(&diagnostics)
    }

    fn checked_warnings(&self, program: &Program) -> Diagnostics {
        let mut warnings = match ProgramChecker::check::<EnforcedOwnership>(program) {
            Ok(typed) => typed.warnings,
            Err(errors) => return errors,
        };
        warnings.extend(self.lint_warnings(program));
        warnings
    }

    fn lint_warnings(&self, program: &Program) -> Diagnostics {
        let mut rules = RuleProgram::expand_from_sources().expect("the lint rules expand");
        Linter::new(program, &self.sources)
            .lint(&mut rules)
            .expect("the lint rules run")
    }

    fn expand(&mut self) -> Result<Program, Diagnostics> {
        let stem = self.path.file_stem().expect("fixtures have a file name");
        let directory = self
            .path
            .parent()
            .expect("fixtures live in a directory")
            .to_path_buf();
        Expander::new(
            &mut self.sources,
            ModuleSources::new(directory.clone(), directory),
        )
        .expand(
            ModulePath::new(stem.to_string_lossy().into_owned()),
            self.entry,
        )
    }
}
