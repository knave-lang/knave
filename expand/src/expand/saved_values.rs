use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::saved_index::{BindingIndex, SavedWords};
use crate::core::BindingId;

#[derive(Default, Serialize)]
pub struct ValueTables {
    rows: Vec<u8>,
    encoded: Vec<u8>,
}

impl ValueTables {
    const START: usize = 1;
    const END: usize = 2;
    const WORDS: usize = 3;

    pub fn of<T: Serialize>(values: impl IntoIterator<Item = (BindingId, T)>) -> Self {
        let mut tables = Self::default();
        for (binding, value) in values {
            tables.add(binding, &value);
        }
        tables
    }

    fn add<T: Serialize>(&mut self, binding: BindingId, value: &T) {
        let start = SavedWords::index(self.encoded.len());
        let encoded = bincode::serialize(value).expect("every saved prelude value serializes");
        self.encoded.extend(encoded);
        let end = SavedWords::index(self.encoded.len());
        SavedWords::write(&[binding.index(), start, end], &mut self.rows);
    }
}

pub struct SavedValues<T> {
    index: BindingIndex,
    encoded: &'static [u8],
    decodes: PhantomData<fn() -> T>,
}

impl<T> Clone for SavedValues<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SavedValues<T> {}

impl<T> Default for SavedValues<T> {
    fn default() -> Self {
        Self::borrowing(&[], &[])
    }
}

impl<'de: 'static, T> serde::Deserialize<'de> for SavedValues<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (rows, encoded) = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self::borrowing(rows, encoded))
    }
}

impl<T> SavedValues<T> {
    fn borrowing(rows: &'static [u8], encoded: &'static [u8]) -> Self {
        Self {
            index: BindingIndex::new(rows, ValueTables::WORDS),
            encoded,
            decodes: PhantomData,
        }
    }
}

impl<T: DeserializeOwned> SavedValues<T> {
    pub fn value_of(&self, binding: BindingId) -> Option<T> {
        let row = self.index.rows_of(binding).next()?;
        let start = self.index.word(row, ValueTables::START) as usize;
        let end = self.index.word(row, ValueTables::END) as usize;
        let value = bincode::deserialize(&self.encoded[start..end]);
        Some(value.expect("a saved prelude value decodes"))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::{BindingId, BindingKind, ModulePath};
    use crate::expand::{Expander, PreludeState};
    use crate::interpret::Closure;
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
            let bindings = &state.tables().bindings;
            let mut macros = bindings.ids().filter(|binding| {
                bindings.decoded(*binding).is_some_and(|info| {
                    info.name() == name && matches!(info.kind(), BindingKind::Macro(_))
                })
            });
            macros.next().expect("the prelude defines the macro")
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
    fn an_unknown_binding_has_no_saved_value() {
        let unknown = BindingId::from_index(u32::MAX);
        assert!(
            super::SavedValues::<Closure>::default()
                .value_of(unknown)
                .is_none()
        );
    }
}
