use std::path::PathBuf;

use super::owned_file::OwnedFile;
use super::owned_line::OwnedLine;
use super::project_component::ProjectComponent;
use super::versioned_text::VersionedText;

pub struct GithubWorkflow;

impl GithubWorkflow {
    const PATH: &str = ".github/workflows/knave.yml";
    const TEXT: VersionedText = VersionedText::new(include_str!("github_workflow.yml"));
    const GITATTRIBUTES: &str = ".gitattributes";
    const LINGUIST_LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";
    const LINGUIST_PREFIX: &str = "linguist-";
}

impl ProjectComponent for GithubWorkflow {
    fn owned_files(&self) -> Vec<OwnedFile> {
        vec![OwnedFile::new(PathBuf::from(Self::PATH), Self::TEXT)]
    }

    fn owned_lines(&self) -> Vec<OwnedLine> {
        vec![OwnedLine::new(
            Self::GITATTRIBUTES,
            Self::LINGUIST_LINE,
            Self::LINGUIST_PREFIX,
        )]
    }
}
