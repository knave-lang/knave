use rustc_hash::FxHashMap;

use super::function_value::FunctionValue;
use crate::core::BindingId;
use crate::types::{ClosureKind, GenericBinding, TemplateBorrows, Type, TypedExpr, TypedExprKind};

pub(super) struct FunctionValues<'a> {
    function_params: &'a FxHashMap<BindingId, Vec<Type>>,
    templates: &'a TemplateBorrows,
}

impl<'a> FunctionValues<'a> {
    pub(super) fn new(
        function_params: &'a FxHashMap<BindingId, Vec<Type>>,
        templates: &'a TemplateBorrows,
    ) -> Self {
        Self {
            function_params,
            templates,
        }
    }

    pub(super) fn recognize(&self, value: &TypedExpr) -> Option<FunctionValue> {
        let (binding, Type::Function(_, used, result)) = value.function_value()? else {
            return None;
        };
        if self.is_template(value) {
            let ty = Type::Function(ClosureKind::Fn, used.clone(), result.clone());
            return Some(FunctionValue::Template { binding, ty });
        }
        let parameters = self.used_params(binding, used)?;
        let ty = Type::Function(ClosureKind::Fn, parameters, result.clone());
        Some(FunctionValue::Named { binding, ty })
    }

    fn is_template(&self, value: &TypedExpr) -> bool {
        let callee = match &value.kind {
            TypedExprKind::Fn(lambda) => lambda.wrapped_call().map(|(callee, _)| callee),
            _ => Some(value),
        };
        callee.is_some_and(|callee| self.templates.is_template(callee))
    }

    pub(super) fn declared_params(&self, binding: BindingId) -> Option<&[Type]> {
        self.function_params.get(&binding).map(Vec::as_slice)
    }

    pub(super) fn used_params(&self, binding: BindingId, used: &[Type]) -> Option<Vec<Type>> {
        let declared = self.function_params.get(&binding)?;
        Some(GenericBinding::of_each(declared, used).substitute(declared))
    }
}
