use super::problems::ExpandProblems;
use crate::diagnostic::Diagnostic;
use crate::reader::Symbol;

#[derive(Clone, Copy, Default)]
pub(super) struct TypeNote {
    shown: bool,
}

impl TypeNote {
    pub(super) fn shown() -> Self {
        Self { shown: true }
    }

    pub(super) fn noted(&mut self, unbound: Diagnostic, type_name: Option<&Symbol>) -> Diagnostic {
        let Some(symbol) = type_name.filter(|_| !self.shown) else {
            return unbound;
        };
        self.shown = true;
        unbound.with_note(ExpandProblems::type_in_value_note(symbol.name()))
    }
}
