use std::path::Path;

use super::new_file::NewFile;
use super::project_component::ProjectComponent;
use super::versioned_text::VersionedText;

pub struct GithubWorkflow;

impl GithubWorkflow {
    const PATH: &str = ".github/workflows/knave.yml";
    const TEXT: VersionedText = VersionedText::new(include_str!("github_workflow.yml"));
}

impl ProjectComponent for GithubWorkflow {
    fn files(&self, root: &Path) -> Vec<NewFile> {
        vec![NewFile {
            path: root.join(Self::PATH),
            text: Self::TEXT.filled(),
        }]
    }
}
