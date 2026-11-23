use std::rc::Rc;

use super::direct_library::DirectLibrary;
use super::exit_status::ExitStatus;
use super::lending_libraries::LendingLibraries;
use super::standard_error::StandardError;
use crate::diagnostic::SimilarName;
use crate::language::Language;

pub struct TaskNameLookup<'l> {
    lending: &'l LendingLibraries,
    command: &'static str,
}

impl<'l> TaskNameLookup<'l> {
    pub fn new(lending: &'l LendingLibraries, command: &'static str) -> Self {
        Self { lending, command }
    }

    pub fn qualified(&self, library: &str, task: &str) -> Result<&'l DirectLibrary, ExitStatus> {
        let Some(found) = self.lending.named(library) else {
            return Err(self.report_unknown_library(library));
        };
        if !found.lends_tasks() {
            return Err(self.report_no_exports(library));
        }
        if !found.exports(task) {
            return Err(self.report_unexported(library, task));
        }
        Ok(found)
    }

    pub fn bare(&self, task: &str, own: &[Rc<str>]) -> Result<&'l DirectLibrary, ExitStatus> {
        match self.lending.exporters_of(task).as_slice() {
            [] => Err(self.report_unknown_task(task, own)),
            [exporter] => Ok(exporter),
            clashing => Err(self.report_clash(task, clashing)),
        }
    }

    fn report_unknown_task(&self, task: &str, own: &[Rc<str>]) -> ExitStatus {
        let rows = self.lending.rows(own);
        let help = match SimilarName::closest(task, &rows) {
            Some(closest) => format!("did you mean `{closest}`?"),
            None if rows.is_empty() => "no `deftask` under `tasks/` defines a task yet".to_string(),
            None => self.listing_help(),
        };
        Self::report(&format!("no task named `{task}`"), &help)
    }

    fn report_unknown_library(&self, library: &str) -> ExitStatus {
        let help = match SimilarName::closest(library, &self.lending.names()) {
            Some(closest) => format!("did you mean `{closest}`?"),
            None => self.listing_help(),
        };
        Self::report(&format!("no library named `{library}`"), &help)
    }

    fn report_no_exports(&self, library: &str) -> ExitStatus {
        Self::report(
            &format!("library `{library}` exports no tasks"),
            &self.listing_help(),
        )
    }

    fn report_unexported(&self, library: &str, task: &str) -> ExitStatus {
        Self::report(
            &format!("library `{library}` exports no task named `{task}`"),
            &self.listing_help(),
        )
    }

    fn report_clash(&self, task: &str, clashing: &[&DirectLibrary]) -> ExitStatus {
        let libraries: Vec<String> = clashing
            .iter()
            .map(|library| format!("`{}`", library.name()))
            .collect();
        let runs: Vec<String> = clashing
            .iter()
            .map(|library| self.run_of(&format!("{}/{task}", library.name())))
            .collect();
        Self::report(
            &format!(
                "libraries {} export a task named `{task}`",
                Self::listed(&libraries, "and")
            ),
            &format!("run {}", Self::listed(&runs, "or")),
        )
    }

    fn listed(items: &[String], conjunction: &str) -> String {
        match items.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} {conjunction} {last}", rest.join(", ")),
            None => String::new(),
        }
    }

    fn listing_help(&self) -> String {
        format!("run {} to list the tasks", self.run_of(""))
    }

    fn run_of(&self, task: &str) -> String {
        let words = [Language::NAME, self.command, task];
        format!("`{}`", words.join(" ").trim_end())
    }

    fn report(message: &str, help: &str) -> ExitStatus {
        StandardError::write_line(format_args!("error: {message}\nhelp: {help}"));
        ExitStatus::UsageError
    }
}
