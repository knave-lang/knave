use rustc_hash::FxHashMap;

use crate::core::BindingId;
use crate::types::{Borrow, TypedArgument, TypedBinder, TypedExprKind, TypedLambda, TypedLeaf};

pub(super) struct DestructuredWrapper {
    lambda: TypedLambda,
    depths: FxHashMap<BindingId, usize>,
}

impl DestructuredWrapper {
    pub(super) fn of(lambda: &TypedLambda) -> Option<Self> {
        let (_, arguments) = lambda.wrapped_call()?;
        let depths: FxHashMap<BindingId, usize> = lambda
            .params
            .iter()
            .zip(arguments)
            .filter_map(|(param, argument)| Self::copied_depth(param, argument))
            .collect();
        if depths.is_empty() {
            return None;
        }
        let mut rewritten = lambda.clone();
        for argument in rewritten.wrapped_arguments_mut()? {
            Self::pass_bare(argument, &depths);
        }
        Some(Self {
            lambda: rewritten,
            depths,
        })
    }

    pub(super) fn lambda(&self) -> &TypedLambda {
        &self.lambda
    }

    pub(super) fn pattern_prefix(&self, param: &TypedBinder) -> String {
        "&".repeat(self.depths.get(&param.binding).copied().unwrap_or(0))
    }

    fn copied_depth(param: &TypedBinder, argument: &TypedArgument) -> Option<(BindingId, usize)> {
        let Borrow::Deref(levels) = argument.borrow else {
            return None;
        };
        let passes_param = matches!(
            argument.value.kind,
            TypedExprKind::Leaf(TypedLeaf::Local(binding)) if binding == param.binding
        );
        (passes_param && levels == param.ty.reference_depth()).then_some((param.binding, levels))
    }

    fn pass_bare(argument: &mut TypedArgument, depths: &FxHashMap<BindingId, usize>) {
        let TypedExprKind::Leaf(TypedLeaf::Local(binding)) = argument.value.kind else {
            return;
        };
        if depths.contains_key(&binding) {
            argument.borrow = Borrow::None;
            argument.value.ty = argument.value.ty.innermost_referent().clone();
        }
    }
}
