use super::linted_program::LintedProgram;
use super::program_position::ProgramPosition;
use super::removal::Removal;
use crate::core::BindingId;

pub trait ProgramTrials {
    type Kept;

    fn removal(
        &mut self,
        position: ProgramPosition,
        linted: &mut LintedProgram<'_>,
        tried: Vec<BindingId>,
    ) -> Removal<Self::Kept>;
}
