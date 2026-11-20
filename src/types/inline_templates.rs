use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use super::registered_template::RegisteredTemplate;
use super::synthetic_template::SyntheticTemplate;
use super::template_ties::TemplateTies;
use crate::core::RustTemplate;

#[derive(Default)]
pub struct InlineTemplates {
    written: FxHashMap<Rc<str>, RegisteredTemplate>,
    compiler: RefCell<FxHashMap<Rc<str>, RegisteredTemplate>>,
}

impl InlineTemplates {
    pub fn register(&mut self, text: &Rc<str>, ties: Option<TemplateTies>) {
        match self.written.get_mut(text) {
            Some(known) => known.widen(ties),
            None => {
                let template = RegisteredTemplate::declared(text, ties);
                self.written.insert(Rc::clone(text), template);
            }
        }
    }

    pub fn read<T>(
        &self,
        template: &RustTemplate,
        read: impl FnOnce(&RegisteredTemplate) -> T,
    ) -> T {
        match self.written.get(&template.template) {
            Some(written) if !Self::is_compiler(template) => read(written),
            _ => self.read_compiler(&template.template, read),
        }
    }

    #[cfg(test)]
    pub fn written(&self) -> impl Iterator<Item = (&Rc<str>, &RegisteredTemplate)> {
        self.written.iter()
    }

    fn read_compiler<T>(&self, text: &Rc<str>, read: impl FnOnce(&RegisteredTemplate) -> T) -> T {
        let mut compiler = self.compiler.borrow_mut();
        let template = compiler
            .entry(Rc::clone(text))
            .or_insert_with(|| RegisteredTemplate::synthetic(text));
        read(template)
    }

    fn is_compiler(template: &RustTemplate) -> bool {
        !template.has_written_type() && SyntheticTemplate::named(&template.template).is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::InlineTemplates;
    use crate::core::{PrimitiveType, RustTemplate, TypeExpr, TypeExprKind};
    use crate::source::{SourceId, Span};
    use crate::types::SyntheticTemplate;
    use crate::types::template_ties::TemplateTies;

    struct InlineFixture;

    impl InlineFixture {
        fn template(text: &str, kind: TypeExprKind) -> RustTemplate {
            RustTemplate {
                template: Rc::from(text),
                type_expr: TypeExpr::new(kind, Span::new(SourceId::new(0), 0..0)),
                clauses: Vec::new(),
            }
        }

        fn written(text: &str) -> RustTemplate {
            Self::template(text, TypeExprKind::Primitive(PrimitiveType::I64))
        }

        fn ties(templates: &InlineTemplates, template: &RustTemplate) -> Option<TemplateTies> {
            templates.read(template, |registered| {
                registered.read(None, |_, ties| ties.cloned())
            })
        }
    }

    #[test]
    fn a_template_read_before_its_second_type_keeps_the_ties_of_both() {
        let mut templates = InlineTemplates::default();
        let written = InlineFixture::written("{0}");
        templates.register(&written.template, Some(TemplateTies::new(&[], &[0])));
        assert_eq!(
            InlineFixture::ties(&templates, &written),
            Some(TemplateTies::new(&[], &[0]))
        );
        templates.register(&written.template, Some(TemplateTies::new(&[0], &[])));
        assert_eq!(
            InlineFixture::ties(&templates, &written),
            Some(TemplateTies::new(&[0], &[0]))
        );
    }

    #[test]
    fn a_compiler_call_keeps_its_record_beside_a_written_template_of_the_same_text() {
        let text = SyntheticTemplate::LOOK_UP.text();
        let mut templates = InlineTemplates::default();
        let written = InlineFixture::written(text);
        templates.register(&written.template, None);
        let compiler = InlineFixture::template(text, TypeExprKind::Hole);
        assert_eq!(InlineFixture::ties(&templates, &written), None);
        assert_eq!(
            InlineFixture::ties(&templates, &compiler),
            Some(SyntheticTemplate::LOOK_UP.ties())
        );
    }
}
