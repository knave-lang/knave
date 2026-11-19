use super::versioned_text::VersionedText;

pub struct ReleasedText {
    version: &'static str,
    text: VersionedText,
}

impl ReleasedText {
    pub const fn new(version: &'static str, text: VersionedText) -> Self {
        Self { version, text }
    }

    pub fn version(&self) -> &'static str {
        self.version
    }

    pub fn text(&self) -> &VersionedText {
        &self.text
    }
}

macro_rules! released_text {
    ($template:literal, $version:literal) => {
        ReleasedText::new(
            $version,
            VersionedText::new(include_str!(concat!(
                "earlier_texts/",
                $template,
                "/",
                $version
            ))),
        )
    };
}

pub(crate) use released_text;

#[cfg(test)]
pub struct KeptTextsFolder {
    template: &'static str,
}

#[cfg(test)]
impl KeptTextsFolder {
    pub fn of(template: &'static str) -> Self {
        Self { template }
    }

    pub fn assert_listed_by(&self, earlier_texts: &[ReleasedText]) {
        let mut listed: Vec<&str> = earlier_texts.iter().map(ReleasedText::version).collect();
        listed.sort_unstable();
        assert_eq!(
            listed,
            self.kept_versions(),
            "`EARLIER_TEXTS` of `{}`",
            self.template
        );
    }

    fn kept_versions(&self) -> Vec<String> {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/cli/earlier_texts")
            .join(self.template);
        let mut versions: Vec<String> = std::fs::read_dir(folder)
            .expect("each owned template keeps a folder of earlier texts")
            .map(|entry| entry.expect("a kept text is readable").file_name())
            .map(|name| name.into_string().expect("a kept text's name is a version"))
            .collect();
        versions.sort_unstable();
        versions
    }
}
