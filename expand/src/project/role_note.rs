use super::role_problem::RoleProblem;
use crate::reader::Form;

pub(super) struct RoleNote {
    pub(super) name: String,
    pub(super) role: String,
}

impl RoleNote {
    pub(super) fn read(note: &Form) -> Result<Self, RoleProblem> {
        let Some([name, role]) = note.as_list() else {
            return Err(RoleProblem::Malformed);
        };
        let symbol = |form: &Form| form.as_symbol().map(|symbol| symbol.name().to_string());
        let (name, role) = symbol(name)
            .zip(symbol(role))
            .ok_or(RoleProblem::Malformed)?;
        Ok(Self { name, role })
    }
}
