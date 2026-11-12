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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::CrateName;

    #[test]
    fn replaces_each_character_rust_rejects_in_the_file_stem() {
        let name = CrateName::of(Path::new("shapes/two-dimensional.knv"));
        assert_eq!(name.as_str(), "two_dimensional");
    }

    #[test]
    fn is_empty_for_a_path_without_a_file_stem() {
        assert_eq!(CrateName::of(Path::new("")).as_str(), "");
    }
}
