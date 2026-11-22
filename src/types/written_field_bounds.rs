use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::core::{CoreModule, FieldBoundNames, ItemKind, References};

pub struct WrittenFieldBounds;

impl WrittenFieldBounds {
    pub fn of<'r>(
        modules: &[CoreModule],
        definitions: impl Iterator<Item = &'r References>,
    ) -> FxHashSet<Rc<str>> {
        let items = modules.iter().flat_map(|module| &module.items);
        let signatures = items.filter_map(|item| match &item.kind {
            ItemKind::Signature(_, signature) => Some(FieldBoundNames::of(signature)),
            _ => None,
        });
        let bodies =
            definitions.map(|references| references.written_types().field_bounds().to_vec());
        signatures.chain(bodies).flatten().collect()
    }
}
