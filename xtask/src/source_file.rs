use std::fs;
use std::path::{Path, PathBuf};

use crate::failure::TaskFailure;

pub struct SourceFile {
    path: PathBuf,
    relative: String,
    text: String,
}

impl SourceFile {
    pub fn read(path: &Path, relative: String) -> Result<Self, TaskFailure> {
        let text =
            fs::read_to_string(path).map_err(|error| TaskFailure::unreadable(path, error))?;
        Ok(Self::new(path.to_path_buf(), relative, text))
    }

    pub fn new(path: PathBuf, relative: String, text: String) -> Self {
        Self {
            path,
            relative,
            text,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn relative(&self) -> &str {
        &self.relative
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn syntax(&self) -> Result<syn::File, TaskFailure> {
        syn::parse_file(&self.text).map_err(|error| TaskFailure::unparsable(&self.path, &error))
    }
}
