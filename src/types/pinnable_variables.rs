use rustc_hash::FxHashSet;

use super::ty::{Type, TypeVar};
use super::type_variables::TypeVariables;
use super::typed::{TypedExpr, TypedExprKind};
use super::unifier::Unifier;
use crate::core::BindingId;

pub(super) struct PinnableVariables<'a> {
    unifier: &'a Unifier,
}

impl<'a> PinnableVariables<'a> {
    pub(super) fn new(unifier: &'a Unifier) -> Self {
        Self { unifier }
    }

    pub(super) fn of(&self, members: &[(BindingId, &TypedExpr)]) -> FxHashSet<TypeVar> {
        let mut pinnable = self.parameter_vars(members);
        pinnable.extend(self.result_vars(members));
        pinnable
    }

    fn parameter_vars(&self, members: &[(BindingId, &TypedExpr)]) -> FxHashSet<TypeVar> {
        Self::parameter_types(members)
            .iter()
            .flat_map(|parameter| self.pinnable_parameter_vars(parameter))
            .collect()
    }

    fn pinnable_parameter_vars(&self, parameter: &Type) -> Vec<TypeVar> {
        match self.unifier.shallow(parameter) {
            Type::Var(var) => vec![var],
            function @ Type::Function(..) => self.variables().free_in(&function),
            _ => Vec::new(),
        }
    }

    fn parameter_types(members: &[(BindingId, &TypedExpr)]) -> Vec<Type> {
        members
            .iter()
            .flat_map(|(_, value)| match &value.kind {
                TypedExprKind::Fn(lambda) => {
                    lambda.params.iter().map(|param| param.ty.clone()).collect()
                }
                _ => Vec::new(),
            })
            .collect()
    }

    fn result_vars(&self, members: &[(BindingId, &TypedExpr)]) -> Vec<TypeVar> {
        members
            .iter()
            .filter_map(|(_, value)| match self.unifier.resolve(&value.ty) {
                Type::Function(_, _, result) => Some(result),
                _ => None,
            })
            .flat_map(|result| self.variables().free_in(&result))
            .collect()
    }

    fn variables(&self) -> TypeVariables<'a> {
        TypeVariables::new(self.unifier)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::generalization_fixture::GeneralizationFixture;
    use crate::types::{ClosureKind, Type};

    #[test]
    fn a_parameter_variable_is_pinnable() {
        let mut fixture = GeneralizationFixture::new();
        let parameter = fixture.var();
        let function = GeneralizationFixture::function(
            vec![Type::Var(parameter)],
            GeneralizationFixture::int(),
        );
        let pinnable = fixture
            .pinnable_variables()
            .of(&[GeneralizationFixture::member(&function)]);
        assert!(pinnable.contains(&parameter));
    }

    #[test]
    fn a_variable_inside_a_function_parameter_is_pinnable() {
        let mut fixture = GeneralizationFixture::new();
        let inner = fixture.var();
        let callback = Type::Function(
            ClosureKind::Fn,
            vec![Type::Var(inner)],
            Box::new(GeneralizationFixture::int()),
        );
        let function =
            GeneralizationFixture::function(vec![callback], GeneralizationFixture::int());
        let pinnable = fixture
            .pinnable_variables()
            .of(&[GeneralizationFixture::member(&function)]);
        assert!(pinnable.contains(&inner));
    }

    #[test]
    fn a_result_variable_is_pinnable() {
        let mut fixture = GeneralizationFixture::new();
        let result = fixture.var();
        let function = GeneralizationFixture::function(Vec::new(), Type::Var(result));
        let pinnable = fixture
            .pinnable_variables()
            .of(&[GeneralizationFixture::member(&function)]);
        assert!(pinnable.contains(&result));
    }
}
