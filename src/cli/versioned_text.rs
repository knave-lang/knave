pub struct VersionedText {
    template: &'static str,
}

impl VersionedText {
    const VERSION_SLOT: &str = "{knave-version}";
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    pub const fn new(template: &'static str) -> Self {
        Self { template }
    }

    pub fn filled(&self) -> String {
        self.template.replace(Self::VERSION_SLOT, Self::VERSION)
    }
}
