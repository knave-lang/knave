use crate::diagnostic::Diagnostic;

pub trait ConflictWording {
    fn changed_while_lent(&self) -> Diagnostic;
    fn moved_while_lent(&self) -> Diagnostic;
    fn borrowed_while_lent_as_mut(&self) -> Diagnostic;
}
