use std::path::PathBuf;

use super::owned_file::OwnedFile;
use super::project_component::ProjectComponent;
use super::versioned_text::VersionedText;

pub struct GithubWorkflow;

impl GithubWorkflow {
    const PATH: &str = ".github/workflows/knave.yml";
    const TEXT: VersionedText = VersionedText::new(include_str!("github_workflow.yml"));
    const EARLIER_TEXTS: &[VersionedText] = &[VersionedText::new(include_str!(
        "earlier_texts/src/cli/github_workflow.yml/0.18.0"
    ))];
}

impl ProjectComponent for GithubWorkflow {
    fn owned_files(&self) -> Vec<OwnedFile> {
        vec![OwnedFile::new(
            PathBuf::from(Self::PATH),
            Self::TEXT,
            Self::EARLIER_TEXTS,
        )]
    }
}
