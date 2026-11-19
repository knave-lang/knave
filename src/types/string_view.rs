use super::checker::TypeChecker;
use super::ty::Type;
use super::typed::{TypedArgument, TypedBlock, TypedBodyItem, TypedExpr, TypedExprKind, TypedLeaf};
use crate::core::BindingId;
use crate::diagnostic::Diagnostic;

impl TypeChecker {
    pub(super) fn view_closure_result(
        &mut self,
        mut body: TypedBlock,
        result: &Type,
        found: &Type,
    ) -> Result<TypedBlock, Diagnostic> {
        if !self.checking_function_body {
            return Ok(body);
        }
        let Some(view) = self.string_views().view_for_closure_result(result, found) else {
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
        let Some(viewed) = (self.string_views()).strings_to_view_beside_views(positions, arguments)
        else {
            return Ok(());
        };
        for position in viewed.positions {
            let argument = &mut arguments[position];
            argument.value = self.call_string_view(viewed.view, argument.value.clone())?;
        }
        Ok(())
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
