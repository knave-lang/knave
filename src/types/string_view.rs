use super::checker::TypeChecker;
use super::ty::Type;
use super::typed::{TypedArgument, TypedBlock, TypedBodyItem, TypedExpr, TypedExprKind, TypedLeaf};
use super::view_coercion::ViewCoercion;
use crate::core::{BindingId, PrimitiveType, ReferenceKind};
use crate::diagnostic::Diagnostic;

impl TypeChecker {
    pub(super) fn view_closure_result(
        &mut self,
        mut body: TypedBlock,
        result: &Type,
        found: &Type,
    ) -> Result<TypedBlock, Diagnostic> {
        if !self.sits_in_function_body() || !self.needs_string_view(result, found) {
            return Ok(body);
        }
        let Some(view) = self.string_view else {
            return Ok(body);
        };
        if let Some(TypedBodyItem::Expr(last)) = body.items.last_mut() {
            *last = self.call_string_view(view, last.clone())?;
        }
        Ok(body)
    }

    pub(super) fn view_borrowed_strings_beside_views(
        &mut self,
        positions: impl Iterator<Item = usize> + Clone,
        arguments: &mut [TypedArgument],
    ) -> Result<(), Diagnostic> {
        let Some(view) = self.string_view else {
            return Ok(());
        };
        if !positions
            .clone()
            .any(|position| self.is_string_view(&arguments[position].value.ty))
        {
            return Ok(());
        }
        for position in positions {
            let argument = &mut arguments[position];
            if self.borrows_string_as_written(&argument.value) {
                argument.value = self.call_string_view(view, argument.value.clone())?;
            }
        }
        Ok(())
    }

    fn is_string_view(&self, value: &Type) -> bool {
        match self.unifier.resolve(value) {
            Type::Ref(_, view) => ViewCoercion::new(&view, &Self::owned_string()).views_string(),
            _ => false,
        }
    }

    fn borrows_string_as_written(&mut self, value: &TypedExpr) -> bool {
        let TypedExprKind::Borrow(ReferenceKind::Shared, place) = &value.kind else {
            return false;
        };
        let borrowed = self.unification().place_type(place);
        self.unifier.resolve(&borrowed) == Self::owned_string()
    }

    fn owned_string() -> Type {
        Type::Primitive(PrimitiveType::String)
    }

    fn sits_in_function_body(&self) -> bool {
        self.checking_function_body
    }

    fn needs_string_view(&self, result: &Type, found: &Type) -> bool {
        match (self.unifier.resolve(result), self.unifier.resolve(found)) {
            (Type::Ref(_, view), Type::Ref(_, owned)) => {
                ViewCoercion::new(&view, &owned).views_string()
            }
            _ => false,
        }
    }

    fn call_string_view(
        &mut self,
        view: BindingId,
        value: TypedExpr,
    ) -> Result<TypedExpr, Diagnostic> {
        let span = value.span;
        let ty = self.global_type(view, span);
        let Type::Function(_, parameters, result) = &ty else {
            unreachable!("the string view is a function")
        };
        let (parameter, result) = (parameters[0].clone(), result.as_ref().clone());
        let callee = TypedExpr {
            kind: TypedExprKind::Leaf(TypedLeaf::Global(view)),
            ty,
            span,
        };
        let borrow = self.pass_call_argument(Some(&callee), 0, &parameter, &value)?;
        Ok(TypedExpr {
            kind: TypedExprKind::Call(Box::new(callee), vec![TypedArgument { value, borrow }]),
            ty: result,
            span,
        })
    }
}
