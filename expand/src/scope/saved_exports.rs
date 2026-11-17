use super::name_key::NameKey;
use super::saved_keys::{SavedKeys, SavedKeysBuilder};
use super::saved_words::{SavedWords, SavedWordsBuilder};
use crate::core::{BindingId, Namespace};

#[derive(serde::Serialize)]
pub struct ExportBytes {
    keys: SavedKeysBuilder,
    bindings: SavedWordsBuilder<1>,
}

impl ExportBytes {
    pub fn of<'a>(exports: impl ExactSizeIterator<Item = (&'a str, Namespace, BindingId)>) -> Self {
        let mut bytes = Self {
            keys: SavedKeysBuilder::for_count(exports.len()),
            bindings: SavedWordsBuilder::default(),
        };
        for (name, namespace, binding) in exports {
            bytes.keys.add(NameKey::new(name, namespace));
            bytes.bindings.push([binding.index() as usize]);
        }
        bytes
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavedExports {
    keys: SavedKeys,
    #[serde(borrow)]
    bindings: &'static [u8],
}

impl SavedExports {
    pub fn get(&self, name: &str, namespace: Namespace) -> Option<BindingId> {
        let position = self.position(NameKey::new(name, namespace))?;
        let (_, _, binding) = self.export(position);
        Some(binding)
    }

    pub fn count(&self) -> usize {
        self.keys.count()
    }

    pub(super) fn position(&self, key: NameKey<'_>) -> Option<usize> {
        self.keys.position(key)
    }

    pub fn export(&self, position: usize) -> (&'static str, Namespace, BindingId) {
        let (name, namespace) = self.keys.key(position);
        let [binding] = SavedWords::<1>::new(self.bindings).row(position);
        (name, namespace, BindingId::from_index(binding))
    }
}
