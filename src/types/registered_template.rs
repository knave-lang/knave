use std::rc::Rc;

use super::synthetic_template::SyntheticTemplate;
use super::template_ties::TemplateTies;
use super::ty::Scheme;
use crate::core::ParsedTemplate;

pub struct RegisteredTemplate {
    text: Rc<str>,
    parsed: ParsedTemplate,
    scheme: Option<Scheme>,
}

impl RegisteredTemplate {
    pub fn parse(text: Rc<str>, scheme: Option<Scheme>) -> Option<Self> {
        let parsed = ParsedTemplate::parse(&text).ok()?;
        Some(Self {
            text,
            parsed,
            scheme,
        })
    }

    pub fn parsed(&self) -> &ParsedTemplate {
        &self.parsed
    }

    pub fn ties(&self) -> Option<TemplateTies> {
        match &self.scheme {
            Some(scheme) => TemplateTies::of(scheme),
            None => SyntheticTemplate::named(&self.text).map(SyntheticTemplate::ties),
        }
    }
}
