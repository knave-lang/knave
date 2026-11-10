use super::constructors::Constructor;
use super::pattern::CoverPattern;
use crate::types::Type;

#[derive(Clone, Copy)]
pub(super) struct Query<'q> {
    patterns: &'q [CoverPattern],
    types: &'q [Type],
}

pub(super) struct ExpandedQuery {
    patterns: Vec<CoverPattern>,
    types: Vec<Type>,
}

impl<'q> Query<'q> {
    pub(super) fn new(patterns: &'q [CoverPattern], types: &'q [Type]) -> Self {
        Self { patterns, types }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    pub(super) fn first_pattern(&self) -> &'q CoverPattern {
        &self.patterns[0]
    }

    pub(super) fn first_type(&self) -> &'q Type {
        &self.types[0]
    }

    pub(super) fn rest(&self) -> Self {
        Self::new(&self.patterns[1..], &self.types[1..])
    }

    pub(super) fn expanded(
        &self,
        constructor: &Constructor,
        arguments: &[CoverPattern],
    ) -> ExpandedQuery {
        ExpandedQuery {
            patterns: [arguments, &self.patterns[1..]].concat(),
            types: constructor.columns_with(&self.types[1..]),
        }
    }
}

impl ExpandedQuery {
    pub(super) fn query(&self) -> Query<'_> {
        Query::new(&self.patterns, &self.types)
    }
}
