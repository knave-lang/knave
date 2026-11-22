use std::rc::Rc;

use super::{TypeExpr, TypeExprKind};

pub struct FieldBoundNames;

impl FieldBoundNames {
    pub fn of(written: &TypeExpr) -> Vec<Rc<str>> {
        let mut names = Vec::new();
        Self::collect(written, &mut names);
        names
    }

    fn collect(written: &TypeExpr, names: &mut Vec<Rc<str>>) {
        let parts = match written.kind() {
            TypeExprKind::Where(binders, body) => {
                let bounds = binders.iter().flat_map(|binder| &binder.bounds);
                names.extend(bounds.filter_map(|bound| bound.field.clone()));
                vec![body.as_ref()]
            }
            TypeExprKind::With(implementor, argument) => {
                vec![implementor.as_ref(), argument.as_ref()]
            }
            _ => written.parts(),
        };
        for part in parts {
            Self::collect(part, names);
        }
    }
}
