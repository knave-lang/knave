use similar::{ChangeTag, TextDiff};

pub struct BaseText<'w> {
    version: &'w str,
    name: String,
    text: String,
}

impl<'w> BaseText<'w> {
    pub fn new(version: &'w str, name: String, text: String) -> Self {
        Self {
            version,
            name,
            text,
        }
    }

    pub fn version(&self) -> &'w str {
        self.version
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn changes_to(&self, changed: &str, changed_name: &str) -> String {
        TextDiff::from_lines(self.text.as_str(), changed)
            .unified_diff()
            .header(&self.name, changed_name)
            .to_string()
    }

    pub fn changed_lines_to(&self, written: &str) -> usize {
        TextDiff::from_lines(self.text.as_str(), written)
            .iter_all_changes()
            .filter(|change| change.tag() != ChangeTag::Equal)
            .count()
    }
}
