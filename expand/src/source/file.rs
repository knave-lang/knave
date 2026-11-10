use std::cell::OnceCell;
use std::ops::Range;

use super::{LineStarts, SourceId, TextPosition};

pub struct SourceFile {
    id: SourceId,
    name: String,
    text: String,
    excerpt: Range<usize>,
    line_starts: OnceCell<LineStarts>,
}

impl SourceFile {
    pub fn new(id: SourceId, name: String, text: String) -> Self {
        let whole = 0..text.len();
        Self::excerpt(id, name, text, whole)
    }

    pub fn excerpt(id: SourceId, name: String, text: String, excerpt: Range<usize>) -> Self {
        Self {
            id,
            name,
            text,
            excerpt,
            line_starts: OnceCell::new(),
        }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn excerpt_range(&self) -> Range<usize> {
        self.excerpt.clone()
    }

    pub fn position(&self, offset: usize) -> TextPosition {
        self.line_starts
            .get_or_init(|| LineStarts::of(&self.text))
            .position(offset)
    }
}

#[cfg(test)]
mod tests {
    use super::SourceFile;
    use crate::source::{SourceId, TextPosition};

    #[test]
    fn a_file_keeps_the_line_starts_it_finds_a_position_with() {
        let file = SourceFile::new(SourceId::new(0), "lines.knv".into(), "a\nb\n".into());
        assert!(file.line_starts.get().is_none());
        assert_eq!(file.position(2), TextPosition::new(2, 1));
        assert!(file.line_starts.get().is_some());
    }
}
