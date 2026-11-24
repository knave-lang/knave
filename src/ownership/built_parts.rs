use super::text::EnumPayloads;
use crate::core::BindingId;
use crate::types::{
    DefinitionTable, FieldOrder, Type, TypedArgument, TypedExpr, TypedExprKind, TypedLeaf,
    TypedProgram,
};

pub struct BuiltParts<'p> {
    program: &'p TypedProgram,
    definitions: &'p DefinitionTable<'p>,
    payloads: EnumPayloads<'p>,
}

impl<'p> BuiltParts<'p> {
    pub fn new(program: &'p TypedProgram, definitions: &'p DefinitionTable<'p>) -> Self {
        Self {
            program,
            definitions,
            payloads: EnumPayloads::new(program),
        }
    }

    pub fn variant_part<'e>(
        &self,
        value: &'e TypedExpr,
        variant: BindingId,
        position: usize,
    ) -> Option<&'e TypedExpr> {
        let shares_one_lifetime = self
            .payloads
            .enumeration_of(variant)
            .is_some_and(|enumeration| enumeration.lifetimes.len() + enumeration.params.len() <= 1);
        self.built_part(value, variant, position)
            .or(shares_one_lifetime.then_some(value))
    }

    pub fn field_part<'e>(
        &self,
        value: &'e TypedExpr,
        structure: BindingId,
    ) -> Option<&'e TypedExpr> {
        let structs = self.program.declarations.types().structs();
        let shares_one_lifetime = structs
            .get(structure)
            .is_some_and(|definition| definition.lifetimes.len() + definition.params.len() <= 1);
        shares_one_lifetime.then_some(value)
    }

    fn built_part<'e>(
        &self,
        value: &'e TypedExpr,
        variant: BindingId,
        position: usize,
    ) -> Option<&'e TypedExpr> {
        match &value.kind {
            TypedExprKind::Variant(built, payloads, order) if *built == variant => {
                Self::placed_payload(payloads, order, position)
            }
            TypedExprKind::Call(callee, arguments) => {
                let argument = self.built_argument(callee, variant, position)?;
                arguments
                    .get(argument)
                    .map(|argument: &TypedArgument| &argument.value)
            }
            _ => None,
        }
    }

    fn placed_payload<'e>(
        payloads: &'e [TypedExpr],
        order: &FieldOrder,
        position: usize,
    ) -> Option<&'e TypedExpr> {
        payloads
            .iter()
            .enumerate()
            .find_map(|(written, payload)| (order.field_of(written) == position).then_some(payload))
    }

    fn built_argument(
        &self,
        callee: &TypedExpr,
        variant: BindingId,
        position: usize,
    ) -> Option<usize> {
        let TypedExprKind::Leaf(TypedLeaf::Global(function)) = callee.kind else {
            return None;
        };
        let Type::Function(_, parameters, result) =
            &self.definitions.definition(function)?.scheme.body
        else {
            return None;
        };
        let Type::Named(_, type_arguments) = result.as_ref() else {
            return None;
        };
        let built = type_arguments.get(self.payloads.argument(variant, position)?)?;
        Self::only_parameter_of(parameters, built)
    }

    fn only_parameter_of(parameters: &[Type], built: &Type) -> Option<usize> {
        let Type::Generic(generic) = built else {
            return None;
        };
        let mut mentioning = parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| parameter.mentions(*generic));
        let (only, parameter) = mentioning.next()?;
        let is_the_part = parameter == built && mentioning.next().is_none();
        is_the_part.then_some(only)
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltParts;
    use crate::core::BindingId;
    use crate::types::{GenericId, Type};

    struct PartsFixture;

    impl PartsFixture {
        fn generic(index: usize) -> Type {
            Type::Generic(GenericId::new(index))
        }

        fn wrapped(inner: Type) -> Type {
            Type::Named(BindingId::from_index(0), vec![inner])
        }
    }

    #[test]
    fn a_part_comes_from_the_one_parameter_of_its_type() {
        let parameters = [PartsFixture::generic(0), PartsFixture::generic(1)];
        let part = BuiltParts::only_parameter_of(&parameters, &PartsFixture::generic(1));
        assert_eq!(part, Some(1));
    }

    #[test]
    fn a_part_two_parameters_share_comes_from_neither() {
        let parameters = [PartsFixture::generic(0), PartsFixture::generic(0)];
        let part = BuiltParts::only_parameter_of(&parameters, &PartsFixture::generic(0));
        assert_eq!(part, None);
    }

    #[test]
    fn a_part_another_parameter_holds_comes_from_neither() {
        let held = PartsFixture::wrapped(PartsFixture::generic(0));
        let parameters = [PartsFixture::generic(0), held];
        let part = BuiltParts::only_parameter_of(&parameters, &PartsFixture::generic(0));
        assert_eq!(part, None);
    }

    #[test]
    fn a_part_of_a_written_type_comes_from_no_parameter() {
        let parameters = [Type::str_ref()];
        let part = BuiltParts::only_parameter_of(&parameters, &Type::str_ref());
        assert_eq!(part, None);
    }
}
