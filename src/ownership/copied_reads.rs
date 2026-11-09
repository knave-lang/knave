use rustc_hash::{FxHashMap, FxHashSet};

use super::functions::{FunctionTable, ParamMode};
use super::value_kinds::ValueKind;
use crate::core::BindingId;
use crate::types::{
    Borrow, CopyRules, Type, TypedArgument, TypedControl, TypedExpr, TypedExprKind, TypedLeaf,
    TypedProgram, TypedStructExpr,
};

pub struct CopiedMutReads {
    changed_copies: FxHashSet<BindingId>,
}

struct CopyOut<'a> {
    changed_copies: &'a FxHashSet<BindingId>,
    function_params: &'a FxHashMap<BindingId, Vec<Type>>,
}

impl CopiedMutReads {
    pub fn new(
        functions: &FunctionTable,
        modes: &FxHashMap<BindingId, ParamMode>,
        copy: &CopyRules,
    ) -> Self {
        let changed_copies = functions
            .definitions()
            .flat_map(|function| &function.params)
            .filter(|slot| {
                slot.inferable
                    && modes.get(&slot.binding) == Some(&ParamMode::Mut)
                    && ValueKind::of(slot, copy) != ValueKind::Moving
            })
            .map(|slot| slot.binding)
            .collect();
        Self { changed_copies }
    }

    pub fn apply(
        &self,
        program: &mut TypedProgram,
        function_params: &FxHashMap<BindingId, Vec<Type>>,
    ) {
        if self.changed_copies.is_empty() {
            return;
        }
        let copy_out = CopyOut {
            changed_copies: &self.changed_copies,
            function_params,
        };
        for definition in program
            .modules
            .iter_mut()
            .flat_map(|module| module.definitions.iter_mut())
        {
            copy_out.visit(&mut definition.value);
        }
    }
}

impl CopyOut<'_> {
    fn visit(&self, expr: &mut TypedExpr) {
        match &mut expr.kind {
            TypedExprKind::Call(callee, arguments) => self.call(callee, arguments),
            TypedExprKind::Recur(arguments) => self.recur(arguments),
            TypedExprKind::Borrow(_, value)
            | TypedExprKind::Deref(value)
            | TypedExprKind::Struct(TypedStructExpr::Field(value, _)) => self.keep(value),
            TypedExprKind::Control(TypedControl::Match(matched)) => {
                self.keep(&mut matched.value);
                matched
                    .arms
                    .iter_mut()
                    .flat_map(|arm| arm.body.expressions_mut())
                    .for_each(|body| self.read(body));
            }
            _ => expr
                .children_mut()
                .into_iter()
                .for_each(|child| self.read(child)),
        }
    }

    fn call(&self, callee: &mut TypedExpr, arguments: &mut [TypedArgument]) {
        let params = self.callee_params(callee);
        self.visit(callee);
        for (position, argument) in arguments.iter_mut().enumerate() {
            match params.get(position) {
                Some(param) if param.is_reference() => self.keep(&mut argument.value),
                _ => self.read(&mut argument.value),
            }
        }
    }

    fn callee_params(&self, callee: &TypedExpr) -> Vec<Type> {
        let declared = match callee.kind {
            TypedExprKind::Leaf(TypedLeaf::Global(function)) => self.function_params.get(&function),
            _ => None,
        };
        match (declared, &callee.ty) {
            (Some(params), _) => params.clone(),
            (None, Type::Function(_, params, _)) => params.clone(),
            _ => Vec::new(),
        }
    }

    fn recur(&self, arguments: &mut [TypedArgument]) {
        for argument in arguments {
            if argument.borrow == Borrow::None {
                self.read(&mut argument.value);
            } else {
                self.keep(&mut argument.value);
            }
        }
    }

    fn keep(&self, expr: &mut TypedExpr) {
        if !self.is_changed_copy(expr) {
            self.visit(expr);
        }
    }

    fn read(&self, expr: &mut TypedExpr) {
        if self.is_changed_copy(expr) {
            Self::copy_out(expr);
        } else {
            self.visit(expr);
        }
    }

    fn is_changed_copy(&self, expr: &TypedExpr) -> bool {
        matches!(
            (&expr.kind, &expr.ty),
            (TypedExprKind::Leaf(TypedLeaf::Local(binding)), Type::Mut(_, _))
                if self.changed_copies.contains(binding)
        )
    }

    fn copy_out(expr: &mut TypedExpr) {
        let Type::Mut(_, referent) = &expr.ty else {
            unreachable!("a changed copy is a mutable borrow")
        };
        let ty = referent.as_ref().clone();
        *expr = TypedExpr {
            kind: TypedExprKind::Deref(Box::new(expr.clone())),
            ty,
            span: expr.span,
        };
    }
}
