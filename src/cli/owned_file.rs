use std::path::{Path, PathBuf};

use super::new_file::NewFile;
use super::versioned_text::VersionedText;

pub struct OwnedFile {
    path: PathBuf,
    text: VersionedText,
    earlier_texts: &'static [VersionedText],
}

#[derive(Debug, PartialEq)]
pub enum OwnedFileState {
    Current,
    WrittenByEarlierKnave,
    ChangedByUser,
}

impl OwnedFile {
    pub fn new(
        path: PathBuf,
        text: VersionedText,
        earlier_texts: &'static [VersionedText],
    ) -> Self {
        Self {
            path,
            text,
            earlier_texts,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn current_text(&self) -> String {
        self.text.filled()
    }

    pub fn new_file(&self, root: &Path) -> NewFile {
        NewFile::new(root.join(&self.path), self.current_text())
    }

    pub fn state(&self, written: &str) -> OwnedFileState {
        if written == self.current_text() {
            return OwnedFileState::Current;
        }
        let knave_wrote = std::iter::once(&self.text)
            .chain(self.earlier_texts)
            .any(|text| text.wrote(written));
        if knave_wrote {
            OwnedFileState::WrittenByEarlierKnave
        } else {
            OwnedFileState::ChangedByUser
        }
    }
}
