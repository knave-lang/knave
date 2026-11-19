use std::path::{Path, PathBuf};

use super::new_file::NewFile;
use super::released_text::ReleasedText;
use super::versioned_text::VersionedText;
use crate::language::Language;

pub struct OwnedFile {
    path: PathBuf,
    text: VersionedText,
    earlier_texts: &'static [ReleasedText],
}

pub enum OwnedFileState {
    Current,
    WrittenByEarlierKnave,
    ChangedByUser,
}

impl OwnedFile {
    pub fn new(path: PathBuf, text: VersionedText, earlier_texts: &'static [ReleasedText]) -> Self {
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

    pub fn bases(&self) -> impl Iterator<Item = (&'static str, &VersionedText)> {
        self.earlier_texts
            .iter()
            .map(|released| (released.version(), released.text()))
            .chain(std::iter::once((Language::VERSION, &self.text)))
    }

    pub fn version_in<'w>(&self, written: &'w str) -> Option<&'w str> {
        self.bases().find_map(|(_, text)| text.version_in(written))
    }

    pub fn state(&self, written: &str) -> OwnedFileState {
        if written == self.current_text() {
            return OwnedFileState::Current;
        }
        let knave_wrote = self.bases().any(|(_, text)| text.wrote(written));
        if knave_wrote {
            OwnedFileState::WrittenByEarlierKnave
        } else {
            OwnedFileState::ChangedByUser
        }
    }
}
