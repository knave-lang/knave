use std::fs;
use std::path::{Path, PathBuf};

use super::compiler_check::CompilerCheck;
use super::crate_section::CrateSection;
use super::embedded_prelude::EmbeddedPrelude;
use super::exit_status::ExitStatus;
use super::library_walk::FetchingStore;
use super::located_project::{LocatedProject, ProjectRoot};
use super::program_file::ProgramFile;
use super::standard_error::StandardError;
use super::terminal::Terminal;
use super::working_directory::WorkingDirectory;
use crate::format::ModuleLookup;
use crate::language::Language;
use crate::project::{CrateDependency, Project, ProjectEditor};

pub struct ProjectFileEdit {
    root: PathBuf,
    project: Project,
}

impl ProjectFileEdit {
    const STAGED_SUFFIX: &str = ".tmp";

    pub fn of_working_directory(
        terminal: &mut Terminal,
        command: &str,
    ) -> Result<Self, ExitStatus> {
        let working_directory = WorkingDirectory::path();
        let located =
            LocatedProject::enter_from(working_directory, terminal, FetchingStore::default())?;
        let Some(located) = located else {
            return Err(Self::report_no_project(command));
        };
        Self::of_root(located.root().to_path_buf(), terminal)
    }

    pub fn of_root_file(terminal: &Terminal, command: &str) -> Result<Self, ExitStatus> {
        let root = ProjectRoot::around(WorkingDirectory::path())
            .ok_or_else(|| Self::report_no_project(command))?;
        CompilerCheck::new(terminal).refuse_before_loading(&root)?;
        Self::of_root(root.shown()?, terminal)
    }

    fn of_root(root: PathBuf, terminal: &Terminal) -> Result<Self, ExitStatus> {
        let mut scratch = terminal.scratch();
        let project = scratch
            .sources()
            .projects()
            .load(&Self::project_file_in(&root))?;
        Ok(Self { root, project })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn listed_crate(&self, section: CrateSection, name: &str) -> Option<&CrateDependency> {
        section
            .listed(&self.project)
            .iter()
            .find(|dependency| dependency.name == name)
    }

    pub fn apply(
        &self,
        terminal: &Terminal,
        edit: impl Fn(&ProjectEditor<'_>) -> Option<String>,
    ) -> Result<bool, ExitStatus> {
        let path = Self::project_file_in(&self.root);
        let mut scratch = terminal.scratch();
        let source = scratch.sources().load(&path)?;
        let text = scratch.source_file(source).text().to_string();
        let forms = scratch.sources().read(source)?;
        let editor = ProjectEditor::of(&text, &forms, EmbeddedPrelude::project_vocabulary())
            .expect("a loaded project file holds a project form");
        let Some(edited) = edit(&editor) else {
            return Ok(false);
        };
        let formatted = self.formatted(&mut scratch, &path, edited);
        Self::replace(&path, &formatted)?;
        Ok(true)
    }

    fn project_file_in(root: &Path) -> PathBuf {
        root.join(Language::project_file_name())
    }

    fn replace(path: &Path, text: &str) -> Result<(), ExitStatus> {
        Self::replace_target(path, text)
            .map_err(|error| ProgramFile::report_unwritable(path, &error))
    }

    fn replace_target(path: &Path, text: &str) -> std::io::Result<()> {
        let path = &fs::canonicalize(path)?;
        let mut staged = path.as_os_str().to_os_string();
        staged.push(Self::STAGED_SUFFIX);
        let staged = Path::new(&staged);
        fs::write(staged, text)?;
        fs::set_permissions(staged, fs::metadata(path)?.permissions())?;
        fs::rename(staged, path)
    }

    fn formatted(&self, scratch: &mut Terminal, path: &Path, text: String) -> String {
        let directory = ProgramFile::directory_of(path);
        let root = ProgramFile::canonical(&directory).unwrap_or(directory);
        let source = scratch.sources().add(path.display(), text);
        EmbeddedPrelude::source_formatter()
            .format(
                scratch.source_file(source),
                &ModuleLookup::beside_project_file(&root),
            )
            .unwrap_or_else(|failure| {
                unreachable!("the formatter reads and keeps an edited project file: {failure:?}")
            })
            .text
    }

    fn report_no_project(command: &str) -> ExitStatus {
        StandardError::write_line(format_args!(
            "error: no `{}` in this directory or a parent\nhelp: run `{} {command}` inside a project",
            Language::project_file_name(),
            Language::NAME
        ));
        ExitStatus::UsageError
    }
}
