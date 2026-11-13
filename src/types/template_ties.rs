use std::collections::BTreeSet;

use super::ty::{GenericId, Lifetime, Scheme, Type};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateTies {
    lifetime: Vec<usize>,
    held: Vec<usize>,
}

impl TemplateTies {
    pub fn new(lifetime: &[usize], held: &[usize]) -> Self {
        Self {
            lifetime: lifetime.to_vec(),
            held: held.to_vec(),
        }
    }

    pub fn of(scheme: &Scheme) -> Option<Self> {
        let Type::Function(_, parameters, result) = &scheme.body else {
            return Some(Self::default());
        };
        let generics: Vec<GenericId> = scheme.params.iter().map(|param| param.id).collect();
        Some(Self {
            lifetime: Self::lifetime_ties(parameters, result)?,
            held: Self::held_ties(parameters, result, &generics),
        })
    }

    pub fn lends_through(&self, position: usize) -> bool {
        self.lifetime.contains(&position)
    }

    pub fn holds_through(&self, position: usize) -> bool {
        self.held.contains(&position)
    }

    fn held_ties(parameters: &[Type], result: &Type, generics: &[GenericId]) -> Vec<usize> {
        let shared: Vec<GenericId> = generics
            .iter()
            .copied()
            .filter(|generic| result.mentions(*generic))
            .collect();
        parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| shared.iter().any(|generic| parameter.mentions(*generic)))
            .map(|(position, _)| position)
            .collect()
    }

    fn lifetime_ties(parameters: &[Type], result: &Type) -> Option<Vec<usize>> {
        let mut tied = BTreeSet::new();
        for output in Self::lifetimes_in(result) {
            tied.extend(Self::sources_of(output, parameters)?);
        }
        Some(tied.into_iter().collect())
    }

    fn sources_of(output: Lifetime, parameters: &[Type]) -> Option<Vec<usize>> {
        let inputs: Vec<(usize, Lifetime)> = parameters
            .iter()
            .enumerate()
            .flat_map(|(position, parameter)| {
                Self::lifetimes_in(parameter)
                    .into_iter()
                    .map(move |lifetime| (position, lifetime))
            })
            .collect();
        match output {
            Lifetime::Written(_) => Some(
                inputs
                    .iter()
                    .filter(|(_, input)| *input == output)
                    .map(|(position, _)| *position)
                    .collect(),
            ),
            Lifetime::Elided => match inputs.as_slice() {
                [(position, _)] => Some(vec![*position]),
                _ => None,
            },
        }
    }

    fn lifetimes_in(ty: &Type) -> Vec<Lifetime> {
        let own = match ty {
            Type::Function(..) => return Vec::new(),
            Type::Ref(lifetime, _) | Type::Mut(lifetime, _) => Some(*lifetime),
            _ => None,
        };
        own.into_iter()
            .chain(ty.parts().into_iter().flat_map(Self::lifetimes_in))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::TemplateTies;
    use crate::core::{BindingId, ClosureKind, PrimitiveType};
    use crate::types::ty::{GenericId, GenericParam, Lifetime, Scheme, Type};

    struct TiesFixture;

    impl TiesFixture {
        fn generic() -> Type {
            Type::Generic(GenericId::new(0))
        }

        fn written(inner: Type) -> Type {
            Type::Ref(Lifetime::written(BindingId::from_index(7)), Box::new(inner))
        }

        fn other_generic() -> Type {
            Type::Generic(GenericId::new(1))
        }

        fn param(index: usize, name: &str) -> GenericParam {
            GenericParam {
                id: GenericId::new(index),
                name: Rc::from(name),
                bounds: Vec::new(),
            }
        }

        fn ties(parameters: Vec<Type>, result: Type) -> Option<TemplateTies> {
            TemplateTies::of(&Scheme {
                lifetimes: Vec::new(),
                params: vec![Self::param(0, "a"), Self::param(1, "b")],
                body: Type::Function(ClosureKind::Fn, parameters, Box::new(result)),
            })
        }
    }

    #[test]
    fn an_elided_result_borrows_the_one_parameter_that_holds_a_reference() {
        let index = Type::Primitive(PrimitiveType::Usize);
        let at = TiesFixture::ties(
            vec![Type::shared(TiesFixture::generic()), index],
            Type::shared(TiesFixture::generic()),
        );
        assert_eq!(at, Some(TemplateTies::new(&[0], &[0])));
    }

    #[test]
    fn an_elided_result_with_two_borrowed_parameters_is_not_settled() {
        let text = Type::str_ref();
        assert_eq!(
            TiesFixture::ties(vec![text.clone(), text.clone()], text),
            None
        );
    }

    #[test]
    fn a_written_lifetime_ties_every_parameter_that_carries_it() {
        let text = || TiesFixture::written(Type::Primitive(PrimitiveType::Str));
        let ties = TiesFixture::ties(vec![text(), text(), Type::str_ref()], text());
        assert_eq!(ties, Some(TemplateTies::new(&[0, 1], &[])));
    }

    #[test]
    fn a_type_variable_ties_what_a_parameter_holds_and_not_its_loan() {
        let vector = Type::mutable(TiesFixture::generic());
        let pop = TiesFixture::ties(vec![vector], TiesFixture::generic());
        let either = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::generic()],
            TiesFixture::generic(),
        );
        assert_eq!(pop, Some(TemplateTies::new(&[], &[0])));
        assert_eq!(either, Some(TemplateTies::new(&[], &[0, 1])));
    }

    #[test]
    fn a_parameter_with_only_a_type_variable_the_result_lacks_holds_nothing_for_it() {
        let lookup = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::other_generic()],
            TiesFixture::generic(),
        );
        assert_eq!(lookup, Some(TemplateTies::new(&[], &[0])));
    }

    #[test]
    fn a_function_parameter_brings_no_input_lifetime() {
        let callback = Type::Function(
            ClosureKind::Fn,
            vec![Type::str_ref()],
            Box::new(Type::bool()),
        );
        let ties = TiesFixture::ties(vec![Type::str_ref(), callback], Type::str_ref());
        assert_eq!(ties, Some(TemplateTies::new(&[0], &[])));
    }
}
