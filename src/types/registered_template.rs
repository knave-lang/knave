use std::cell::OnceCell;
use std::rc::Rc;

use super::synthetic_template::SyntheticTemplate;
use super::template_ties::TemplateTies;
use crate::core::ParsedTemplate;

pub struct RegisteredTemplate {
    text: Rc<str>,
    ties: Option<TemplateTies>,
    parsed: OnceCell<Option<ParsedTemplate>>,
}

impl RegisteredTemplate {
    pub fn declared(text: &Rc<str>, ties: Option<TemplateTies>) -> Self {
        Self {
            text: Rc::clone(text),
            ties,
            parsed: OnceCell::new(),
        }
    }

    pub fn synthetic(text: &Rc<str>) -> Self {
        Self::declared(
            text,
            SyntheticTemplate::named(text).map(SyntheticTemplate::ties),
        )
    }

    pub fn widen(&mut self, ties: Option<TemplateTies>) {
        self.ties = TemplateTies::widened(self.ties.take(), ties);
    }

    pub fn read<T>(
        &self,
        absent: T,
        read: impl FnOnce(&ParsedTemplate, Option<&TemplateTies>) -> T,
    ) -> T {
        match self.parsed() {
            Some(parsed) => read(parsed, self.ties.as_ref()),
            None => absent,
        }
    }

    #[cfg(test)]
    pub fn ties(&self) -> Option<&TemplateTies> {
        self.ties.as_ref()
    }

    fn parsed(&self) -> Option<&ParsedTemplate> {
        self.parsed
            .get_or_init(|| ParsedTemplate::parse(&self.text).ok())
            .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::RegisteredTemplate;
    use crate::types::SyntheticTemplate;
    use crate::types::template_ties::TemplateTies;

    struct RegisteredFixture;

    impl RegisteredFixture {
        fn ties_read(template: &RegisteredTemplate) -> Option<Option<TemplateTies>> {
            template.read(None, |_, ties| Some(ties.cloned()))
        }
    }

    #[test]
    fn a_compiler_template_takes_the_ties_of_its_synthetic_record_or_none() {
        let look_up = RegisteredTemplate::synthetic(&Rc::from(SyntheticTemplate::LOOK_UP.text()));
        let unknown = RegisteredTemplate::synthetic(&Rc::from("&{0}[0]"));
        assert_eq!(
            RegisteredFixture::ties_read(&look_up),
            Some(Some(SyntheticTemplate::LOOK_UP.ties()))
        );
        assert_eq!(RegisteredFixture::ties_read(&unknown), Some(None));
    }

    #[test]
    fn a_template_that_does_not_parse_reads_as_absent() {
        let broken = RegisteredTemplate::declared(&Rc::from("{0"), Some(TemplateTies::default()));
        assert_eq!(RegisteredFixture::ties_read(&broken), None);
    }

    #[test]
    fn widening_a_template_keeps_the_ties_of_both_types() {
        let mut template =
            RegisteredTemplate::declared(&Rc::from("{0}"), Some(TemplateTies::new(&[0], &[])));
        template.widen(Some(TemplateTies::new(&[], &[0])));
        assert_eq!(template.ties(), Some(&TemplateTies::new(&[0], &[0])));
    }
}
