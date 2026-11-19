use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

use super::build_directory::BuildDirectory;
use super::doc_write::DocWriteCommand;
use super::new_file::NewFile;
use super::project_component::ProjectComponent;
use super::standard_error::StandardError;

pub struct GitSetup;

impl ProjectComponent for GitSetup {
    fn files(&self, root: &Path) -> Vec<NewFile> {
        let text = format!(
            "/{}/\n/{}/\n",
            BuildDirectory::PARENT,
            DocWriteCommand::DEFAULT_FOLDER
        );
        vec![NewFile::new(root.join(Self::IGNORE_FILE), text)]
    }

    fn set_up(&self, root: &Path) {
        if !Self::is_inside_work_tree(root) {
            Self::init(root);
        }
    }
}

impl GitSetup {
    const GIT: &str = "git";
    const IGNORE_FILE: &str = ".gitignore";
    const INSIDE_WORK_TREE: [&str; 2] = ["rev-parse", "--is-inside-work-tree"];
    const INIT: [&str; 2] = ["init", "--quiet"];

    fn is_inside_work_tree(root: &Path) -> bool {
        Self::git(root, &Self::INSIDE_WORK_TREE)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn init(root: &Path) {
        match Self::git(root, &Self::INIT).status() {
            Ok(status) if status.success() => {}
            Ok(_) => Self::warn("`git init` failed"),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Self::warn("git is not installed");
            }
            Err(error) => Self::warn(&format!("git could not start: {error}")),
        }
    }

    fn git(root: &Path, arguments: &[&str]) -> Command {
        let mut command = Command::new(Self::GIT);
        command
            .args(arguments)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    fn warn(reason: &str) {
        StandardError::write_line(format_args!(
            "warning: {reason}, so the project has no git repository\nhelp: run `git init` in it later"
        ));
    }
}
