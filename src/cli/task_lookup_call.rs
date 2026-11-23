use std::rc::Rc;

use super::exported_task_names::ExportedTaskNames;
use crate::diagnostic::Diagnostics;
use crate::expand::ExpansionSession;
use crate::reader::{Form, FormKind};

pub struct TaskLookupCall {
    text: String,
    exported_task: Option<(Rc<ExportedTaskNames>, usize)>,
}

impl TaskLookupCall {
    pub fn new(text: String) -> Self {
        Self {
            text,
            exported_task: None,
        }
    }

    pub fn checking_export(text: String, names: Rc<ExportedTaskNames>, index: usize) -> Self {
        Self {
            text,
            exported_task: Some((names, index)),
        }
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
        let arguments = match &self.exported_task {
            Some((names, index)) => names.argument(session, *index)?,
            None => Vec::new(),
        };
        let items = written.as_list().unwrap_or_default();
        let items = items.iter().chain(&arguments).cloned().collect();
        Ok(Form::new(FormKind::List(items), written.span()))
    }
}
