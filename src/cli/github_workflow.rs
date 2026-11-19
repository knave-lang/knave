use std::path::PathBuf;

use super::owned_file::OwnedFile;
use super::project_component::ProjectComponent;
use super::released_text::{ReleasedText, released_text};
use super::versioned_text::VersionedText;

pub struct GithubWorkflow;

impl GithubWorkflow {
    const PATH: &str = ".github/workflows/knave.yml";
    const TEXT: VersionedText = VersionedText::new(include_str!("github_workflow.yml"));
    const EARLIER_TEXTS: &[ReleasedText] =
        &[released_text!("src/cli/github_workflow.yml", "0.18.0")];
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

#[cfg(test)]
mod tests {
    use super::GithubWorkflow;
    use crate::cli::released_text::KeptTextsFolder;

    #[test]
    fn the_workflow_lists_each_kept_text() {
        KeptTextsFolder::of("src/cli/github_workflow.yml")
            .assert_listed_by(GithubWorkflow::EARLIER_TEXTS);
    }
}
