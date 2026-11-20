use rustc_hash::FxHashMap;

use crate::core::{BindingId, ItemKind, NamedTypes};
use crate::expand::Program;

pub(super) struct ContainedTypes {
    contained: FxHashMap<BindingId, Vec<BindingId>>,
}

impl ContainedTypes {
    pub(super) fn of(program: &Program) -> Self {
        let definitions = program.modules.iter().flat_map(|module| &module.items);
        let contained = definitions
            .filter_map(|item| Self::contained_by(&item.kind))
            .collect();
        Self { contained }
    }

    pub(super) fn in_type(&self, binding: BindingId) -> &[BindingId] {
        self.contained
            .get(&binding)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn contained_by(kind: &ItemKind) -> Option<(BindingId, Vec<BindingId>)> {
        let (ItemKind::Struct(binding, ..) | ItemKind::Enum(binding, ..)) = kind else {
            return None;
        };
        let mut contained = Vec::new();
        NamedTypes::of(kind).visit(&mut |named| contained.push(named));
        Some((*binding, contained))
    }
}
