use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::registered_template::RegisteredTemplate;
use super::ty::Scheme;
use super::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{BindingId, ReferenceKind};

#[derive(Default)]
pub struct TemplateBorrows {
    templates: FxHashMap<BindingId, RegisteredTemplate>,
    inline: RefCell<FxHashMap<Rc<str>, Option<RegisteredTemplate>>>,
}

impl TemplateBorrows {
    pub fn register(&mut self, binding: BindingId, template: &str, scheme: &Scheme) {
        if let Some(registered) = RegisteredTemplate::parse(template, Some(scheme)) {
            self.templates.insert(binding, registered);
        }
    }

    pub fn borrows(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |template| {
            template.parsed().borrows(position)
        })
    }

    pub fn receives(&self, callee: &TypedExpr, position: usize) -> bool {
        self.read_template(callee, false, |template| {
            template.parsed().receives(position)
        })
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
        read: impl Fn(&RegisteredTemplate) -> T,
    ) -> T {
        match &callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(binding)) => {
                self.templates.get(binding).map(read).unwrap_or(absent)
            }
            TypedExprKind::Leaf(TypedLeaf::Rust(template)) => {
                let mut inline = self.inline.borrow_mut();
                let parsed = inline
                    .entry(template.template.clone())
                    .or_insert_with(|| RegisteredTemplate::parse(&template.template, None));
                parsed.as_ref().map(read).unwrap_or(absent)
            }
            _ => absent,
        }
    }
}
