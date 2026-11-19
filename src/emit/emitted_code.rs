use std::collections::BTreeSet;

use super::runtime_part::RuntimePart;
use super::string_literals::StringLiterals;

pub struct EmittedCode<'a> {
    text: &'a str,
}

impl<'a> EmittedCode<'a> {
    const RUNTIME_PATH: &'static str = "crate::rt::";

    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    pub fn named_parts(&self) -> BTreeSet<RuntimePart> {
        self.text
            .match_indices(Self::RUNTIME_PATH)
            .filter_map(|(start, path)| RuntimePart::named(self.identifier_at(start + path.len())))
            .collect()
    }

    fn identifier_at(&self, start: usize) -> &'a str {
        let rest = &self.text[start..];
        let end = rest
            .find(|character| !Self::is_identifier_character(character))
            .unwrap_or(rest.len());
        &rest[..end]
    }

    pub fn contains(&self, marker: &str) -> bool {
        self.text.contains(marker)
    }

    pub fn contains_outside_strings(&self, marker: &str) -> bool {
        StringLiterals::code_spans(self.text)
            .iter()
            .any(|code| code.contains(marker))
    }

    pub fn contains_unprefixed(&self, marker: &str) -> bool {
        self.text
            .match_indices(marker)
            .any(|(start, _)| !self.continues_identifier(start))
    }

    fn continues_identifier(&self, start: usize) -> bool {
        self.text[..start]
            .chars()
            .next_back()
            .is_some_and(Self::is_identifier_character)
    }

    fn is_identifier_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }
}
