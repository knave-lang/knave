use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::result_ties::ResultTies;
use super::ty::Scheme;
use super::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{BindingId, ParsedTemplate, ReferenceKind};

#[derive(Default)]
pub struct TemplateBorrows {
    templates: FxHashMap<BindingId, ParsedTemplate>,
    result_ties: FxHashMap<BindingId, ResultTies>,
    inline: RefCell<FxHashMap<Rc<str>, Option<ParsedTemplate>>>,
}

impl TemplateBorrows {
    pub fn register(&mut self, binding: BindingId, template: &str, scheme: &Scheme) {
        if let Ok(parsed) = ParsedTemplate::parse(template) {
            self.templates.insert(binding, parsed);
        }
        if let Some(ties) = ResultTies::of(scheme) {
            self.result_ties.insert(binding, ties);
        }
    }

    pub fn result_ties(&self, callee: &TypedExpr) -> Option<&[usize]> {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                self.result_ties.get(binding).map(ResultTies::parameters)
            }
            _ => None,
        }
    }

    pub fn borrows(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |parsed| parsed.borrows(position))
    }

    pub fn receives(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |parsed| parsed.receives(position))
    }

    pub fn keeps(&self, callee: &TypedExpr, position: usize) -> bool {
        self.reference(callee, position).is_some()
    }

    pub fn reference(&self, callee: &TypedExpr, position: usize) -> Option<ReferenceKind> {
        self.borrows(callee, position)
            .then_some(ReferenceKind::Shared)
    }

    pub fn read_template<T>(
        &self,
        callee: &TypedExpr,
        absent: T,
        read: impl Fn(&ParsedTemplate) -> T,
    ) -> T {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                self.templates.get(binding).map(read).unwrap_or(absent)
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => {
                let mut inline = self.inline.borrow_mut();
                let parsed = inline
                    .entry(template.template.clone())
                    .or_insert_with(|| ParsedTemplate::parse(&template.template).ok());
                parsed.as_ref().map(read).unwrap_or(absent)
            }
            _ => absent,
        }
    }
}
