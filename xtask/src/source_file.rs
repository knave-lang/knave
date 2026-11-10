use std::fs;
use std::path::{Path, PathBuf};

use crate::failure::TaskFailure;

pub struct SourceFile {
    path: PathBuf,
    relative: String,
    line_count: usize,
    syntax: syn::File,
}

impl SourceFile {
    pub const EXTENSION: &str = "rs";

    pub fn read(path: &Path, relative: String) -> Result<Self, TaskFailure> {
        let text =
            fs::read_to_string(path).map_err(|error| TaskFailure::unreadable(path, error))?;
        Self::parse(path.to_path_buf(), relative, &text)
    }

    pub fn parse(path: PathBuf, relative: String, text: &str) -> Result<Self, TaskFailure> {
        let syntax =
            syn::parse_file(text).map_err(|error| TaskFailure::unparsable(&path, &error))?;
        Ok(Self {
            path,
            relative,
            line_count: text.lines().count(),
            syntax,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn relative(&self) -> &str {
        &self.relative
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn syntax(&self) -> &syn::File {
        &self.syntax
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::SourceFile;

    #[test]
    fn unparsable_source_names_its_file() {
        let failure = SourceFile::parse(PathBuf::from("src/a.rs"), "src/a.rs".to_string(), "fn {")
            .err()
            .expect("the source is unparsable");
        assert!(
            failure.to_string().starts_with("cannot parse `src/a.rs`"),
            "{failure}"
        );
    }

    #[test]
    fn counts_the_physical_lines() {
        let source = SourceFile::parse(
            PathBuf::from("src/a.rs"),
            "src/a.rs".to_string(),
            "fn a() {}\n\nfn b() {}\n",
        )
        .expect("the source parses");
        assert_eq!(source.line_count(), 3);
    }
}
