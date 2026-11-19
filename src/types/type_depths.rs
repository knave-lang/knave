use rustc_hash::FxHashMap;

use crate::core::{BindingId, ItemKind, NamedTypes, TypeExpr};
use crate::expand::Program;

pub(super) struct TypeDepths {
    contained: FxHashMap<BindingId, Vec<BindingId>>,
    depths: FxHashMap<BindingId, usize>,
}

impl TypeDepths {
    pub(super) fn of(program: &Program) -> Self {
        let definitions = program.modules.iter().flat_map(|module| &module.items);
        let contained = definitions
            .filter_map(|item| Self::contained_by(&item.kind))
            .collect();
        Self {
            contained,
            depths: FxHashMap::default(),
        }
    }

    pub(super) fn of_type(&mut self, type_expr: &TypeExpr) -> usize {
        type_expr
            .named_bindings()
            .into_iter()
            .map(|named| self.of_binding(named))
            .max()
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

    fn of_binding(&mut self, binding: BindingId) -> usize {
        if let Some(depth) = self.depths.get(&binding) {
            return *depth;
        }
        self.depths.insert(binding, 0);
        let contained = self.contained.get(&binding).cloned().unwrap_or_default();
        let deepest = contained
            .into_iter()
            .map(|named| self.of_binding(named))
            .max();
        let depth = deepest.unwrap_or_default() + 1;
        self.depths.insert(binding, depth);
        depth
    }
}
