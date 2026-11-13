use super::synthetic_template::SyntheticTemplate;
use super::template_ties::TemplateTies;
use super::ty::Scheme;
use crate::core::ParsedTemplate;

pub struct RegisteredTemplate {
    parsed: ParsedTemplate,
    ties: Option<TemplateTies>,
}

impl RegisteredTemplate {
    pub fn parse(text: &str, scheme: Option<&Scheme>) -> Option<Self> {
        let parsed = ParsedTemplate::parse(text).ok()?;
        let ties = match scheme {
            Some(scheme) => TemplateTies::of(scheme),
            None => SyntheticTemplate::named(text).map(SyntheticTemplate::ties),
        };
        Some(Self { parsed, ties })
    }

    pub fn parsed(&self) -> &ParsedTemplate {
        &self.parsed
    }

    pub fn ties(&self) -> Option<&TemplateTies> {
        self.ties.as_ref()
    }
}
#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::RegisteredTemplate;
    use crate::core::ClosureKind;
    use crate::types::SyntheticTemplate;
    use crate::types::template_ties::TemplateTies;
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
    fn a_declared_template_keeps_the_ties_of_its_scheme() {
        let scheme = RegisteredFixture::at_scheme();
        let declared =
            RegisteredTemplate::parse("&{0}[0]", Some(&scheme)).expect("the text parses");
        assert_eq!(declared.ties(), Some(&TemplateTies::new(&[0], &[0])));
    }

    #[test]
    fn an_inline_template_takes_the_ties_of_its_synthetic_record_or_none() {
        let look_up = SyntheticTemplate::LOOK_UP.text();
        let synthetic = RegisteredTemplate::parse(look_up, None).expect("the text parses");
        let unknown = RegisteredTemplate::parse("&{0}[0]", None).expect("the text parses");
        assert_eq!(synthetic.ties(), Some(&SyntheticTemplate::LOOK_UP.ties()));
        assert_eq!(unknown.ties(), None);
    }
}
