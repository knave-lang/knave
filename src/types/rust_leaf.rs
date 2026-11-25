use std::rc::Rc;

use super::ty::Type;
use super::typed::{TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::{RustTemplate, TypeExpr, TypeExprKind};
use crate::source::Span;

pub struct RustLeaf {
    text: String,
}

impl RustLeaf {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn typed(self, ty: Type, span: Span) -> TypedExpr {
        let template = RustTemplate {
            template: Rc::from(self.text),
            type_expr: TypeExpr::new(TypeExprKind::Hole, span),
            clauses: Vec::new(),
        };
        TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Rust(Rc::new(template))),
            ty,
            span,
        }
    }
}
