use std::fs;
use std::path::{Path, PathBuf};

use knave::cli::{ModuleAlone, TrialStages};
use knave::core::ModulePath;
use knave::diagnostic::{DiagnosticCode, DiagnosticRenderer, Diagnostics};
use knave::expand::{Expander, Program};
use knave::language::Language;
use knave::lint::{
    LintDefaults, LintedProgram, Linter, NeedlessDeriveRule, ProgramPosition, ProgramRun,
    ProjectLibrary, RuleProgram, TrialLevels, TrialLints,
};
use knave::modules::ModuleSources;
use knave::project::LintLevel;
use knave::source::{SourceId, SourceMap, Span};
use knave::types::TypedProgram;

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
            Ok(program) => self.checked_warnings(program),
            Err(errors) => errors,
        };
        DiagnosticRenderer::plain(&self.sources).render_all(&diagnostics)
    }

    fn checked_warnings(&mut self, mut program: Program) -> Diagnostics {
        let mut typed = match TrialStages::check(&program) {
            Ok(typed) => typed,
            Err(errors) => return errors,
        };
        let mut warnings = std::mem::take(&mut typed.warnings);
        warnings.extend(self.lint_warnings(&program));
        warnings.extend(self.trial_warnings(&mut program, &typed));
        warnings
    }

    fn trial_warnings(&mut self, program: &mut Program, typed: &TypedProgram) -> Diagnostics {
        let levels = TrialLevels {
            needless_type: LintDefaults::level_of(
                &DiagnosticCode::NEEDLESS_TYPE,
                self.written_needless_type_level(),
            ),
            needless_derive: LintDefaults::level_of(&DiagnosticCode::NEEDLESS_DERIVE, None),
        };
        let derive = NeedlessDeriveRule::new(ProjectLibrary::Unpublished);
        let holds_unlinted_example = self.holds_example(program);
        TrialLints::new(&mut self.runs(), derive).warnings(
            levels,
            &mut [LintedProgram {
                program,
                declared: typed,
                holds_unlinted_example,
            }],
        )
    }

    fn runs(&mut self) -> FixtureRuns<'_> {
        FixtureRuns {
            sources: &mut self.sources,
            path: &self.path,
            entry: self.entry,
        }
    }

    fn written_needless_type_level(&self) -> Option<LintLevel> {
        let project = self.path.with_file_name(Language::project_file_name());
        if !project.exists() {
            return None;
        }
        Fixture::load(&project)
            .project()
            .expect("the fixture project file is a project")
            .lints
            .levels()
            .find(|(code, _)| **code == DiagnosticCode::NEEDLESS_TYPE)
            .map(|(_, level)| level)
    }

    fn holds_example(&self, program: &Program) -> bool {
        let mut rules = RuleProgram::expand_from_sources().expect("the lint rules expand");
        Linter::new(program, &self.sources)
            .holds_example(&mut rules)
            .expect("the example question runs")
    }

    fn lint_warnings(&self, program: &Program) -> Diagnostics {
        let mut rules = RuleProgram::expand_from_sources().expect("the lint rules expand");
        Linter::new(program, &self.sources)
            .lint(&mut rules)
            .expect("the lint rules run")
    }

    fn expand(&mut self) -> Result<Program, Diagnostics> {
        FixtureRuns::expander(&mut self.sources, &self.path)
            .expand(FixtureRuns::module_path(&self.path), self.entry)
    }
}

struct FixtureRuns<'f> {
    sources: &'f mut SourceMap,
    path: &'f Path,
    entry: SourceId,
}

impl FixtureRuns<'_> {
    fn expander<'s>(sources: &'s mut SourceMap, path: &Path) -> Expander<'s> {
        let directory = path
            .parent()
            .expect("fixtures live in a directory")
            .to_path_buf();
        let display = PathBuf::from(Fixture::display_name(&directory));
        Expander::new(sources, ModuleSources::new(directory, display))
    }

    fn module_path(path: &Path) -> ModulePath {
        let stem = path.file_stem().expect("fixtures have a file name");
        ModulePath::new(stem.to_string_lossy().into_owned())
    }

    fn expanded_alone(&mut self, module: &ModulePath) -> Result<Program, Diagnostics> {
        let entry = self
            .sources
            .add(Fixture::display_name(self.path), String::new());
        let mut session =
            Self::expander(self.sources, self.path).open(Self::module_path(self.path), &[entry])?;
        ModuleAlone::load(&mut session, module.as_str())?;
        session.finish()
    }
}

impl ProgramRun for FixtureRuns<'_> {
    fn check(&mut self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        TrialStages::check(program)
    }

    fn emit(&self, typed: &TypedProgram) -> Option<String> {
        TrialStages::emit(typed, Span::new(self.entry, 0..0))
    }

    fn module_alone(&mut self, _beside: ProgramPosition, module: &ModulePath) -> Option<Program> {
        self.expanded_alone(module).ok()
    }

    fn source_path(&self, source: SourceId) -> Option<PathBuf> {
        fs::canonicalize(self.sources.file(source).name()).ok()
    }
}
