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

    fn named(&self) -> String {
        self.template
            .replace(Self::PROJECT_FILE_SLOT, &Language::project_file_name())
            .replace(Self::VERSION_KEY_SLOT, Language::NAME)
    }
}
