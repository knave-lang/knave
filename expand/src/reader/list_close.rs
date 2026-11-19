use super::Reader;
use super::syntax::Syntax;
use crate::source::{SourceMap, Span};

pub struct ListClose;

impl ListClose {
    pub fn of(sources: &SourceMap, list: Span, head: Span) -> usize {
        if Self::opens(list, head) {
            list.range().end - Syntax::CLOSE.len_utf8()
        } else {
            Self::after(sources, head)
        }
    }

    fn opens(list: Span, head: Span) -> bool {
        list.source() == head.source()
            && list.range().start + Syntax::OPEN.len_utf8() == head.range().start
    }

    fn after(sources: &SourceMap, head: Span) -> usize {
        let mut reader = Reader::starting_at(sources.file(head.source()), head.range().end);
        let mut unclosed = 0;
        while let Some(next) = reader.skip_trivia() {
            if next == Syntax::CLOSE && unclosed == 0 {
                break;
            }
            unclosed = reader.skip_list_part(next, unclosed);
        }
        reader.cursor.position()
    }
}
