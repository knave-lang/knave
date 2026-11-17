use rustc_hash::FxHashSet;

use super::macro_references::MacroReferences;
use super::used_imports::UsedImports;
use crate::core::{BindingId, CoreModule, ModuleId};
use crate::interpret::Closure;
use crate::source::SourceMap;

pub(in crate::expand) struct LintHandOff<'e, Macros> {
    references: MacroReferences<'e>,
    read_out: UsedImports<'e>,
    macros: Macros,
}

impl<'e, 'm, Macros> LintHandOff<'e, Macros>
where
    Macros: Iterator<Item = (BindingId, &'m Closure)>,
{
    pub(in crate::expand) fn new(
        references: MacroReferences<'e>,
        read_out: UsedImports<'e>,
        macros: Macros,
    ) -> Self {
        Self {
            references,
            read_out,
            macros,
        }
    }

    pub(in crate::expand) fn hand_to(
        self,
        modules: &mut [CoreModule],
        sources: &SourceMap,
    ) -> FxHashSet<(ModuleId, BindingId)> {
        let named = self.references.named_by(self.macros, modules);
        self.read_out.read_each(modules, sources);
        named
    }
}
