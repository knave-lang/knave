use super::base_text::BaseText;
use super::owned_file::OwnedFile;
use super::standard_error::StandardError;
use crate::language::Language;

pub struct KeptFile<'f> {
    file: &'f OwnedFile,
    written: &'f str,
}

impl<'f> KeptFile<'f> {
    pub fn new(file: &'f OwnedFile, written: &'f str) -> Self {
        Self { file, written }
    }

    pub fn report(&self) {
        let base = self.file.closest_base(self.written);
        let current_text = self.file.current_text();
        if base.text() == current_text {
            self.report_changes_to_current(&base);
        } else {
            self.report_changes_to_earlier(&base, &current_text);
        }
    }

    fn report_changes_to_current(&self, base: &BaseText<'_>) {
        let shown = self.shown();
        let version = base.version();
        let yours = base.changes_to(self.written, &shown);
        StandardError::write_line(format_args!(
            "warning: kept `{shown}`, because it differs from every text Knave wrote\n\
             help: below are your changes to the text Knave {version} wrote, and Knave's text has not changed since {version}\n\
             your changes to {shown}\n{yours}"
        ));
    }

    fn report_changes_to_earlier(&self, base: &BaseText<'_>, current_text: &str) {
        let shown = self.shown();
        let version = base.version();
        let yours = base.changes_to(self.written, &shown);
        let current_name = format!("{shown} (Knave {})", Language::VERSION);
        let knaves = base.changes_to(current_text, &current_name);
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
}
