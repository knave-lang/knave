use std::path::{Path, PathBuf};

use super::owned_file::OwnedFile;
use super::owned_line::OwnedLine;
use super::project_component::ProjectComponent;
use super::versioned_text::VersionedText;

pub struct GithubWorkflow;

impl GithubWorkflow {
    const WORKFLOW_PATH: &str = ".github/workflows/knave.yml";
    const WORKFLOW: VersionedText = VersionedText::new(include_str!("github_workflow.yml"));
    const ACTION_PATH: &str = ".github/actions/setup-knave/action.yml";
    const ACTION: VersionedText = VersionedText::new(include_str!("setup_knave_action.yml"));
    const READ_VERSION_PATH: &str = ".github/actions/setup-knave/read-version.sh";
    const READ_VERSION: VersionedText = VersionedText::new(include_str!("read_version.sh"));
    const GITATTRIBUTES: &str = ".gitattributes";
    const LINGUIST_LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";
    const LINGUIST_PREFIX: &str = "linguist-";
}

impl ProjectComponent for GithubWorkflow {
    fn owned_files(&self) -> Vec<OwnedFile> {
        vec![
            OwnedFile::new(PathBuf::from(Self::WORKFLOW_PATH), Self::WORKFLOW),
            OwnedFile::new(PathBuf::from(Self::ACTION_PATH), Self::ACTION),
            OwnedFile::new(PathBuf::from(Self::READ_VERSION_PATH), Self::READ_VERSION),
        ]
    }

    fn shows(&self, root: &Path) -> bool {
        root.join(Self::WORKFLOW_PATH).exists()
    }

    fn owned_lines(&self) -> Vec<OwnedLine> {
        vec![OwnedLine::new(
            Self::GITATTRIBUTES,
            Self::LINGUIST_LINE,
            Self::LINGUIST_PREFIX,
        )]
    }
}
