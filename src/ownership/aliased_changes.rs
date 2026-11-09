use rustc_hash::FxHashMap;

use super::problems::OwnershipProblems;
use crate::core::{BindingId, ReferenceKind};
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::types::{
    FieldPlace, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf, TypedProgram,
};

struct ChangedArgument<'e> {
    argument: &'e TypedExpr,
    place: &'e TypedExpr,
}

pub struct AliasedChanges<'a> {
    program: &'a TypedProgram,
    function_params: &'a FxHashMap<BindingId, Vec<Type>>,
}

impl<'a> AliasedChanges<'a> {
    pub fn new(
        program: &'a TypedProgram,
        function_params: &'a FxHashMap<BindingId, Vec<Type>>,
    ) -> Self {
        Self {
            program,
            function_params,
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
            self.visit(&definition.value, &mut diagnostics);
        }
        diagnostics
    }

    fn visit(&self, expr: &TypedExpr, diagnostics: &mut Diagnostics) {
        if let TypedExprKind::Call(callee, arguments) = &expr.kind {
            self.call(callee, arguments, diagnostics);
        }
        for child in expr.children() {
            self.visit(child, diagnostics);
        }
    }

    fn call(&self, callee: &TypedExpr, arguments: &[TypedArgument], diagnostics: &mut Diagnostics) {
        let params = self.callee_params(callee);
        let changed: Vec<ChangedArgument> = arguments
            .iter()
            .enumerate()
            .filter_map(|(position, argument)| {
                Self::changed_place(argument, params.get(position)).map(|place| ChangedArgument {
                    argument: &argument.value,
                    place,
                })
            })
            .collect();
        for (later, second) in changed.iter().enumerate() {
            if let Some(first) = changed[..later]
                .iter()
                .find(|first| Self::overlap(first.place, second.place))
            {
                diagnostics.push(self.aliased_change(first, second));
            }
        }
    }

    fn aliased_change(&self, first: &ChangedArgument, second: &ChangedArgument) -> Diagnostic {
        let root = second
            .place
            .place_root()
            .expect("a changed place has a root");
        OwnershipProblems::aliased_change(
            self.program.bindings.get(root).name(),
            first.argument.span,
            second.argument.span,
        )
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

    fn changed_place<'e>(
        argument: &'e TypedArgument,
        param: Option<&Type>,
    ) -> Option<&'e TypedExpr> {
        let place: &TypedExpr = match (&argument.value.kind, param) {
            (TypedExprKind::Borrow(ReferenceKind::Mutable, place), _) => place,
            (_, Some(Type::Mut(..))) => &argument.value,
            _ => return None,
        };
        place.is_place().then_some(place)
    }

    fn overlap(first: &TypedExpr, second: &TypedExpr) -> bool {
        let (Some(first), Some(second)) = (first.field_place(), second.field_place()) else {
            return false;
        };
        first.root == second.root && Self::nested(&first, &second)
    }

    fn nested(first: &FieldPlace, second: &FieldPlace) -> bool {
        first
            .path
            .iter()
            .zip(&second.path)
            .all(|(left, right)| left.name == right.name)
    }
}
