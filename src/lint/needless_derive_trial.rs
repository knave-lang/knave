use super::copy_types::CopyTypes;
use super::declaration_edits::DeclarationEdits;
use super::program_run::ProgramRun;
use super::removal_trial::{RemovalTrial, TrialVerdict};
use super::trial_face::TrialFace;
use super::trial_program::TrialProgram;
use crate::core::BindingId;
use crate::expand::Program;
use crate::types::TypedProgram;

pub struct NeedlessDeriveTrial<'r, Run: ProgramRun> {
    run: &'r mut Run,
    program: TrialProgram<'r>,
    declared: TrialFace<&'r TypedProgram>,
}

impl<'r, Run: ProgramRun> NeedlessDeriveTrial<'r, Run> {
    pub fn new(run: &'r mut Run, program: &'r mut Program, declared: &'r TypedProgram) -> Self {
        let program = TrialProgram::new(program);
        let declared = TrialFace::new(declared, program.definitions());
        Self {
            run,
            program,
            declared,
        }
    }

    fn face_keeps(&mut self, stripped: &mut TrialFace) -> bool {
        let definitions = self.program.definitions();
        self.declared
            .changed_signatures(stripped, definitions)
            .is_empty()
            && DeriveAttributes::removed_from(self.declared.rust(&*self.run))
                == DeriveAttributes::removed_from(stripped.rust(&*self.run))
    }
}

impl<Run: ProgramRun> RemovalTrial for NeedlessDeriveTrial<'_, Run> {
    type Kept = Vec<BindingId>;

    fn verdict(&mut self, removed: &[BindingId]) -> TrialVerdict<Vec<BindingId>> {
        let modules = DeclarationEdits::new(self.program.written()).without_derives(removed);
        let Ok(stripped) = self.program.checked_with(modules, &mut *self.run) else {
            return TrialVerdict::Rejected(Vec::new());
        };
        let losing = GivenTraits::new(self.declared.typed(), &stripped).lost_by(removed);
        if !losing.is_empty() {
            return TrialVerdict::Rejected(losing);
        }
        let copies = CopyTypes::new(&stripped).possibly_copy(removed);
        let mut stripped = TrialFace::new(stripped, self.program.definitions());
        if self.face_keeps(&mut stripped) {
            TrialVerdict::Kept(copies)
        } else {
            TrialVerdict::Rejected(Vec::new())
        }
    }
}

struct GivenTraits<'t> {
    declared: &'t TypedProgram,
    stripped: &'t TypedProgram,
}

impl<'t> GivenTraits<'t> {
    fn new(declared: &'t TypedProgram, stripped: &'t TypedProgram) -> Self {
        Self { declared, stripped }
    }

    fn lost_by(&self, removed: &[BindingId]) -> Vec<BindingId> {
        removed
            .iter()
            .copied()
            .filter(|target| {
                let kept = Self::of(self.stripped, *target);
                Self::of(self.declared, *target)
                    .iter()
                    .any(|given| !kept.contains(given))
            })
            .collect()
    }

    fn of(typed: &TypedProgram, target: BindingId) -> &[BindingId] {
        let types = typed.declarations.types();
        types
            .structs()
            .get(target)
            .map(|definition| definition.derived_traits.as_slice())
            .or_else(|| {
                types
                    .enums()
                    .get(target)
                    .map(|definition| definition.derived_traits.as_slice())
            })
            .unwrap_or_default()
    }
}

struct DeriveAttributes;

impl DeriveAttributes {
    const OPENING: &str = "#[derive(";

    fn removed_from(rust: &Option<String>) -> Option<String> {
        rust.as_ref().map(|code| {
            code.lines()
                .filter(|line| !line.trim_start().starts_with(Self::OPENING))
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
}
