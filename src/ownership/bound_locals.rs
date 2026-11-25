use rustc_hash::FxHashMap;

use super::built_parts::BuiltParts;
use super::changed_locals::ChangedLocals;
use crate::core::{BindingId, FieldLabel};
use crate::types::{
    TypedArgument, TypedBinder, TypedBlock, TypedBodyItem, TypedControl, TypedExpr, TypedExprKind,
    TypedLambda, TypedLeaf, TypedMatch, TypedPattern, TypedPatternKind, TypedProgram,
    TypedStructExpr,
};

#[derive(Clone, Copy)]
pub enum BoundLocal<'e> {
    Value(&'e TypedExpr),
    Within(&'e TypedExpr),
    Unresolved,
}

pub struct BoundLocals<'b, 'p, 'e> {
    program: &'p TypedProgram,
    parts: &'b BuiltParts<'p>,
    changed: ChangedLocals,
    locals: FxHashMap<BindingId, BoundLocal<'e>>,
}

impl<'b, 'p, 'e> BoundLocals<'b, 'p, 'e> {
    pub fn in_argument(
        program: &'p TypedProgram,
        argument: &'e TypedExpr,
        parts: &'b BuiltParts<'p>,
    ) -> Self {
        let mut bound = Self {
            program,
            parts,
            changed: ChangedLocals::in_value(argument),
            locals: FxHashMap::default(),
        };
        bound.collect(argument);
        bound
    }

    pub fn of(&self, value: &'e TypedExpr) -> Option<BoundLocal<'e>> {
        match &value.kind {
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => self.locals.get(binding).copied(),
            TypedExprKind::Struct(TypedStructExpr::Field(owner, label)) => {
                self.field(value, owner, label)
            }
            _ => None,
        }
    }

    pub fn owns(&self, value: &TypedExpr) -> bool {
        self.locals
            .values()
            .any(|local| matches!(local, BoundLocal::Value(bound) if std::ptr::eq(*bound, value)))
    }

    fn field(
        &self,
        read: &TypedExpr,
        owner: &'e TypedExpr,
        label: &FieldLabel,
    ) -> Option<BoundLocal<'e>> {
        match self.of(owner)? {
            BoundLocal::Value(bound) => self.field_of_value(read, self.built_value(bound), label),
            BoundLocal::Within(whole) if self.program.type_holds_borrow(&read.ty) => {
                Some(BoundLocal::Within(whole))
            }
            BoundLocal::Within(_) | BoundLocal::Unresolved => None,
        }
    }

    fn field_of_value(
        &self,
        read: &TypedExpr,
        built: &'e TypedExpr,
        label: &FieldLabel,
    ) -> Option<BoundLocal<'e>> {
        let TypedExprKind::Struct(TypedStructExpr::New(structure, fields)) = &built.kind else {
            return None;
        };
        let (_, field) = fields
            .iter()
            .find(|(written, _)| written.name == label.name)?;
        let shares_the_struct_lifetime = self.program.type_holds_borrow(&read.ty)
            && self.parts.whole_struct(built, *structure).is_some();
        Some(if shares_the_struct_lifetime {
            BoundLocal::Within(built)
        } else {
            BoundLocal::Value(field)
        })
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
                let selected = self.parts.whole_struct(scrutinee, *structure);
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

    fn built_value(&self, value: &'e TypedExpr) -> &'e TypedExpr {
        let value = self.bound_value(value);
        match &value.kind {
            TypedExprKind::Borrow(_, referent) => self.built_value(referent),
            _ => value,
        }
    }

    fn loop_binders(&mut self, body: &'e TypedLambda, arguments: &'e [TypedArgument]) {
        for (parameter, argument) in body.params.iter().zip(arguments) {
            self.bind_value(parameter, &argument.value);
        }
        self.lets(&body.body);
    }

    fn lets(&mut self, block: &'e TypedBlock) {
        for item in &block.items {
            if let TypedBodyItem::Let(binder, value) = item {
                self.bind_value(binder, value);
            }
        }
    }

    fn bind_value(&mut self, binder: &TypedBinder, value: &'e TypedExpr) {
        let local = if self.changes_what_it_borrows(binder) {
            BoundLocal::Unresolved
        } else {
            BoundLocal::Value(value)
        };
        self.locals.insert(binder.binding, local);
    }

    fn changes_what_it_borrows(&self, binder: &TypedBinder) -> bool {
        self.changed.contains(binder.binding) && self.program.type_holds_borrow(&binder.ty)
    }
}
