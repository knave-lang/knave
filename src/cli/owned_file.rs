use std::path::{Path, PathBuf};

use super::base_text::BaseText;
use super::new_file::NewFile;
use super::released_text::ReleasedText;
use super::versioned_text::VersionedText;
use crate::language::Language;

pub struct OwnedFile {
    path: PathBuf,
    current: ReleasedText,
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
            current: ReleasedText::new(Language::VERSION, text),
            earlier_texts,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn current_text(&self) -> String {
        self.current.text().filled()
    }

    pub fn new_file(&self, root: &Path) -> NewFile {
        NewFile::new(root.join(&self.path), self.current_text())
    }

    pub fn state(&self, written: &str) -> OwnedFileState {
        if written == self.current_text() {
            return OwnedFileState::Current;
        }
        let knave_wrote = self
            .releases()
            .any(|released| released.text().wrote(written));
        if knave_wrote {
            OwnedFileState::WrittenByEarlierKnave
        } else {
            OwnedFileState::ChangedByUser
        }
    }

    pub fn closest_base<'w>(&self, written: &'w str) -> BaseText<'w> {
        let version_written = self.version_in(written);
        self.releases()
            .map(|released| {
                let version = version_written.unwrap_or(released.version());
                let name = format!("{} (Knave {version})", self.path.display());
                BaseText::new(version, name, released.text().filled_as(version))
            })
            .min_by_key(|base| base.changed_lines_to(written))
            .expect("an owned file has a current text")
    }

    fn version_in<'w>(&self, written: &'w str) -> Option<&'w str> {
        self.releases()
            .find_map(|released| released.text().version_in(written))
    }

    fn releases(&self) -> impl Iterator<Item = &ReleasedText> {
        self.earlier_texts
            .iter()
            .chain(std::iter::once(&self.current))
    }
}
