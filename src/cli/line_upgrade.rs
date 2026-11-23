pub struct LineUpgrade {
    text: String,
    announcement: String,
}

impl LineUpgrade {
    pub fn new(text: String, announcement: String) -> Self {
        Self { text, announcement }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn announcement(&self) -> &str {
        &self.announcement
    }
}
