use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::core::{BindingArena, BindingId};
use crate::source::Span;
use crate::types::{FieldPlace, TypedExpr, TypedExprKind};

#[derive(Clone)]
pub struct MovedPlace {
    written: Rc<str>,
    borrowed: Option<Rc<str>>,
}

impl MovedPlace {
    pub fn of(place: &FieldPlace, bindings: &BindingArena) -> Self {
        let written = place.written(bindings).into();
        let borrowed = Some(Self::borrowed_root(place, bindings));
        Self { written, borrowed }
    }

    pub fn whole(name: &str) -> Self {
        Self {
            written: name.into(),
            borrowed: Some(format!("(ref {name})").into()),
        }
    }

    pub fn captured(closure: &TypedExpr, capture: BindingId, bindings: &BindingArena) -> Self {
        let moved_part = match &closure.kind {
            TypedExprKind::Fn(lambda) => lambda.facts.moved_capture.and_then(|(_, span)| {
                let mut body = lambda.body.expressions();
                body.find_map(|expr| Self::field_read_at(expr, span, bindings))
            }),
            _ => None,
        };
        moved_part.unwrap_or_else(|| Self::whole(bindings.get(capture).name()))
    }

    fn field_read_at(expr: &TypedExpr, span: Span, bindings: &BindingArena) -> Option<Self> {
        match expr.field_place().filter(|place| place.reads_field()) {
            Some(place) if expr.span == span => Some(Self::of(&place, bindings).owned()),
            _ => expr
                .children()
                .into_iter()
                .find_map(|child| Self::field_read_at(child, span, bindings)),
        }
    }

    fn borrowed_root(place: &FieldPlace, bindings: &BindingArena) -> Rc<str> {
        let root = format!("(ref {})", bindings.get(place.root).name());
        place.path.iter().fold(root, FieldPlace::field_text).into()
    }

    pub fn owned_unless_bound(self, bound_values: &FxHashSet<Span>, span: Span) -> Self {
        if bound_values.contains(&span) {
            self
        } else {
            self.owned()
        }
    }

    pub fn owned(self) -> Self {
        Self {
            borrowed: None,
            ..self
        }
    }

    pub fn written(&self) -> &str {
        &self.written
    }

    pub fn borrowed(&self) -> Option<&str> {
        self.borrowed.as_deref()
    }
}
