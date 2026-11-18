use crate::language::Language;

pub struct VersionedText {
    template: &'static str,
}

impl VersionedText {
    const VERSION_SLOT: &str = "{knave-version}";
    const PROJECT_FILE_SLOT: &str = "{project-file}";
    const VERSION_KEY_SLOT: &str = "{version-key}";
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    pub const fn new(template: &'static str) -> Self {
        Self { template }
    }

    pub fn filled(&self) -> String {
        self.named().replace(Self::VERSION_SLOT, Self::VERSION)
    }

    pub fn wrote(&self, text: &str) -> bool {
        let named = self.named();
        match Self::written_version(&named, text) {
            Some(version) => named.replace(Self::VERSION_SLOT, version) == text,
            None => named == text,
        }
    }

    fn named(&self) -> String {
        self.template
            .replace(Self::PROJECT_FILE_SLOT, &Language::project_file_name())
            .replace(Self::VERSION_KEY_SLOT, Language::NAME)
    }

    fn written_version<'t>(named: &str, text: &'t str) -> Option<&'t str> {
        let mut parts = named.split(Self::VERSION_SLOT);
        let before = parts.next()?;
        let after = parts.next()?;
        let rest = text.strip_prefix(before)?;
        let end = if after.is_empty() {
            rest.len()
        } else {
            rest.find(after)?
        };
        Some(&rest[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::VersionedText;

    const TEMPLATE: VersionedText =
        VersionedText::new("version: \"{knave-version}\"\nrun `knave` {knave-version}\n");

    #[test]
    fn a_template_recognizes_the_text_it_wrote_for_another_version() {
        assert!(TEMPLATE.wrote("version: \"0.1.0\"\nrun `knave` 0.1.0\n"));
    }

    #[test]
    fn a_template_refuses_a_text_whose_versions_disagree() {
        assert!(!TEMPLATE.wrote("version: \"0.1.0\"\nrun `knave` 0.2.0\n"));
    }

    #[test]
    fn a_template_refuses_a_text_with_another_start() {
        assert!(!TEMPLATE.wrote("edited: \"0.1.0\"\nrun `knave` 0.1.0\n"));
    }

    #[test]
    fn a_template_refuses_a_text_without_the_rest_of_the_template() {
        assert!(!TEMPLATE.wrote("version: \"0.1.0\"\n"));
    }

    #[test]
    fn a_template_refuses_a_changed_text() {
        assert!(!TEMPLATE.wrote("version: \"0.1.0\"\nrun `knave` 0.1.0\nmore\n"));
    }

    #[test]
    fn a_template_without_a_version_recognizes_only_itself() {
        let fixed = VersionedText::new("read {project-file} for :{version-key}\n");
        assert!(fixed.wrote("read project.knv for :knave\n"));
        assert!(!fixed.wrote("read project.knv for :knave\nmore\n"));
    }

    #[test]
    fn a_template_ending_in_its_version_takes_the_rest_as_the_version() {
        let trailing = VersionedText::new("knave {knave-version}");
        assert!(trailing.wrote("knave 0.3.0"));
    }
}
