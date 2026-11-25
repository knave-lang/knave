use rustc_hash::FxHashMap;
use std::cell::{OnceCell, RefCell};
use std::ops::Range;

use super::{EmbeddedSources, LineStarts, SourceFile, SourceId, Span, TextPosition};

#[derive(Default)]
pub struct SourceMap {
    files: Vec<OnceCell<SourceFile>>,
    line_starts: RefCell<FxHashMap<SourceId, LineStarts>>,
    embedded: Option<EmbeddedRange>,
}

struct EmbeddedRange {
    first: SourceId,
    sources: Box<dyn EmbeddedSources>,
}

impl SourceMap {
    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> SourceId {
        let id = self.next_id();
        let text: String = text.into();
        self.push(SourceFile::new(id, name.into(), text))
    }

    pub fn add_embedded(&mut self, sources: Box<dyn EmbeddedSources>) {
        assert!(
            self.embedded.is_none(),
            "a source map holds at most one embedded range"
        );
        let first = self.next_id();
        let end = first.index() + sources.count();
        self.files.resize_with(end, OnceCell::new);
        self.embedded = Some(EmbeddedRange { first, sources });
    }

    pub fn add_inside(
        &mut self,
        enclosing: SourceId,
        position: TextPosition,
        text: &str,
    ) -> Option<SourceId> {
        let enclosing_text = self.file(enclosing).text();
        let start = position.offset_in(enclosing_text)?;
        let range = start..start + text.len();
        let holds_text = enclosing_text.get(range.clone()) == Some(text);
        holds_text.then(|| self.add_excerpt(enclosing, range))
    }

    pub fn next_id(&self) -> SourceId {
        SourceId::new(self.files.len())
    }

    pub fn file(&self, id: SourceId) -> &SourceFile {
        self.files[id.index()].get_or_init(|| self.embedded_file(id))
    }

    pub fn start_position(&self, span: Span) -> TextPosition {
        let source = span.source();
        self.line_starts
            .borrow_mut()
            .entry(source)
            .or_insert_with(|| LineStarts::of(self.file(source).text()))
            .position(span.range().start)
    }

    pub fn location(&self, span: Span) -> String {
        let name = self.file(span.source()).name();
        format!("{name}:{}", self.start_position(span))
    }

    fn add_excerpt(&mut self, enclosing: SourceId, range: Range<usize>) -> SourceId {
        let id = self.next_id();
        let excerpt = self.file(enclosing).excerpt(id, range);
        self.push(excerpt)
    }

    fn push(&mut self, file: SourceFile) -> SourceId {
        let id = file.id();
        self.files.push(OnceCell::from(file));
        id
    }

    fn embedded_file(&self, id: SourceId) -> SourceFile {
        let range = self
            .embedded
            .as_ref()
            .expect("a source without a file is an embedded one");
        let position = id.index() - range.first.index();
        SourceFile::new(
            id,
            range.sources.name(position),
            range.sources.text(position),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::SourceMap;
    use crate::source::{EmbeddedSources, SourceId, Span, TextPosition};

    struct TwoEmbeddedFiles;

    impl EmbeddedSources for TwoEmbeddedFiles {
        fn count(&self) -> usize {
            2
        }

        fn name(&self, position: usize) -> String {
            format!("embedded-{position}.knv")
        }

        fn text(&self, _position: usize) -> &'static str {
            "(embedded)"
        }
    }

    struct InsideFixture {
        sources: SourceMap,
        enclosing: SourceId,
    }

    impl InsideFixture {
        const MODULE: &str = "geometry.knv";
        const TEXT: &str = "(defn area (w h) (* w h))\n(area 2 3)\n(area 4 5)\n";
        const SECOND_LINE: &str = "(area 2 3)";

        fn new() -> Self {
            let mut sources = SourceMap::default();
            let enclosing = sources.add(Self::MODULE, Self::TEXT);
            Self { sources, enclosing }
        }

        fn add(&mut self, position: TextPosition, text: &str) -> Option<SourceId> {
            self.sources.add_inside(self.enclosing, position, text)
        }

        fn second_line() -> TextPosition {
            TextPosition::new(2, 1)
        }
    }

    #[test]
    fn a_source_inside_another_starts_at_the_offset_of_its_position() {
        let mut fixture = InsideFixture::new();
        let inside = fixture
            .add(InsideFixture::second_line(), InsideFixture::SECOND_LINE)
            .expect("the text sits at that position");
        let file = fixture.sources.file(inside);
        let start = InsideFixture::TEXT
            .find(InsideFixture::SECOND_LINE)
            .expect("the text holds the line");
        assert_eq!(file.excerpt_range().start, start);
        assert_eq!(
            &file.text()[file.excerpt_range()],
            InsideFixture::SECOND_LINE
        );
        assert_eq!(file.text(), InsideFixture::TEXT);
        assert_eq!(file.name(), InsideFixture::MODULE);
    }

    #[test]
    fn a_source_inside_another_must_be_its_text_at_that_position() {
        let mut fixture = InsideFixture::new();
        assert_eq!(
            fixture.add(InsideFixture::second_line(), "(area 4 5)"),
            None
        );
    }

    #[test]
    fn a_source_inside_another_must_end_inside_its_text() {
        let mut fixture = InsideFixture::new();
        let past_the_end = format!("{}(", InsideFixture::TEXT);
        assert_eq!(fixture.add(TextPosition::new(1, 1), &past_the_end), None);
    }

    #[test]
    fn a_source_inside_another_needs_a_position_of_its_text() {
        let mut fixture = InsideFixture::new();
        assert_eq!(fixture.add(TextPosition::new(9, 1), ""), None);
    }

    #[test]
    fn a_map_keeps_the_line_starts_it_finds_a_position_with() {
        let fixture = InsideFixture::new();
        let second_line = InsideFixture::TEXT
            .find(InsideFixture::SECOND_LINE)
            .expect("the text holds the line");
        let span = Span::new(fixture.enclosing, second_line..second_line);
        assert!(fixture.sources.line_starts.borrow().is_empty());
        assert_eq!(
            fixture.sources.start_position(span),
            InsideFixture::second_line()
        );
        assert!(
            fixture
                .sources
                .line_starts
                .borrow()
                .contains_key(&fixture.enclosing)
        );
    }

    #[test]
    fn a_location_names_the_file_and_the_start_position() {
        let fixture = InsideFixture::new();
        let second_line = InsideFixture::TEXT
            .find(InsideFixture::SECOND_LINE)
            .expect("the text holds the line");
        let span = Span::new(fixture.enclosing, second_line..second_line);
        assert_eq!(fixture.sources.location(span), "geometry.knv:2:1");
    }

    #[test]
    fn embedded_sources_fill_no_slot_until_a_file_is_read() {
        let mut sources = SourceMap::default();
        let entry = sources.add("geometry.knv", "(area 2 3)");
        sources.add_embedded(Box::new(TwoEmbeddedFiles));
        assert!(sources.files[1..].iter().all(|slot| slot.get().is_none()));
        let second = SourceId::new(entry.index() + 2);
        assert_eq!(sources.file(second).name(), "embedded-1.knv");
        assert!(sources.files[1].get().is_none());
    }

    #[test]
    #[should_panic(expected = "a source map holds at most one embedded range")]
    fn a_second_embedded_range_is_refused() {
        let mut sources = SourceMap::default();
        sources.add_embedded(Box::new(TwoEmbeddedFiles));
        sources.add_embedded(Box::new(TwoEmbeddedFiles));
    }
}
