use std::borrow::Cow;
use std::ops::Range;

use super::SourceId;

pub struct SourceFile {
    id: SourceId,
    name: String,
    text: Cow<'static, str>,
    excerpt: Range<usize>,
}

impl SourceFile {
    pub fn new(id: SourceId, name: String, text: impl Into<Cow<'static, str>>) -> Self {
        let text = text.into();
        let excerpt = 0..text.len();
        Self {
            id,
            name,
            text,
            excerpt,
        }
    }

    pub fn excerpt(&self, id: SourceId, excerpt: Range<usize>) -> Self {
        Self {
            id,
            name: self.name.clone(),
            text: self.text.clone(),
            excerpt,
        }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_generated(&self) -> bool {
        self.name.starts_with('<') && self.name.ends_with('>')
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn excerpt_range(&self) -> Range<usize> {
        self.excerpt.clone()
    }
}
