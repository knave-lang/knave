use super::Expander;
use crate::core::BindingId;
use crate::reader::{Form, Reader};
use crate::source::Span;

impl Expander<'_> {
    pub(super) fn defining_call(&self, binding: BindingId) -> Option<Form> {
        let name = self.module_level_name(binding)?;
        let call_site = self.macro_origin_at(name)?.call_site;
        self.written_form(call_site)
    }

    fn module_level_name(&self, binding: BindingId) -> Option<Span> {
        self.definition_module(binding)?;
        self.tables.bindings.get(binding).span()
    }

    fn written_form(&self, span: Span) -> Option<Form> {
        let source = self.sources.file(span.source());
        Reader::read_form_at(source, span.range().start)
    }
}
