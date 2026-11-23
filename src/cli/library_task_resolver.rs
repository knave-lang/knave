use std::rc::Rc;

use super::direct_library::DirectLibrary;
use super::exit_status::ExitStatus;
use super::standard_error::StandardError;
use crate::diagnostic::SimilarName;
use crate::language::Language;

pub struct LibraryTaskResolver<'l> {
    libraries: &'l [DirectLibrary],
}

impl<'l> LibraryTaskResolver<'l> {
    const TASK_COMMAND: &'static str = "task";

    pub fn new(libraries: &'l [DirectLibrary]) -> Self {
        Self { libraries }
    }

    pub fn has_exports(&self) -> bool {
        self.libraries
            .iter()
            .any(|library| !library.exported_tasks.is_empty())
    }

    pub fn qualified(&self, library: &str, task: &str) -> Result<&'l DirectLibrary, ExitStatus> {
        let Some(found) = self.libraries.iter().find(|listed| listed.name == library) else {
            return Err(self.report_unknown_library(library));
        };
        if found.exported_tasks.is_empty() {
            return Err(Self::report_no_exports(library));
        }
        if !Self::exports(found, task) {
            return Err(Self::report_unexported(library, task));
        }
        Ok(found)
    }

    pub fn bare(&self, task: &str, own: &[Rc<str>]) -> Result<&'l DirectLibrary, ExitStatus> {
        let exporters: Vec<&'l DirectLibrary> = self
            .libraries
            .iter()
            .filter(|library| Self::exports(library, task))
            .collect();
        match exporters.as_slice() {
            [] => Err(self.report_unknown_task(task, own)),
            [exporter] => Ok(exporter),
            clashing => Err(Self::report_clash(task, clashing)),
        }
    }

    fn exports(library: &DirectLibrary, task: &str) -> bool {
        library
            .exported_tasks
            .iter()
            .any(|exported| exported == task)
    }

    fn report_unknown_library(&self, library: &str) -> ExitStatus {
        let names: Vec<Rc<str>> = self
            .libraries
            .iter()
            .map(|listed| Rc::from(listed.name.as_str()))
            .collect();
        let help = match SimilarName::closest(library, &names) {
            Some(closest) => format!("did you mean `{closest}`?"),
            None => Self::listing_help(),
        };
        Self::report(&format!("no library named `{library}`"), &help)
    }

    fn report_no_exports(library: &str) -> ExitStatus {
        Self::report(
            &format!("library `{library}` exports no tasks"),
            &Self::listing_help(),
        )
    }

    fn report_unexported(library: &str, task: &str) -> ExitStatus {
        Self::report(
            &format!("library `{library}` exports no task named `{task}`"),
            &Self::listing_help(),
        )
    }

    fn report_unknown_task(&self, task: &str, own: &[Rc<str>]) -> ExitStatus {
        let rows = self.rows(own);
        let help = match SimilarName::closest(task, &rows) {
            Some(closest) => format!("did you mean `{closest}`?"),
            None => Self::listing_help(),
        };
        Self::report(&format!("no task named `{task}`"), &help)
    }

    fn report_clash(task: &str, clashing: &[&DirectLibrary]) -> ExitStatus {
        let libraries: Vec<String> = clashing
            .iter()
            .map(|library| format!("`{}`", library.name))
            .collect();
        let runs: Vec<String> = clashing
            .iter()
            .map(|library| {
                format!(
                    "`{} {} {}/{task}`",
                    Language::NAME,
                    Self::TASK_COMMAND,
                    library.name
                )
            })
            .collect();
        Self::report(
            &format!(
                "libraries {} export a task named `{task}`",
                Self::listed(&libraries, "and")
            ),
            &format!("run {}", Self::listed(&runs, "or")),
        )
    }

    fn rows(&self, own: &[Rc<str>]) -> Vec<Rc<str>> {
        let exported = self.libraries.iter().flat_map(|library| {
            library
                .exported_tasks
                .iter()
                .map(|task| Rc::from(self.row_name(library, task, own)))
        });
        own.iter().cloned().chain(exported).collect()
    }

    fn row_name(&self, library: &DirectLibrary, task: &str, own: &[Rc<str>]) -> String {
        let shadowed = own.iter().any(|name| name.as_ref() == task);
        let shared = self
            .libraries
            .iter()
            .filter(|other| Self::exports(other, task))
            .count()
            > 1;
        if shadowed || shared {
            format!("{}/{task}", library.name)
        } else {
            task.to_string()
        }
    }

    fn listed(items: &[String], conjunction: &str) -> String {
        match items.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} {conjunction} {last}", rest.join(", ")),
            None => String::new(),
        }
    }

    fn listing_help() -> String {
        format!(
            "run `{} {}` to list the tasks",
            Language::NAME,
            Self::TASK_COMMAND
        )
    }

    fn report(message: &str, help: &str) -> ExitStatus {
        StandardError::write_line(format_args!("error: {message}\nhelp: {help}"));
        ExitStatus::UsageError
    }
}
