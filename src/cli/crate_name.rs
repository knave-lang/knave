use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub struct CrateName {
    name: String,
}

impl CrateName {
    pub fn of(path: &Path) -> Self {
        let name = Self::module_name(path)
            .replace(|character: char| !character.is_ascii_alphanumeric(), "_");
        Self { name }
    }

    pub fn path_in(&self, directory: &Path) -> PathBuf {
        directory.join(&self.name)
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    fn module_name(path: &Path) -> String {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
