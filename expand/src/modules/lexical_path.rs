use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexicalPath(PathBuf);

impl LexicalPath {
    pub fn of(path: &Path) -> Self {
        let mut lexical = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir if lexical.file_name().is_some() => {
                    lexical.pop();
                }
                other => lexical.push(other),
            }
        }
        Self(lexical)
    }

    pub fn absolute(path: &Path) -> Self {
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        Self::of(&absolute)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::LexicalPath;

    #[test]
    fn drops_each_current_folder_and_the_folder_before_each_parent_step() {
        let lexical = LexicalPath::of(Path::new("shop/./src/../tests/deep/../stock.knv"));
        assert_eq!(lexical.as_path(), Path::new("shop/tests/stock.knv"));
    }

    #[test]
    fn keeps_a_parent_step_with_no_folder_before_it() {
        let lexical = LexicalPath::of(Path::new("../shop/../../stock.knv"));
        assert_eq!(lexical.as_path(), Path::new("../../stock.knv"));
    }

    #[test]
    fn makes_a_relative_path_absolute_before_it_drops_the_steps() {
        let working = std::env::current_dir().expect("the working directory exists");
        let lexical = LexicalPath::absolute(Path::new("shop/src/../tests"));
        assert_eq!(lexical.into_path_buf(), working.join("shop/tests"));
    }
}
