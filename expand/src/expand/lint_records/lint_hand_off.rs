use rustc_hash::FxHashSet;

use super::macro_references::MacroReferences;
use crate::core::{BindingId, CoreModule, ModuleId};
use crate::interpret::Closure;

pub(in crate::expand) struct LintHandOff<'e, Macros> {
    references: MacroReferences<'e>,
    macros: Macros,
}

impl<'e, 'm, Macros> LintHandOff<'e, Macros>
where
    Macros: Iterator<Item = (BindingId, &'m Closure)>,
{
    pub(in crate::expand) fn new(references: MacroReferences<'e>, macros: Macros) -> Self {
        Self { references, macros }
    }

    pub(in crate::expand) fn hand_to(
        self,
        modules: &[CoreModule],
    ) -> FxHashSet<(ModuleId, BindingId)> {
        self.references.named_by(self.macros, modules)
    }
}
