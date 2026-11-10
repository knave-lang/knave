use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::typed::{TypedExpr, TypedExprKind, TypedLeaf, TypedModule};
use crate::core::{BindingId, Literal};

#[derive(Clone, Debug, Default)]
pub struct TypeTexts {
    by_definition: FxHashMap<BindingId, Vec<Rc<str>>>,
    operands: FxHashMap<BindingId, Vec<TypedExpr>>,
}

impl TypeTexts {
    pub fn keep_operands(&mut self, definition: BindingId, operands: Vec<TypedExpr>) {
        self.operands.insert(definition, operands);
    }

    pub fn operands_of(&self, definition: BindingId) -> &[TypedExpr] {
        self.operands.get(&definition).map_or(&[], Vec::as_slice)
    }

    pub fn is_empty(&self) -> bool {
        self.by_definition.is_empty()
    }

    pub fn rewrite(&self, modules: &mut [TypedModule], replacements: &TypeTexts) {
        for definition in modules
            .iter_mut()
            .flat_map(|module| &mut module.definitions)
        {
            let written = self.of(definition.binding);
            let replacing = replacements.of(definition.binding);
            Self::rewrite_expr(&mut definition.value, written, replacing);
        }
    }

    fn of(&self, definition: BindingId) -> &[Rc<str>] {
        self.by_definition
            .get(&definition)
            .map_or(&[], Vec::as_slice)
    }

    fn rewrite_expr(expr: &mut TypedExpr, written: &[Rc<str>], replacing: &[Rc<str>]) {
        if let TypedExprKind::Leaf(TypedLeaf::Literal(Literal::String(text))) = &mut expr.kind
            && let Some(position) = written.iter().position(|known| Rc::ptr_eq(known, text))
            && let Some(replacement) = replacing.get(position)
        {
            *text = Rc::clone(replacement);
        }
        for child in expr.children_mut() {
            Self::rewrite_expr(child, written, replacing);
        }
    }

    pub(super) fn record(&mut self, definition: BindingId, text: Rc<str>) {
        self.by_definition.entry(definition).or_default().push(text);
    }
}
