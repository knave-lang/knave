use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind};
use crate::types::{
    Pipelines, Type, TypedBinder, TypedBodyItem, TypedExpr, TypedExprKind, TypedLambda, TypedLeaf,
    TypedProgram,
};

pub struct CapturedCopies<'a> {
    changed_copies: &'a FxHashSet<BindingId>,
}

impl<'a> CapturedCopies<'a> {
    pub fn new(changed_copies: &'a FxHashSet<BindingId>) -> Self {
        Self { changed_copies }
    }

    pub fn apply(&self, program: &mut TypedProgram) {
        if self.changed_copies.is_empty() {
            return;
        }
        for definition in program
            .modules
            .iter_mut()
            .flat_map(|module| module.definitions.iter_mut())
        {
            if let TypedExprKind::Fn(lambda) = &mut definition.value.kind
                && !Self::takes_opaque_parameter(lambda, Pipelines::new(&program.rust_types))
            {
                self.copy_into_returned_closure(Rc::make_mut(lambda), &mut program.bindings);
            }
        }
    }

    fn copy_into_returned_closure(&self, function: &mut TypedLambda, bindings: &mut BindingArena) {
        let read_only: Vec<(BindingId, Type)> = match function.body.items.last() {
            Some(TypedBodyItem::Expr(closure)) if matches!(closure.kind, TypedExprKind::Fn(_)) => {
                function
                    .params
                    .iter()
                    .filter(|param| self.changed_copies.contains(&param.binding))
                    .filter(|param| closure.mentions_local(param.binding))
                    .filter(|param| Self::only_copies(closure, param.binding))
                    .map(|param| (param.binding, param.ty.referent().clone()))
                    .collect()
            }
            _ => Vec::new(),
        };
        for (param, value_type) in read_only {
            self.capture_copy(function, bindings, param, value_type);
        }
    }

    fn capture_copy(
        &self,
        function: &mut TypedLambda,
        bindings: &mut BindingArena,
        param: BindingId,
        value_type: Type,
    ) {
        let info = bindings.get(param);
        let span = info.span().expect("a parameter has a span");
        let copy = bindings.add(BindingInfo::new(
            info.name().into(),
            BindingKind::Local,
            Some(span),
        ));
        let Some(TypedBodyItem::Expr(closure)) = function.body.items.last_mut() else {
            unreachable!("the caller found the returned closure")
        };
        Self::read_copy(closure, param, copy);
        let read = TypedExpr {
            kind: TypedExprKind::Deref(Box::new(TypedExpr {
                kind: TypedExprKind::Leaf(TypedLeaf::Local(param)),
                ty: Type::mutable(value_type.clone()),
                span,
            })),
            ty: value_type.clone(),
            span,
        };
        let binder = TypedBinder {
            binding: copy,
            ty: value_type,
            span,
            annotated: false,
        };
        let tail = function.body.items.len() - 1;
        function
            .body
            .items
            .insert(tail, TypedBodyItem::Let(binder, read));
    }

    fn takes_opaque_parameter(function: &TypedLambda, pipelines: Pipelines) -> bool {
        function
            .params
            .iter()
            .any(|param| matches!(param.ty, Type::Function(..)) || pipelines.is_pipeline(&param.ty))
    }

    fn only_copies(expr: &TypedExpr, param: BindingId) -> bool {
        match &expr.kind {
            TypedExprKind::Deref(value) if Self::is_local(value, param) => true,
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) => *binding != param,
            _ => expr
                .children()
                .into_iter()
                .all(|child| Self::only_copies(child, param)),
        }
    }

    fn read_copy(expr: &mut TypedExpr, param: BindingId, copy: BindingId) {
        if let TypedExprKind::Deref(value) = &expr.kind
            && Self::is_local(value, param)
        {
            expr.kind = TypedExprKind::Leaf(TypedLeaf::Local(copy));
            return;
        }
        for child in expr.children_mut() {
            Self::read_copy(child, param, copy);
        }
    }

    fn is_local(expr: &TypedExpr, param: BindingId) -> bool {
        matches!(expr.kind, TypedExprKind::Leaf(TypedLeaf::Local(binding)) if binding == param)
    }
}
