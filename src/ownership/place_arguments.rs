use rustc_hash::FxHashMap;

use super::functions::{FunctionInfo, FunctionTable};
use super::problems::{ChangeSite, OwnershipProblems};
use super::tail_calls::TailCalls;
use super::whole_value_changes::WholeValueChanges;
use crate::core::BindingId;
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::types::{
    CopyRules, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram,
};

pub struct PlaceArguments<'a> {
    program: &'a TypedProgram,
    functions: &'a FunctionTable,
    function_params: &'a FxHashMap<BindingId, Vec<Type>>,
    copy: &'a CopyRules<'a>,
}

impl<'a> PlaceArguments<'a> {
    pub fn new(
        program: &'a TypedProgram,
        functions: &'a FunctionTable,
        function_params: &'a FxHashMap<BindingId, Vec<Type>>,
        copy: &'a CopyRules<'a>,
    ) -> Self {
        Self {
            program,
            functions,
            function_params,
            copy,
        }
    }

    pub fn check(&self) -> Diagnostics {
        let mut diagnostics = Diagnostics::default();
        for definition in self
            .program
            .modules
            .iter()
            .flat_map(|module| &module.definitions)
        {
            let tail_calls = match &definition.value.kind {
                TypedExprKind::Fn(lambda) => TailCalls::of(Some(definition.binding), lambda)
                    .arguments()
                    .to_vec(),
                _ => Vec::new(),
            };
            self.visit(&definition.value, &tail_calls, &mut diagnostics);
        }
        diagnostics
    }

    fn visit(
        &self,
        expr: &TypedExpr,
        tail_calls: &[&[TypedArgument]],
        diagnostics: &mut Diagnostics,
    ) {
        if let TypedExprKind::Call(callee, arguments) = &expr.kind
            && !Self::is_tail_call(arguments, tail_calls)
        {
            self.call(callee, arguments, diagnostics);
        }
        for child in expr.children() {
            self.visit(child, tail_calls, diagnostics);
        }
    }

    fn is_tail_call(arguments: &[TypedArgument], tail_calls: &[&[TypedArgument]]) -> bool {
        tail_calls
            .iter()
            .any(|tail_call| std::ptr::eq(tail_call.as_ptr(), arguments.as_ptr()))
    }

    fn call(&self, callee: &TypedExpr, arguments: &[TypedArgument], diagnostics: &mut Diagnostics) {
        let TypedExprKind::Leaf(TypedLeaf::Global(function)) = callee.kind else {
            return;
        };
        let (Some(info), Some(params)) = (
            self.functions.definition(function),
            self.function_params.get(&function),
        ) else {
            return;
        };
        for ((slot, param), argument) in info.params.iter().zip(params).zip(arguments) {
            if slot.inferable
                && matches!(param, Type::Mut(..))
                && !Self::can_change(&argument.value)
            {
                let change = self.change_site(function, info, slot.binding);
                diagnostics.push(self.copy_argument(&argument.value, change));
            }
        }
    }

    fn can_change(value: &TypedExpr) -> bool {
        value.is_place() || matches!(value.kind, TypedExprKind::Borrow(..))
    }

    fn change_site(
        &self,
        function: BindingId,
        info: &FunctionInfo,
        param: BindingId,
    ) -> Option<ChangeSite> {
        WholeValueChanges::site(&info.lambda, param).map(|span| ChangeSite {
            function: self.program.bindings.get(function).name().into(),
            span,
        })
    }

    fn copy_argument(&self, value: &TypedExpr, change: Option<ChangeSite>) -> Diagnostic {
        let TypedExprKind::Leaf(TypedLeaf::Global(definition)) = value.kind else {
            return OwnershipProblems::inferred_mut_of_copy(value.span, change);
        };
        let name = self.program.bindings.get(definition).name();
        if self.copy.is_copy(&value.ty) {
            OwnershipProblems::inferred_mut_of_copy_definition(value.span, change, name)
        } else {
            OwnershipProblems::inferred_mut_of_cloned_definition(value.span, change, name)
        }
    }
}
