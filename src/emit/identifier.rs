pub struct Identifier<'a> {
    text: &'a str,
}

impl<'a> Identifier<'a> {
    pub fn ending(written: &'a str) -> Self {
        let before = written
            .trim_end_matches(|character: char| character.is_alphanumeric() || character == '_');
        Self {
            text: &written[before.len()..],
        }
    }

    pub fn text(&self) -> &'a str {
        self.text
    }
}
