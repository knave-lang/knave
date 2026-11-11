use crate::diagnostic::Diagnostic;

pub trait Warnings {
    fn report_warning(&mut self, warning: Diagnostic);
}
