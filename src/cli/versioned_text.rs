use crate::language::Language;

pub struct VersionedText {
    template: &'static str,
}

impl VersionedText {
    const VERSION_SLOT: &str = "{knave-version}";
    const PROJECT_FILE_SLOT: &str = "{project-file}";
    const VERSION_KEY_SLOT: &str = "{version-key}";

    pub const fn new(template: &'static str) -> Self {
        Self { template }
    }

    pub fn filled(&self) -> String {
        self.template
            .replace(Self::PROJECT_FILE_SLOT, &Language::project_file_name())
            .replace(Self::VERSION_KEY_SLOT, Language::NAME)
            .replace(Self::VERSION_SLOT, Language::VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::VersionedText;
    use crate::language::Language;

    #[test]
    fn a_template_fills_each_slot() {
        let template =
            VersionedText::new("read {project-file} for :{version-key} {knave-version}\n");
        assert_eq!(
            template.filled(),
            format!("read project.knv for :knave {}\n", Language::VERSION)
        );
    }
}
