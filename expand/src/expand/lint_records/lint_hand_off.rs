use rustc_hash::FxHashSet;

use super::macro_references::MacroReferences;
use super::notes_by_module::NotesByModule;
use super::used_imports::UsedImports;
use crate::core::{BindingId, CoreModule};
use crate::expand::module_notes::ModuleNotes;
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
        notes: &mut ModuleNotes,
        sources: &SourceMap,
    ) -> FxHashSet<BindingId> {
        let named = self.references.named_by(self.macros, modules);
        NotesByModule::of(notes).hand_to(modules);
        self.read_out.read_each(modules, sources);
        named
    }
}
