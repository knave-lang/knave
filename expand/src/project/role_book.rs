use rustc_hash::{FxHashMap, FxHashSet};

use super::role_note::RoleNote;
use super::role_problem::RoleProblem;
use crate::reader::Form;

#[derive(Default)]
pub(super) struct RoleBook {
    name_of_role: FxHashMap<String, String>,
    names_with_a_role: FxHashSet<String>,
}

impl RoleBook {
    pub(super) fn of(
        notes: &[Form],
        admits: impl Fn(&RoleNote) -> Result<(), RoleProblem>,
    ) -> Result<Self, RoleProblem> {
        let mut book = Self::default();
        for note in notes {
            let note = RoleNote::read(note)?;
            admits(&note)?;
            book.add(note)?;
        }
        Ok(book)
    }

    pub(super) fn taken(&mut self, role: &'static str) -> Result<String, RoleProblem> {
        self.name_of_role
            .remove(role)
            .ok_or(RoleProblem::Missing(role))
    }

    pub(super) fn finish(self) -> Result<(), RoleProblem> {
        match self.name_of_role.into_keys().next() {
            Some(role) => Err(RoleProblem::Unknown(role)),
            None => Ok(()),
        }
    }

    fn add(&mut self, note: RoleNote) -> Result<(), RoleProblem> {
        if self.name_of_role.contains_key(&note.role) {
            return Err(RoleProblem::Repeated(note.role));
        }
        if !self.names_with_a_role.insert(note.name.clone()) {
            return Err(RoleProblem::TwoRoles(note.name));
        }
        self.name_of_role.insert(note.role, note.name);
        Ok(())
    }
}
