use super::result_ties::ResultTies;
use super::ty::Scheme;
use crate::core::ParsedTemplate;

pub struct RegisteredTemplate {
    parsed: ParsedTemplate,
    ties: Option<ResultTies>,
}

impl RegisteredTemplate {
    pub fn parse(template: &str, scheme: Option<&Scheme>) -> Option<Self> {
        let parsed = ParsedTemplate::parse(template).ok()?;
        Some(Self {
            parsed,
            ties: scheme.and_then(ResultTies::of),
        })
    }

    pub fn parsed(&self) -> &ParsedTemplate {
        &self.parsed
    }

    pub fn tied_parameters(&self) -> Option<&[usize]> {
        self.ties.as_ref().map(ResultTies::parameters)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::RegisteredTemplate;
    use crate::core::ClosureKind;
    use crate::types::ty::{GenericId, GenericParam, Scheme, Type};

    struct RegisteredFixture;

    impl RegisteredFixture {
        fn at_scheme() -> Scheme {
            let element = Type::Generic(GenericId::new(0));
            Scheme {
                lifetimes: Vec::new(),
                params: vec![GenericParam {
                    id: GenericId::new(0),
                    name: Rc::from("a"),
                    bounds: Vec::new(),
                }],
                body: Type::Function(
                    ClosureKind::Fn,
                    vec![Type::shared(element.clone())],
                    Box::new(Type::shared(element)),
                ),
            }
        }
    }

    #[test]
    fn a_template_with_a_scheme_keeps_its_result_ties_and_an_inline_one_has_none() {
        let scheme = RegisteredFixture::at_scheme();
        let declared =
            RegisteredTemplate::parse("&{0}[0]", Some(&scheme)).expect("the text parses");
        let inline = RegisteredTemplate::parse("&{0}[0]", None).expect("the text parses");
        assert_eq!(declared.tied_parameters(), Some(&[0][..]));
        assert_eq!(inline.tied_parameters(), None);
        assert!(declared.parsed().borrows(0) == inline.parsed().borrows(0));
    }
}
