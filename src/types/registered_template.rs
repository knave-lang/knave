use super::synthetic_template::SyntheticTemplate;
use super::template_ties::TemplateTies;
use crate::core::ParsedTemplate;

pub struct RegisteredTemplate {
    parsed: ParsedTemplate,
    ties: Option<TemplateTies>,
}

impl RegisteredTemplate {
    pub fn declared(text: &str, ties: Option<TemplateTies>) -> Option<Self> {
        let parsed = ParsedTemplate::parse(text).ok()?;
        Some(Self { parsed, ties })
    }

    pub fn synthetic(text: &str) -> Option<Self> {
        Self::declared(
            text,
            SyntheticTemplate::named(text).map(SyntheticTemplate::ties),
        )
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
        let ties = TemplateTies::of(&scheme);
        let declared = RegisteredTemplate::declared("&{0}[0]", ties).expect("the text parses");
        assert_eq!(declared.ties(), Some(&TemplateTies::new(&[0], &[0])));
    }

    #[test]
    fn an_undeclared_template_takes_the_ties_of_its_synthetic_record_or_none() {
        let look_up = SyntheticTemplate::LOOK_UP.text();
        let synthetic = RegisteredTemplate::synthetic(look_up).expect("the text parses");
        let unknown = RegisteredTemplate::synthetic("&{0}[0]").expect("the text parses");
        assert_eq!(synthetic.ties(), Some(&SyntheticTemplate::LOOK_UP.ties()));
        assert_eq!(unknown.ties(), None);
    }
}
