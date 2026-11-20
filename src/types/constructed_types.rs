use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::{BindingId, Expr, ExprKind, ImplMethod, ItemKind, References, StructExpr};
use crate::expand::Program;

pub(super) struct ConstructedTypes<'r> {
    enum_of_variant: FxHashMap<BindingId, BindingId>,
    built_by_definition: FxHashMap<BindingId, Vec<BindingId>>,
    references: &'r FxHashMap<BindingId, References>,
}

impl<'r> ConstructedTypes<'r> {
    pub(super) fn of(program: &Program, references: &'r FxHashMap<BindingId, References>) -> Self {
        let items: Vec<&ItemKind> = program
            .modules
            .iter()
            .flat_map(|module| &module.items)
            .map(|item| &item.kind)
            .collect();
        let mut constructed = Self {
            enum_of_variant: Self::enum_of_variant(&items),
            built_by_definition: FxHashMap::default(),
            references,
        };
        constructed.built_by_definition = constructed.built_by_definitions(&items);
        constructed
    }

    pub(super) fn by_method(&self, method: &ImplMethod) -> Vec<BindingId> {
        let mut built = self.in_expressions(method.lambda.body.expressions());
        let mut visited = FxHashSet::default();
        let mut pending = self.globals_of(method.binding).to_vec();
        while let Some(global) = pending.pop() {
            let Some(built_by_global) = self.built_by_definition.get(&global) else {
                continue;
            };
            if visited.insert(global) {
                built.extend(built_by_global);
                pending.extend(self.globals_of(global));
            }
        }
        built
    }

    fn enum_of_variant(items: &[&ItemKind]) -> FxHashMap<BindingId, BindingId> {
        items
            .iter()
            .filter_map(|kind| match kind {
                ItemKind::Enum(binding, _, variants, ..) => Some((*binding, variants)),
                _ => None,
            })
            .flat_map(|(binding, variants)| {
                variants
                    .iter()
                    .map(move |variant| (variant.binding, binding))
            })
            .collect()
    }

    fn built_by_definitions(&self, items: &[&ItemKind]) -> FxHashMap<BindingId, Vec<BindingId>> {
        items
            .iter()
            .filter_map(|kind| match kind {
                ItemKind::Def(binding, value) => Some((
                    *binding,
                    self.in_expressions(std::iter::once(value.as_ref())),
                )),
                _ => None,
            })
            .collect()
    }

    fn globals_of(&self, binding: BindingId) -> &[BindingId] {
        self.references
            .get(&binding)
            .map(References::globals)
            .unwrap_or_default()
    }

    fn in_expressions<'e>(&self, expressions: impl Iterator<Item = &'e Expr>) -> Vec<BindingId> {
        let mut constructed = Vec::new();
        let mut pending: Vec<&Expr> = expressions.collect();
        while let Some(expr) = pending.pop() {
            constructed.extend(self.constructed_by(expr));
            pending.extend(expr.children());
        }
        constructed
    }

    fn constructed_by(&self, expr: &Expr) -> Option<BindingId> {
        match expr.kind() {
            ExprKind::Struct(StructExpr::New(binding, _)) => Some(*binding),
            ExprKind::Variant(variant, _) | ExprKind::LabeledVariant(variant, _) => {
                self.enum_of_variant.get(variant).copied()
            }
            _ => None,
        }
    }
}
