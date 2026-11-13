use super::ty::{Scheme, Type};

pub struct ResultTies {
    parameters: Vec<usize>,
}

impl ResultTies {
    pub fn of(scheme: &Scheme) -> Option<Self> {
        let Type::Function(_, parameters, result) = &scheme.body else {
            return None;
        };
        let shared: Vec<_> = scheme
            .params
            .iter()
            .map(|param| param.id)
            .filter(|generic| result.mentions(*generic))
            .collect();
        let tied: Vec<usize> = parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| shared.iter().any(|generic| parameter.mentions(*generic)))
            .map(|(position, _)| position)
            .collect();
        (!tied.is_empty()).then_some(Self { parameters: tied })
    }

    pub fn parameters(&self) -> &[usize] {
        &self.parameters
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::ResultTies;
    use crate::core::{ClosureKind, PrimitiveType};
    use crate::types::ty::{GenericId, GenericParam, Scheme, Type};

    struct TiesFixture;

    impl TiesFixture {
        fn generic() -> Type {
            Type::Generic(GenericId::new(0))
        }

        fn scheme(parameters: Vec<Type>, result: Type) -> Scheme {
            Scheme {
                lifetimes: Vec::new(),
                params: vec![GenericParam {
                    id: GenericId::new(0),
                    name: Rc::from("a"),
                    bounds: Vec::new(),
                }],
                body: Type::Function(ClosureKind::Fn, parameters, Box::new(result)),
            }
        }

        fn ties(parameters: Vec<Type>, result: Type) -> Option<Vec<usize>> {
            ResultTies::of(&Self::scheme(parameters, result)).map(|ties| ties.parameters().to_vec())
        }
    }

    #[test]
    fn a_result_ties_each_parameter_that_shares_one_of_its_type_variables() {
        let index = Type::Primitive(PrimitiveType::Usize);
        let at = TiesFixture::ties(
            vec![Type::shared(TiesFixture::generic()), index],
            Type::shared(TiesFixture::generic()),
        );
        let either = TiesFixture::ties(
            vec![TiesFixture::generic(), TiesFixture::generic()],
            TiesFixture::generic(),
        );
        assert_eq!(at, Some(vec![0]));
        assert_eq!(either, Some(vec![0, 1]));
    }

    #[test]
    fn a_result_without_a_type_variable_has_no_record() {
        let text = Type::str_ref();
        assert_eq!(TiesFixture::ties(vec![text.clone()], text), None);
    }
}
