use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::types::{ReturnedValues, TypedExpr, TypedExprKind, TypedLambda, TypedProgram};

pub struct ReturnedLambdas {
    lambdas: FxHashSet<*const TypedLambda>,
}

impl ReturnedLambdas {
    pub fn mark(program: &mut TypedProgram) {
        let returned = Self::of(program);
        if returned.lambdas.is_empty() {
            return;
        }
        for definition in program
            .modules
            .iter_mut()
            .flat_map(|module| &mut module.definitions)
        {
            returned.mark_expr(&mut definition.value);
        }
    }

    fn of(program: &TypedProgram) -> Self {
        let values = ReturnedValues::new(program);
        let lambdas = program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
            .filter_map(|definition| match &definition.value.kind {
                TypedExprKind::Fn(function) => Some(function),
                _ => None,
            })
            .flat_map(|function| values.closures(function))
            .filter_map(|closure| match &closure.kind {
                TypedExprKind::Fn(lambda) => Some(Rc::as_ptr(lambda)),
                _ => None,
            })
            .collect();
        Self { lambdas }
    }

    fn mark_expr(&self, expr: &mut TypedExpr) {
        if let TypedExprKind::Fn(lambda) = &mut expr.kind
            && self.lambdas.contains(&Rc::as_ptr(lambda))
        {
            Rc::make_mut(lambda).facts.returned = true;
        }
        for child in expr.children_mut() {
            self.mark_expr(child);
        }
    }
}
