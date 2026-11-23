use super::ExportedTask;
use super::exported_task_problems::ExportedTaskProblems;
use super::problems::ProjectProblems;
use crate::diagnostic::Diagnostics;
use crate::reader::Form;

pub struct ExportedTaskReader<'p> {
    section: &'p str,
    problems: &'p mut Diagnostics,
}

impl<'p> ExportedTaskReader<'p> {
    pub fn new(section: &'p str, problems: &'p mut Diagnostics) -> Self {
        Self { section, problems }
    }

    pub fn read(mut self, entries: &[Form], exported: &mut Vec<ExportedTask>) {
        for entry in entries {
            if let Some(task) = self.exported_task(entry, exported) {
                exported.push(task);
            }
        }
    }

    fn exported_task(&mut self, entry: &Form, exported: &[ExportedTask]) -> Option<ExportedTask> {
        let Some(symbol) = entry.as_symbol() else {
            self.problems.push(ExportedTaskProblems::unexpected_name(
                entry.span(),
                self.section,
            ));
            return None;
        };
        let task = ExportedTask {
            name: symbol.name().to_string(),
            span: entry.span(),
        };
        match exported.iter().find(|earlier| earlier.name == task.name) {
            Some(earlier) => {
                self.problems.push(ProjectProblems::duplicate_entry(
                    &task.name,
                    task.span,
                    earlier.span,
                ));
                None
            }
            None => Some(task),
        }
    }
}
