use std::path::Path;

use super::new_file::NewFile;
use super::project_component::ProjectComponent;

pub struct GithubLinguist;

impl GithubLinguist {
    const PATH: &str = ".gitattributes";
    const LINE: &str = "*.knv linguist-language=Clojure linguist-detectable=false";
}

impl ProjectComponent for GithubLinguist {
    fn files(&self, root: &Path) -> Vec<NewFile> {
        vec![NewFile::new(
            root.join(Self::PATH),
            format!("{}\n", Self::LINE),
        )]
    }
}
