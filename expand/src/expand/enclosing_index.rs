use crate::source::Span;

pub struct EnclosingIndex {
    call_sites: Vec<Span>,
    furthest_ends: Vec<usize>,
}

impl EnclosingIndex {
    pub fn of(call_sites: impl Iterator<Item = Span>) -> Self {
        let mut call_sites: Vec<Span> = call_sites.collect();
        call_sites.sort_by_key(|call_site| {
            let range = call_site.range();
            (call_site.source().index(), range.start, range.end)
        });
        let furthest_ends = Self::furthest_ends_of(&call_sites);
        Self {
            call_sites,
            furthest_ends,
        }
    }

    fn furthest_ends_of(call_sites: &[Span]) -> Vec<usize> {
        let mut furthest_ends = Vec::with_capacity(call_sites.len());
        let mut furthest = 0;
        let mut source = None;
        for call_site in call_sites {
            if source != Some(call_site.source()) {
                source = Some(call_site.source());
                furthest = 0;
            }
            furthest = furthest.max(call_site.range().end);
            furthest_ends.push(furthest);
        }
        furthest_ends
    }

    pub fn smallest_enclosing(&self, span: Span) -> Option<Span> {
        self.candidates(span)
            .filter(|call_site| call_site.contains(span))
            .min_by_key(|call_site| call_site.range().len())
    }

    fn candidates(&self, span: Span) -> impl Iterator<Item = Span> + '_ {
        let position = (span.source().index(), span.range().start);
        let starting_by_span = self.call_sites.partition_point(|call_site| {
            (call_site.source().index(), call_site.range().start) <= position
        });
        (0..starting_by_span)
            .rev()
            .take_while(move |index| self.reaches_end_of(*index, span))
            .map(|index| self.call_sites[index])
    }

    fn reaches_end_of(&self, index: usize, span: Span) -> bool {
        self.call_sites[index].source() == span.source()
            && self.furthest_ends[index] >= span.range().end
    }
}

#[cfg(test)]
mod tests {
    use super::EnclosingIndex;
    use crate::source::{SourceId, Span};

    struct IndexFixture;

    impl IndexFixture {
        const SIBLING_WIDTH: usize = 10;

        fn span(source: usize, start: usize, end: usize) -> Span {
            Span::new(SourceId::new(source), start..end)
        }

        fn index(call_sites: &[Span]) -> EnclosingIndex {
            EnclosingIndex::of(call_sites.iter().copied())
        }

        fn siblings(count: usize) -> Vec<Span> {
            (0..count)
                .map(|position| {
                    let start = position * Self::SIBLING_WIDTH;
                    Self::span(1, start, start + Self::SIBLING_WIDTH)
                })
                .collect()
        }
    }

    #[test]
    fn a_lookup_visits_only_the_call_sites_that_reach_the_span() {
        let index = IndexFixture::index(&IndexFixture::siblings(1_000));
        let last_sibling = IndexFixture::span(1, 9_991, 9_992);
        assert_eq!(index.candidates(last_sibling).count(), 1);
    }

    #[test]
    fn a_lookup_past_every_call_site_visits_nothing() {
        let index = IndexFixture::index(&IndexFixture::siblings(1_000));
        assert_eq!(
            index
                .candidates(IndexFixture::span(1, 20_000, 20_001))
                .count(),
            0
        );
    }

    #[test]
    fn picks_the_narrowest_call_site_around_a_span() {
        let wide = IndexFixture::span(1, 0, 100);
        let narrow = IndexFixture::span(1, 10, 20);
        let index = IndexFixture::index(&[wide, narrow]);
        let found = index.smallest_enclosing(IndexFixture::span(1, 12, 13));
        assert_eq!(found, Some(narrow));
    }

    #[test]
    fn finds_the_outer_call_site_past_a_finished_inner_one() {
        let outer = IndexFixture::span(1, 0, 100);
        let finished = IndexFixture::span(1, 10, 20);
        let index = IndexFixture::index(&[outer, finished]);
        let found = index.smallest_enclosing(IndexFixture::span(1, 50, 51));
        assert_eq!(found, Some(outer));
    }

    #[test]
    fn a_call_site_of_another_source_does_not_enclose() {
        let index = IndexFixture::index(&[IndexFixture::span(1, 0, 100)]);
        assert_eq!(
            index.smallest_enclosing(IndexFixture::span(2, 12, 13)),
            None
        );
    }

    #[test]
    fn a_call_site_of_an_earlier_source_does_not_stop_the_furthest_end() {
        let long = IndexFixture::span(1, 0, 1_000);
        let short = IndexFixture::span(2, 0, 10);
        let index = IndexFixture::index(&[long, short]);
        assert_eq!(
            index.smallest_enclosing(IndexFixture::span(2, 500, 501)),
            None
        );
        assert_eq!(index.candidates(IndexFixture::span(2, 500, 501)).count(), 0);
    }

    #[test]
    fn a_lookup_does_not_walk_into_an_earlier_source() {
        let index = IndexFixture::index(&[IndexFixture::span(1, 0, 1_000)]);
        assert_eq!(index.candidates(IndexFixture::span(2, 500, 501)).count(), 0);
    }

    #[test]
    fn a_call_site_that_starts_with_the_span_encloses_it() {
        let call_site = IndexFixture::span(1, 10, 20);
        let index = IndexFixture::index(&[call_site]);
        let found = index.smallest_enclosing(IndexFixture::span(1, 10, 15));
        assert_eq!(found, Some(call_site));
    }

    #[test]
    fn a_span_that_overruns_a_call_site_is_not_enclosed() {
        let index = IndexFixture::index(&[IndexFixture::span(1, 0, 10)]);
        assert_eq!(index.smallest_enclosing(IndexFixture::span(1, 5, 15)), None);
    }
}
