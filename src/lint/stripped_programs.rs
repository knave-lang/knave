use std::mem;

use super::declaration_edits::DeclarationEdits;
use super::linted_program::LintedProgram;
use super::program_run::ProgramRun;
use crate::core::{BindingId, CoreModule};
use crate::types::TypedProgram;

pub struct StrippedPrograms {
    written: Vec<Option<Vec<CoreModule>>>,
}

impl StrippedPrograms {
    pub fn strip(programs: &mut [LintedProgram<'_>], removed: &[Vec<BindingId>]) -> Self {
        let written = programs
            .iter_mut()
            .zip(removed)
            .map(|(linted, removed)| Self::strip_one(linted, removed))
            .collect();
        Self { written }
    }

    pub fn rechecked(
        &self,
        programs: &[LintedProgram<'_>],
        run: &mut impl ProgramRun,
    ) -> Option<Vec<Option<TypedProgram>>> {
        programs
            .iter()
            .zip(&self.written)
            .map(|(linted, written)| match written {
                None => Some(None),
                Some(_) => run.check(linted.program).ok().map(Some),
            })
            .collect()
    }

    pub fn restore(self, programs: &mut [LintedProgram<'_>]) {
        for (linted, written) in programs.iter_mut().zip(self.written) {
            if let Some(modules) = written {
                linted.program.modules = modules;
            }
        }
    }

    fn strip_one(linted: &mut LintedProgram<'_>, removed: &[BindingId]) -> Option<Vec<CoreModule>> {
        if removed.is_empty() {
            return None;
        }
        let stripped = DeclarationEdits::new(&linted.program.modules).without(removed);
        Some(mem::replace(&mut linted.program.modules, stripped))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::StrippedPrograms;
    use crate::check::ProgramChecker;
    use crate::core::{BindingId, ModulePath};
    use crate::expand::{Expander, Program};
    use crate::lint::declared_functions::DeclaredFunctions;
    use crate::lint::example_runs::ExampleRuns;
    use crate::lint::linted_program::LintedProgram;
    use crate::modules::ModuleSources;
    use crate::ownership::EnforcedOwnership;
    use crate::source::SourceMap;
    use crate::types::TypedProgram;

    const TWICE: &str = "(type twice (fn (int) int))\n\n(defn twice (x) (* 2 x))\n\n(defn main () (println (twice 4)))\n";

    struct StrippedFixture {
        program: Program,
        declared: TypedProgram,
    }

    impl StrippedFixture {
        fn expand(text: &str) -> Self {
            let mut sources = SourceMap::default();
            let entry = sources.add("fixture", text);
            let directory = PathBuf::from(".");
            let program = Expander::new(
                &mut sources,
                ModuleSources::new(directory.clone(), directory),
            )
            .expand(ModulePath::new("fixture"), entry)
            .expect("the source expands");
            let declared =
                ProgramChecker::check::<EnforcedOwnership>(&program).expect("the source checks");
            Self { program, declared }
        }

        fn declarations(&self) -> Vec<BindingId> {
            DeclaredFunctions::new(&self.program)
                .candidates()
                .iter()
                .map(|declaration| declaration.function)
                .collect()
        }

        fn modules_of(program: &Program) -> String {
            format!("{:?}", program.modules)
        }
    }

    #[test]
    fn restoring_brings_back_the_written_modules() {
        let mut fixture = StrippedFixture::expand(TWICE);
        let written = StrippedFixture::modules_of(&fixture.program);
        let removed = vec![fixture.declarations()];
        let mut programs = [LintedProgram {
            program: &mut fixture.program,
            declared: &fixture.declared,
            examples: ExampleRuns::NoModule,
            holds_unlinted_example: false,
        }];
        let stripped = StrippedPrograms::strip(&mut programs, &removed);
        assert_ne!(StrippedFixture::modules_of(programs[0].program), written);
        stripped.restore(&mut programs);
        assert_eq!(StrippedFixture::modules_of(programs[0].program), written);
    }
}
