use rustc_hash::FxHashMap;

use super::ty::{GenericId, Type};
use super::typed::TypedDefinition;

pub struct SchemeBindings<'d> {
    definition: &'d TypedDefinition,
}

impl<'d> SchemeBindings<'d> {
    pub fn new(definition: &'d TypedDefinition) -> Self {
        Self { definition }
    }

    pub fn of(&self, instantiated: &Type) -> Vec<(String, Type)> {
        let mut bindings = self.bindings(instantiated);
        self.definition
            .scheme
            .params
            .iter()
            .filter_map(|param| Some((param.name.to_string(), bindings.remove(&param.id)?)))
            .collect()
    }

    pub fn of_parameter(&self, instantiated: &Type, parameter: GenericId) -> Option<Type> {
        self.bindings(instantiated).remove(&parameter)
    }

    fn bindings(&self, instantiated: &Type) -> FxHashMap<GenericId, Type> {
        let mut bindings = FxHashMap::default();
        Self::collect(&self.definition.scheme.body, instantiated, &mut bindings);
        bindings
    }

    fn collect(pattern: &Type, instantiated: &Type, bindings: &mut FxHashMap<GenericId, Type>) {
        match (pattern, instantiated) {
            (Type::Generic(generic), ty) => {
                bindings.entry(*generic).or_insert_with(|| ty.clone());
            }
            (Type::Named(owner, arguments), Type::Named(other, other_arguments))
                if owner == other =>
            {
                for (argument, other_argument) in arguments.iter().zip(other_arguments) {
                    Self::collect(argument, other_argument, bindings);
                }
            }
            (Type::Function(_, parameters, result), Type::Function(_, other, other_result)) => {
                for (parameter, other_parameter) in parameters.iter().zip(other) {
                    Self::collect(parameter, other_parameter, bindings);
                }
                Self::collect(result, other_result, bindings);
            }
            (Type::Ref(_, inner), Type::Ref(_, other))
            | (Type::Mut(_, inner), Type::Mut(_, other)) => {
                Self::collect(inner, other, bindings);
            }
            _ => {}
        }
    }
}
