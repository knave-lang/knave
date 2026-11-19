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
