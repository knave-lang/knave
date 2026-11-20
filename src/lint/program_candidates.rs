use super::program_position::ProgramPosition;

pub struct ProgramCandidates<Declaration> {
    per_program: Vec<Vec<Declaration>>,
}

impl<Declaration> ProgramCandidates<Declaration> {
    pub fn new(per_program: Vec<Vec<Declaration>>) -> Self {
        Self { per_program }
    }

    pub fn tries(&self, position: ProgramPosition) -> bool {
        self.per_program
            .get(position.index())
            .is_some_and(|candidates| !candidates.is_empty())
    }

    pub fn tries_any(&self) -> bool {
        self.per_program
            .iter()
            .any(|candidates| !candidates.is_empty())
    }

    pub fn into_lists(self) -> Vec<Vec<Declaration>> {
        self.per_program
    }
}
