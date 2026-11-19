use super::node_id::NodeId;
use super::source::Source;
use super::transfer::Transfer;
use super::type_path::TypePath;
use crate::source::Span;

#[derive(Clone, Debug, Default)]
pub struct TextValue {
    sources: Vec<(Source, Transfer, Span)>,
    nested: Vec<(TypePath, NodeId)>,
}

impl TextValue {
    pub fn from(source: Source, span: Span) -> Self {
        Self {
            sources: vec![(source, Transfer::Move, span)],
            nested: Vec::new(),
        }
    }

    pub fn nesting(mut self, path: TypePath, node: NodeId) -> Self {
        self.nested.push((path, node));
        self
    }

    pub fn merged(mut self, other: Self) -> Self {
        self.sources.extend(other.sources);
        self.nested.extend(other.nested);
        self
    }

    pub fn transferred(self, transfer: Transfer) -> Self {
        Self {
            sources: self
                .sources
                .into_iter()
                .map(|(source, own, span)| (source, own.then(transfer), span))
                .collect(),
            nested: self.nested,
        }
    }

    pub fn sources(&self) -> &[(Source, Transfer, Span)] {
        &self.sources
    }

    pub fn nested(&self) -> &[(TypePath, NodeId)] {
        &self.nested
    }

    pub fn place(&self) -> Option<NodeId> {
        match self.sources.as_slice() {
            [(Source::Slot(node), Transfer::Move, _)] => Some(*node),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NodeId, Source, TextValue, Transfer};
    use crate::ownership::text::reaching_text::ReachingText;
    use crate::source::Span;

    struct Written;

    impl Written {
        fn span() -> Span {
            Span::new(crate::source::SourceId::new(0), 0..1)
        }
    }

    #[test]
    fn a_moved_slot_is_a_place() {
        let value = TextValue::from(Source::Slot(NodeId(3)), Written::span());
        assert_eq!(value.place(), Some(NodeId(3)));
    }

    #[test]
    fn a_referred_slot_is_no_place() {
        let value =
            TextValue::from(Source::Slot(NodeId(3)), Written::span()).transferred(Transfer::Refer);
        assert_eq!(value.place(), None);
    }

    #[test]
    fn merged_values_keep_every_source() {
        let text = TextValue::from(Source::Text(ReachingText::STATIC), Written::span());
        let merged = text.clone().merged(text);
        assert_eq!(merged.sources().len(), 2);
        assert_eq!(merged.place(), None);
    }
}
