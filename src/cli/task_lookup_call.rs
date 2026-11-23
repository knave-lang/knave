use crate::diagnostic::Diagnostics;
use crate::expand::ExpansionSession;
use crate::reader::{Form, FormKind};

pub struct TaskLookupCall {
    text: String,
    arguments: Vec<Form>,
}

impl TaskLookupCall {
    pub fn new(text: String) -> Self {
        Self {
            text,
            arguments: Vec::new(),
        }
    }

    pub fn with_arguments(text: String, arguments: Vec<Form>) -> Self {
        Self { text, arguments }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn form(
        &self,
        session: &mut ExpansionSession<'_>,
        source_name: &str,
    ) -> Result<Form, Diagnostics> {
        let source = session.add_source(source_name, &self.text);
        let written = session.read(source)?.remove(0);
        let items = written.as_list().unwrap_or_default();
        let items = items.iter().chain(&self.arguments).cloned().collect();
        Ok(Form::new(FormKind::List(items), written.span()))
    }
}
