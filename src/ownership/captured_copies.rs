use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::core::{BindingArena, BindingId, BindingInfo, BindingKind};
use crate::types::{
    Pipelines, ReturnedValues, Type, TypedBinder, TypedBlock, TypedBodyItem, TypedControl,
    TypedExpr, TypedExprKind, TypedLambda, TypedLeaf, TypedProgram,
};

pub struct CapturedCopies<'a> {
    changed_copies: &'a FxHashSet<BindingId>,
}

struct ReachedClosures {
    addresses: FxHashSet<usize>,
    params: Vec<(BindingId, Type)>,
}

impl<'a> CapturedCopies<'a> {
    pub fn new(changed_copies: &'a FxHashSet<BindingId>) -> Self {
        Self { changed_copies }
    }

    pub fn apply(&self, program: &mut TypedProgram) {
        if self.changed_copies.is_empty() {
            return;
        }
        for position in Self::copying_definitions(program) {
            let reached = self.reached_closures(program, position);
            let (module, definition) = position;
            let TypedExprKind::Fn(lambda) =
                &mut program.modules[module].definitions[definition].value.kind
            else {
                unreachable!("a copying definition is a function")
            };
            for body in Rc::make_mut(lambda).body.expressions_mut() {
                Self::copy_into_reached(body, &reached, &mut program.bindings);
            }
        }
    }

    fn copying_definitions(program: &mut TypedProgram) -> Vec<(usize, usize)> {
        let pipelines = Pipelines::new(&program.rust_types);
        let mut positions = Vec::new();
        for (module_position, module) in program.modules.iter_mut().enumerate() {
            for (position, definition) in module.definitions.iter_mut().enumerate() {
                if let TypedExprKind::Fn(lambda) = &mut definition.value.kind
                    && !Self::takes_opaque_parameter(lambda, pipelines)
                {
                    Rc::make_mut(lambda);
                    positions.push((module_position, position));
                }
            }
        }
        positions
    }

    fn reached_closures(
        &self,
        program: &TypedProgram,
        (module, definition): (usize, usize),
    ) -> ReachedClosures {
        let TypedExprKind::Fn(lambda) = &program.modules[module].definitions[definition].value.kind
        else {
            unreachable!("a copying definition is a function")
        };
        ReachedClosures {
            addresses: ReturnedValues::new(program)
                .closures(lambda)
                .into_iter()
                .map(Self::address)
                .collect(),
            params: lambda
                .params
                .iter()
                .filter(|param| self.changed_copies.contains(&param.binding))
                .map(|param| (param.binding, param.ty.referent().clone()))
                .collect(),
        }
    }

    fn copy_into_reached(
        expr: &mut TypedExpr,
        reached: &ReachedClosures,
        bindings: &mut BindingArena,
    ) {
        if reached.addresses.contains(&Self::address(expr)) {
            Self::capture_copies(expr, &reached.params, bindings);
            return;
        }
        for child in expr.children_mut() {
            Self::copy_into_reached(child, reached, bindings);
        }
    }

    fn capture_copies(
        closure: &mut TypedExpr,
        params: &[(BindingId, Type)],
        bindings: &mut BindingArena,
    ) {
        let mut items = Vec::new();
        for (param, value_type) in params {
            if closure.mentions_local(*param) && Self::only_copies(closure, *param) {
                let (binder, read) = Self::copy_of(*param, value_type, bindings);
                Self::read_copy(closure, *param, binder.binding);
                items.push(TypedBodyItem::Let(binder, read));
            }
        }
        if items.is_empty() {
            return;
        }
        if let TypedExprKind::Fn(lambda) = &mut closure.kind {
            Rc::make_mut(lambda).moves_captures = true;
        }
        items.push(TypedBodyItem::Expr(closure.clone()));
        closure.kind = TypedExprKind::Control(TypedControl::Block(TypedBlock { items }));
    }

    fn copy_of(
        param: BindingId,
        value_type: &Type,
        bindings: &mut BindingArena,
    ) -> (TypedBinder, TypedExpr) {
        let info = bindings.get(param);
        let span = info.span().expect("a parameter has a span");
        let copy = bindings.add(BindingInfo::new(
            info.name().into(),
            BindingKind::Local,
            Some(span),
        ));
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
            ty: value_type.clone(),
            span,
            annotated: false,
        };
        (binder, read)
    }

    fn address(expr: &TypedExpr) -> usize {
        std::ptr::from_ref(expr) as usize
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
