use super::source::Source;
use super::text_value::TextValue;
use crate::source::Span;

pub struct ValueParts<'v> {
    value: &'v TextValue,
}

impl<'v> ValueParts<'v> {
    pub fn of(value: &'v TextValue) -> Self {
        Self { value }
    }

    pub fn part(&self, index: usize, span: Span) -> TextValue {
        self.value
            .nested()
            .iter()
            .filter_map(|(path, node)| Some((path.below(index)?, *node)))
            .fold(TextValue::default(), |part, (path, node)| {
                if path.is_top() {
                    part.merged(TextValue::from(Source::Slot(node), span))
                } else {
                    part.nesting(path, node)
                }
            })
    }

    pub fn top(&self) -> TextValue {
        self.value
            .sources()
            .iter()
            .fold(TextValue::default(), |top, (source, transfer, span)| {
                top.merged(TextValue::from(*source, *span).transferred(*transfer))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{TextValue, ValueParts};
    use crate::ownership::text::node_id::NodeId;
    use crate::ownership::text::source::Source;
    use crate::ownership::text::transfer::Transfer;
    use crate::ownership::text::type_path::TypePath;
    use crate::source::{SourceId, Span};

    struct Written;

    impl Written {
        fn span() -> Span {
            Span::new(SourceId::new(0), 0..1)
        }
    }

    #[test]
    fn a_part_of_a_value_reads_that_part_s_text() {
        let path = TypePath::default().child(1);
        let value = TextValue::default()
            .nesting(path.child(0), NodeId(4))
            .nesting(path, NodeId(5))
            .nesting(TypePath::default().child(0), NodeId(6));
        let part = ValueParts::of(&value).part(1, Written::span());
        assert_eq!(part.place(), Some(NodeId(5)));
        assert_eq!(part.nested(), &[(TypePath::default().child(0), NodeId(4))]);
    }

    #[test]
    fn the_top_of_a_value_keeps_its_sources_and_drops_its_parts() {
        let value = TextValue::from(Source::Slot(NodeId(2)), Written::span())
            .transferred(Transfer::Refer)
            .nesting(TypePath::default().child(0), NodeId(3));
        let top = ValueParts::of(&value).top();
        assert_eq!(top.sources().len(), 1);
        assert_eq!(top.sources()[0].1, Transfer::Refer);
        assert!(top.nested().is_empty());
    }
}
