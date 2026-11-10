use std::rc::Rc;

use crate::core::{BindingArena, BindingId, CoreModule, Expr, ExprKind, ItemKind, Lambda, Leaf};
use crate::source::Span;

pub struct ParameterCall {
    pub span: Span,
    pub arguments: usize,
    pub parameter: Rc<str>,
    pub shadows_function: bool,
}

struct CallSite {
    span: Span,
    arguments: usize,
}

pub(super) struct CalledParameters<'m> {
    modules: &'m [CoreModule],
    bindings: &'m BindingArena,
}

impl<'m> CalledParameters<'m> {
    pub(super) fn new(modules: &'m [CoreModule], bindings: &'m BindingArena) -> Self {
        Self { modules, bindings }
    }

    pub(super) fn first_call_of(
        &self,
        function: BindingId,
        position: usize,
    ) -> Option<ParameterCall> {
        let (module, lambda) = self.definition(function)?;
        let param = lambda.params.get(position)?;
        let site = Self::first_call(lambda.body.expressions(), param.binding)?;
        let parameter: Rc<str> = self.bindings.get(param.binding).name().into();
        Some(ParameterCall {
            span: site.span,
            arguments: site.arguments,
            shadows_function: self.defines_function(module, &parameter),
            parameter,
        })
    }

    fn definition(&self, function: BindingId) -> Option<(&'m CoreModule, &'m Lambda)> {
        self.modules.iter().find_map(|module| {
            Self::functions(module)
                .find(|(binding, _)| *binding == function)
                .map(|(_, lambda)| (module, lambda))
        })
    }

    fn defines_function(&self, module: &CoreModule, name: &str) -> bool {
        Self::functions(module).any(|(function, _)| self.bindings.get(function).name() == name)
    }

    fn functions(module: &CoreModule) -> impl Iterator<Item = (BindingId, &Lambda)> {
        module.items.iter().filter_map(|item| match &item.kind {
            ItemKind::Def(binding, value) => match value.kind() {
                ExprKind::Fn(lambda) => Some((*binding, lambda.as_ref())),
                _ => None,
            },
            _ => None,
        })
    }

    fn first_call<'a>(
        mut expressions: impl Iterator<Item = &'a Expr>,
        parameter: BindingId,
    ) -> Option<CallSite> {
        expressions.find_map(|expr| Self::call_within(expr, parameter))
    }

    fn call_within(expr: &Expr, parameter: BindingId) -> Option<CallSite> {
        match Self::arguments_of_call(expr, parameter) {
            Some(arguments) => Some(CallSite {
                span: expr.span(),
                arguments,
            }),
            None => Self::first_call(expr.children().into_iter(), parameter),
        }
    }

    fn arguments_of_call(expr: &Expr, parameter: BindingId) -> Option<usize> {
        match expr.kind() {
            ExprKind::Call(callee, arguments) if matches!(callee.kind(), ExprKind::Leaf(Leaf::Var(binding)) if *binding == parameter) => {
                Some(arguments.len())
            }
            _ => None,
        }
    }
}
