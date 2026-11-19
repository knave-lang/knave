use similar::{ChangeTag, TextDiff};

use super::owned_file::OwnedFile;
use super::standard_error::StandardError;
use crate::language::Language;

pub struct KeptFile<'f> {
    file: &'f OwnedFile,
    written: &'f str,
}

struct BaseText {
    version: &'static str,
    name: String,
    text: String,
}

impl<'f> KeptFile<'f> {
    pub fn new(file: &'f OwnedFile, written: &'f str) -> Self {
        Self { file, written }
    }

    pub fn report(&self) {
        let shown = self.shown();
        let base = self.closest_base();
        let version = base.version;
        let yours = base.changes_to(self.written, &shown);
        let current_name = format!("{shown} (Knave {})", Language::VERSION);
        let knaves = base.changes_to(&self.file.current_text(), &current_name);
        StandardError::write_line(format_args!(
            "warning: kept `{shown}`, because it differs from every text Knave wrote\n\
             help: below are your changes to the text Knave {version} wrote, then Knave's changes since {version}: merge Knave's changes into your file by hand\n\
             your changes to {shown}\n{yours}\
             Knave's changes since {version}\n{knaves}"
        ));
    }

    fn shown(&self) -> String {
        self.file.path().display().to_string()
    }

    fn closest_base(&self) -> BaseText {
        let version_written = self.file.version_in(self.written);
        self.file
            .bases()
            .map(|(version, text)| BaseText {
                version,
                name: format!("{} (Knave {version})", self.shown()),
                text: text.filled_as(version_written.unwrap_or(version)),
            })
            .min_by_key(|base| Self::changed_lines(&base.text, self.written))
            .expect("an owned file has a current text")
    }

    fn changed_lines(base: &str, written: &str) -> usize {
        TextDiff::from_lines(base, written)
            .iter_all_changes()
            .filter(|change| change.tag() != ChangeTag::Equal)
            .count()
    }
}

impl BaseText {
    fn changes_to(&self, changed: &str, changed_name: &str) -> String {
        TextDiff::from_lines(self.text.as_str(), changed)
            .unified_diff()
            .header(&self.name, changed_name)
            .to_string()
    }
}
