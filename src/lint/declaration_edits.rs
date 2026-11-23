use rustc_hash::FxHashMap;

use super::by_value_signature::ByValueSignature;
use crate::core::{BindingId, CoreModule, Item, ItemKind};

pub struct DeclarationEdits<'w> {
    written: &'w [CoreModule],
}

impl<'w> DeclarationEdits<'w> {
    pub fn new(written: &'w [CoreModule]) -> Self {
        Self { written }
    }

    pub fn without(&self, removed: &[BindingId]) -> Vec<CoreModule> {
        self.edited(|item| match &item.kind {
            ItemKind::Signature(function, _) if removed.contains(function) => None,
            _ => Some(item.clone()),
        })
    }

    pub fn without_derives(&self, removed: &[BindingId]) -> Vec<CoreModule> {
        self.edited(|item| match &item.kind {
            ItemKind::Derive(target, _, _) if removed.contains(target) => None,
            _ => Some(item.clone()),
        })
    }

    pub fn passing_by_value(
        &self,
        rewrites: &FxHashMap<BindingId, ByValueSignature>,
    ) -> Vec<CoreModule> {
        self.edited(|item| match &item.kind {
            ItemKind::Signature(function, _) if rewrites.contains_key(function) => Some(Item {
                kind: ItemKind::Signature(*function, rewrites[function].signature.clone()),
                ..item.clone()
            }),
            _ => Some(item.clone()),
        })
    }

    fn edited(&self, edit: impl Fn(&Item) -> Option<Item>) -> Vec<CoreModule> {
        self.written
            .iter()
            .map(|module| {
                let mut edited = module.clone();
                edited.items = module.items.iter().filter_map(&edit).collect();
                edited
            })
            .collect()
    }
}
