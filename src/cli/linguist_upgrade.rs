pub enum LinguistUpgrade {
    Appended(String),
    Replaced(String),
}

impl LinguistUpgrade {
    pub fn text(&self) -> &str {
        match self {
            LinguistUpgrade::Appended(text) | LinguistUpgrade::Replaced(text) => text,
        }
    }
}
