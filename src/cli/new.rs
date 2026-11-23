use std::fs;
use std::path::{Path, PathBuf};

use super::embedded_prelude::EmbeddedPrelude;
use super::exit_status::ExitStatus;
use super::new_file::NewFile;
use super::project_component::ProjectComponent;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use super::working_directory::WorkingDirectory;
use crate::diagnostic::Diagnostics;
use crate::format::{ModuleLookup, SourceFormatter};
use crate::language::Language;
use crate::modules::SearchScope;
use crate::project::ProjectName;
use crate::source::Span;

pub struct NewCommand {
    name: String,
    components: Vec<Box<dyn ProjectComponent>>,
    formatter: SourceFormatter,
}

impl NewCommand {
    const GREETING_FUNCTION: &str = "greeting";
    const GREETING: &str = "\"hello\"";
    const NAME_ARGUMENT: &str = "<name argument>";

    pub fn new(name: String, components: Vec<Box<dyn ProjectComponent>>) -> Self {
        Self {
            name,
            components,
            formatter: EmbeddedPrelude::source_formatter(),
        }
    }

    pub fn run(mut self, terminal: &mut Terminal) -> ExitStatus {
        let root = PathBuf::from(&self.name);
        match self.files(terminal, &root) {
            Ok(files) => self.write_all(&root, &files),
            Err(status) => status,
        }
    }

    fn files(&mut self, terminal: &mut Terminal, root: &Path) -> Result<Vec<NewFile>, ExitStatus> {
        self.check_name(terminal)?;
        Self::refuse_existing(root)?;
        let mut files = self.base_files(terminal, root)?;
        files.extend(
            self.components
                .iter()
                .flat_map(|component| component.files(root)),
        );
        Ok(files)
    }

    fn base_files(
        &mut self,
        terminal: &mut Terminal,
        root: &Path,
    ) -> Result<Vec<NewFile>, ExitStatus> {
        let project_file = self.project_file(root);
        let entry_file = Self::entry_file(terminal, root, &project_file)?;
        let starter_test = Self::starter_test(root, entry_file.path());
        let working_directory = WorkingDirectory::path();
        let source_lookup = ModuleLookup::beside(working_directory);
        Ok(vec![
            self.formatted(
                terminal,
                project_file,
                &ModuleLookup::beside_project_file(working_directory),
            ),
            self.formatted(terminal, entry_file, &source_lookup),
            self.formatted(terminal, starter_test, &source_lookup),
        ])
    }

    fn project_file(&self, root: &Path) -> NewFile {
        let text = format!(
            "({} {} :{} \"{}\")\n",
            EmbeddedPrelude::project_vocabulary().head(),
            self.name,
            Language::NAME,
            Language::VERSION
        );
        NewFile::new(root.join(Language::project_file_name()), text)
    }

    fn entry_file(
        terminal: &mut Terminal,
        root: &Path,
        project_file: &NewFile,
    ) -> Result<NewFile, ExitStatus> {
        let mut sources = terminal.sources();
        let source = sources.add(
            project_file.path().display(),
            project_file.text().to_string(),
        );
        let project = sources.projects().read(source)?;
        let function = Self::GREETING_FUNCTION;
        let text = format!(
            "(export {function})\n\n(defn {function} () {})\n\n(defn main ()\n  (println ({function})))\n",
            Self::GREETING
        );
        Ok(NewFile::new(root.join(&project.modules.entry), text))
    }

    fn starter_test(root: &Path, entry: &Path) -> NewFile {
        let function = Self::GREETING_FUNCTION;
        let entry_module = entry
            .file_stem()
            .expect("the project's entry names a file")
            .display();
        let text = format!(
            "(use {entry_module} {function})\n\n(deftest {function}-says-hello\n  (assert (= ({function}) {})))\n",
            Self::GREETING
        );
        let file_name = format!("{function}.{}", Language::EXTENSION);
        NewFile::new(
            root.join(SearchScope::TESTS_DIRECTORY).join(file_name),
            text,
        )
    }

    fn check_name(&self, terminal: &mut Terminal) -> Result<(), ExitStatus> {
        let source = terminal
            .sources()
            .add(Self::NAME_ARGUMENT, self.name.clone());
        let whole_name = Span::new(source, 0..self.name.len());
        ProjectName::check(&self.name, whole_name).map_err(|problem| {
            let mut problems = Diagnostics::default();
            problems.push(problem);
            terminal.source_report().report(&problems)
        })
    }

    fn refuse_existing(root: &Path) -> Result<(), ExitStatus> {
        if !root.exists() {
            return Ok(());
        }
        StandardError::write_line(format_args!(
            "error: `{}` already exists\nhelp: choose another name, or remove it first",
            root.display()
        ));
        Err(ExitStatus::UsageError)
    }

    fn formatted(
        &mut self,
        terminal: &mut Terminal,
        file: NewFile,
        lookup: &ModuleLookup,
    ) -> NewFile {
        let source = terminal
            .sources()
            .add(file.path().display(), file.text().to_string());
        let text = self
            .formatter
            .format(terminal.source_file(source), lookup)
            .unwrap_or_else(|failure| {
                unreachable!("the formatter reads and keeps a new project's files: {failure:?}")
            })
            .text;
        file.with_text(text)
    }

    fn write_all(&self, root: &Path, files: &[NewFile]) -> ExitStatus {
        match files.iter().find_map(|file| file.write().err()) {
            Some(failure) => {
                fs::remove_dir_all(root).ok();
                failure
            }
            None => self.set_up_all(root),
        }
    }

    fn set_up_all(&self, root: &Path) -> ExitStatus {
        for component in &self.components {
            component.set_up(root);
        }
        ExitStatus::Success
    }
}
