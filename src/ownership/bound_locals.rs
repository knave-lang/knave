use rustc_hash::FxHashMap;

use super::built_parts::BuiltParts;
use crate::core::BindingId;
use crate::types::{
    TypedArgument, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind, TypedLambda,
    TypedLeaf, TypedMatch, TypedPattern, TypedPatternKind,
};

#[derive(Clone, Copy)]
pub enum BoundLocal<'e> {
    Value(&'e TypedExpr),
    LoopBinder(&'e TypedExpr),
    Within(&'e TypedExpr),
    Unresolved,
}

pub struct BoundLocals<'b, 'p, 'e> {
    parts: &'b BuiltParts<'p>,
    locals: FxHashMap<BindingId, BoundLocal<'e>>,
}

impl<'b, 'p, 'e> BoundLocals<'b, 'p, 'e> {
    pub fn in_argument(argument: &'e TypedExpr, parts: &'b BuiltParts<'p>) -> Self {
        let mut bound = Self {
            parts,
            locals: FxHashMap::default(),
        };
        bound.collect(argument);
        bound
    }

    pub fn of(&self, value: &TypedExpr) -> Option<BoundLocal<'e>> {
        match value.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.locals.get(&binding).copied(),
            _ => None,
        }
    }

    pub fn owns(&self, value: &TypedExpr) -> bool {
        self.locals
            .values()
            .any(|local| matches!(local, BoundLocal::Value(bound) if std::ptr::eq(*bound, value)))
    }

    fn collect(&mut self, expr: &'e TypedExpr) {
        self.bind(expr);
        for child in expr.children() {
            self.collect(child);
        }
    }

    fn bind(&mut self, expr: &'e TypedExpr) {
        match &expr.kind {
            TypedExprKind::Control(TypedControl::Block(block)) => self.lets(block),
            TypedExprKind::Control(TypedControl::Match(matched)) => self.arms(matched),
            TypedExprKind::Call(callee, arguments)
                if let TypedExprKind::Fn(body) = &callee.kind =>
            {
                self.loop_binders(body, arguments)
            }
            _ => {}
        }
    }

    fn arms(&mut self, matched: &'e TypedMatch) {
        for arm in &matched.arms {
            self.pattern(&arm.pattern, &matched.value);
            self.lets(&arm.body);
        }
    }

    fn pattern(&mut self, pattern: &'e TypedPattern, scrutinee: &'e TypedExpr) {
        match &pattern.kind {
            TypedPatternKind::Bind(binder) => {
                self.locals
                    .insert(binder.binding, BoundLocal::Within(scrutinee));
            }
            TypedPatternKind::Variant(variant, parts) => {
                let built = self.bound_value(scrutinee);
                for (position, part) in parts.iter().enumerate() {
                    let selected = self.parts.variant_part(built, *variant, position);
                    self.part(part, selected);
                }
            }
            TypedPatternKind::Struct(structure, parts) => {
                let selected = self.parts.field_part(scrutinee, *structure);
                for part in parts {
                    self.part(part, selected);
                }
            }
            TypedPatternKind::Wildcard | TypedPatternKind::Literal(..) => {}
        }
    }

    fn part(&mut self, pattern: &'e TypedPattern, selected: Option<&'e TypedExpr>) {
        match selected {
            Some(value) => self.pattern(pattern, value),
            None => {
                for binder in pattern.binders() {
                    self.locals.insert(binder.binding, BoundLocal::Unresolved);
                }
            }
        }
    }

    fn bound_value(&self, value: &'e TypedExpr) -> &'e TypedExpr {
        match self.of(value) {
            Some(BoundLocal::Value(bound)) => self.bound_value(bound),
            _ => value,
        }
    }

    fn loop_binders(&mut self, body: &'e TypedLambda, arguments: &'e [TypedArgument]) {
        for (parameter, argument) in body.params.iter().zip(arguments) {
            self.locals
                .insert(parameter.binding, BoundLocal::LoopBinder(&argument.value));
        }
        self.lets(&body.body);
    }

    fn lets(&mut self, block: &'e TypedBlock) {
        for item in &block.items {
            if let TypedBodyItem::Let(binder, value) = item {
                self.locals.insert(binder.binding, BoundLocal::Value(value));
            }
        }
    }
}
