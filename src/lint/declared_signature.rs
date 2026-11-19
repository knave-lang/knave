use crate::core::{ClosureKind, TypeExpr, TypeExprKind};
use crate::source::Span;

#[derive(Clone)]
pub struct DeclaredSignature {
    written: TypeExpr,
    closure: ClosureKind,
    parameters: Vec<TypeExpr>,
    result: TypeExpr,
    function_span: Span,
}

impl DeclaredSignature {
    pub fn of(written: &TypeExpr) -> Option<Self> {
        let function = written.where_body();
        let TypeExprKind::Function(closure, parameters, result) = function.kind() else {
            return None;
        };
        Some(Self {
            written: written.clone(),
            closure: *closure,
            parameters: parameters.clone(),
            result: result.as_ref().clone(),
            function_span: function.span(),
        })
    }

    pub fn writes_lifetime(&self) -> bool {
        matches!(self.written.kind(),
            TypeExprKind::Where(binders, _) if binders.iter().any(|binder| binder.lifetime))
    }

    pub fn parameters(&self) -> &[TypeExpr] {
        &self.parameters
    }

    pub fn passing_by_value(&self, positions: &[usize]) -> TypeExpr {
        let function = TypeExpr::new(
            TypeExprKind::Function(
                self.closure,
                self.parameters_by_value(positions),
                Box::new(self.result.clone()),
            ),
            self.function_span,
        );
        match self.written.kind() {
            TypeExprKind::Where(binders, _) => TypeExpr::new(
                TypeExprKind::Where(binders.clone(), Box::new(function)),
                self.written.span(),
            ),
            _ => function,
        }
    }

    fn parameters_by_value(&self, positions: &[usize]) -> Vec<TypeExpr> {
        self.parameters
            .iter()
            .enumerate()
            .map(|(position, parameter)| match parameter.kind() {
                TypeExprKind::Reference(_, _, referent) if positions.contains(&position) => {
                    referent.as_ref().clone()
                }
                _ => parameter.clone(),
            })
            .collect()
    }
}
