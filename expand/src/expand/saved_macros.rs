use std::rc::Rc;

use super::saved_index::{BindingIndex, SavedWords};
use crate::core::BindingId;
use crate::interpret::Closure;

#[derive(Default, serde::Serialize)]
pub struct MacroTables {
    rows: Vec<u8>,
    closures: Vec<u8>,
}

impl MacroTables {
    const START: usize = 1;
    const END: usize = 2;
    const WORDS: usize = 3;

    pub fn of(ready: Vec<(BindingId, Rc<Closure>)>) -> Self {
        let mut tables = Self::default();
        for (binding, closure) in ready {
            tables.add(binding, &closure);
        }
        tables
    }

    fn add(&mut self, binding: BindingId, closure: &Closure) {
        let start = SavedWords::index(self.closures.len());
        let encoded = bincode::serialize(closure).expect("every prelude macro serializes");
        self.closures.extend(encoded);
        let end = SavedWords::index(self.closures.len());
        SavedWords::write(&[binding.index(), start, end], &mut self.rows);
    }
}

#[derive(Clone, Copy)]
pub struct SavedMacros {
    index: BindingIndex,
    closures: &'static [u8],
}

impl Default for SavedMacros {
    fn default() -> Self {
        Self::borrowing(&[], &[])
    }
}

impl<'de: 'static> serde::Deserialize<'de> for SavedMacros {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (rows, closures) = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self::borrowing(rows, closures))
    }
}

impl SavedMacros {
    fn borrowing(rows: &'static [u8], closures: &'static [u8]) -> Self {
        Self {
            index: BindingIndex::new(rows, MacroTables::WORDS),
            closures,
        }
    }

    pub fn closure_of(&self, binding: BindingId) -> Option<Rc<Closure>> {
        let row = self.index.rows_of(binding).next()?;
        let start = self.index.word(row, MacroTables::START) as usize;
        let end = self.index.word(row, MacroTables::END) as usize;
        let closure = bincode::deserialize(&self.closures[start..end]);
        Some(Rc::new(closure.expect("a saved prelude macro decodes")))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::{BindingId, BindingKind, ModulePath};
    use crate::expand::{Expander, PreludeState};
    use crate::modules::ModuleSources;
    use crate::source::SourceMap;

    struct SavedMacroFixture;

    impl SavedMacroFixture {
        const ENTRY: &str = "macro-user";
        const CALLS_DEFN: &str = "(defn main ())";
        const USED: &str = "defn";
        const UNUSED: &str = "assoc-in";

        fn loaded_state() -> PreludeState {
            PreludeState::load(PreludeState::build().save().leak())
                .expect("the saved prelude loads")
        }

        fn macro_binding(state: &PreludeState, name: &str) -> BindingId {
            let bindings = state.tables().bindings.iter().enumerate();
            let mut macros = bindings.filter(|(_, info)| {
                info.name() == name && matches!(info.kind(), BindingKind::Macro(_))
            });
            let (index, _) = macros.next().expect("the prelude defines the macro");
            BindingId::from_index(u32::try_from(index).expect("few bindings"))
        }

        fn readiness_after_expanding(names: [&str; 2]) -> [bool; 2] {
            let state = Self::loaded_state();
            let bindings = names.map(|name| Self::macro_binding(&state, name));
            let mut sources = SourceMap::default();
            let entry = sources.add(Self::ENTRY, Self::CALLS_DEFN);
            let module_sources = ModuleSources::new(PathBuf::new(), PathBuf::new());
            let mut expander = Expander::with_prelude(&mut sources, module_sources, state);
            expander.load_source_module(ModulePath::new(Self::ENTRY), &[entry]);
            bindings.map(|binding| expander.tables.macros.holds_ready(binding))
        }
    }

    #[test]
    fn a_saved_state_keeps_no_ready_macro_in_its_table() {
        let state = SavedMacroFixture::loaded_state();
        let defn = SavedMacroFixture::macro_binding(&state, SavedMacroFixture::USED);
        assert!(!state.tables().macros.holds_ready(defn));
    }

    #[test]
    fn a_called_macro_decodes_and_an_uncalled_one_does_not() {
        let names = [SavedMacroFixture::USED, SavedMacroFixture::UNUSED];
        let readiness = SavedMacroFixture::readiness_after_expanding(names);
        assert_eq!(readiness, [true, false]);
    }

    #[test]
    fn an_unknown_binding_has_no_saved_macro() {
        let unknown = BindingId::from_index(u32::MAX);
        assert!(super::SavedMacros::default().closure_of(unknown).is_none());
    }
}
