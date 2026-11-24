use std::path::{Path, PathBuf};

use super::owned_file::OwnedFile;
use super::owned_line::OwnedLine;
use super::project_component::ProjectComponent;
use super::versioned_text::VersionedText;

pub struct GithubChecks;

impl GithubChecks {
    const WORKFLOW_PATH: &str = ".github/workflows/knave.yml";
    const WORKFLOW: VersionedText = VersionedText::new(include_str!("github_workflow.yml"));
    const ACTION_PATH: &str = ".github/actions/setup-knave/action.yml";
    const ACTION: VersionedText = VersionedText::new(include_str!("setup_knave_action.yml"));
    const VERSION_SCRIPT_PATH: &str = ".github/actions/setup-knave/read-version.sh";
    const VERSION_SCRIPT: VersionedText =
        VersionedText::new(include_str!("setup_knave_read_version.sh"));
    const GITATTRIBUTES: &str = ".gitattributes";
    const LINGUIST_LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";
    const LINGUIST_PREFIX: &str = "linguist-";

    fn workflow() -> OwnedFile {
        OwnedFile::new(PathBuf::from(Self::WORKFLOW_PATH), Self::WORKFLOW)
    }
}

impl ProjectComponent for GithubChecks {
    fn owned_files(&self) -> Vec<OwnedFile> {
        vec![
            Self::workflow(),
            OwnedFile::new(PathBuf::from(Self::ACTION_PATH), Self::ACTION),
            OwnedFile::new(
                PathBuf::from(Self::VERSION_SCRIPT_PATH),
                Self::VERSION_SCRIPT,
            ),
        ]
    }

    fn shows(&self, root: &Path) -> bool {
        Self::workflow().exists_in(root)
    }

    fn owned_lines(&self) -> Vec<OwnedLine> {
        vec![OwnedLine::new(
            Self::GITATTRIBUTES,
            Self::LINGUIST_LINE,
            Self::LINGUIST_PREFIX,
        )]
    }
}
