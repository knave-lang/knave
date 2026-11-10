use super::constructor::Constructor;
use super::cover_pattern::CoverPattern;
use crate::core::Literal;

#[derive(Default)]
pub(super) struct PatternMatrix {
    rows: Vec<Vec<CoverPattern>>,
}

impl FromIterator<CoverPattern> for PatternMatrix {
    fn from_iter<Patterns: IntoIterator<Item = CoverPattern>>(patterns: Patterns) -> Self {
        Self::of_rows(patterns.into_iter().map(|pattern| vec![pattern]).collect())
    }
}

impl PatternMatrix {
    pub(super) fn push(&mut self, pattern: CoverPattern) {
        self.rows.push(vec![pattern]);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(super) fn has_wild_row(&self) -> bool {
        self.rows.iter().any(|row| {
            row.iter()
                .all(|pattern| matches!(pattern, CoverPattern::Wild))
        })
    }

    pub(super) fn specialize(&self, constructor: &Constructor) -> Self {
        self.keep_rows(|first| constructor.arguments_of(first))
    }

    pub(super) fn specialize_literal(&self, literal: &Literal) -> Self {
        self.keep_rows(|first| first.matches_literal(literal).then(Vec::new))
    }

    pub(super) fn default_rows(&self) -> Self {
        self.keep_rows(|first| matches!(first, CoverPattern::Wild).then(Vec::new))
    }

    pub(super) fn concrete_rows(&self) -> Self {
        Self::of_rows(
            self.rows
                .iter()
                .filter(|row| !matches!(row.first(), Some(CoverPattern::Wild)))
                .cloned()
                .collect(),
        )
    }

    fn of_rows(rows: Vec<Vec<CoverPattern>>) -> Self {
        Self { rows }
    }

    fn keep_rows(&self, expand_first: impl Fn(&CoverPattern) -> Option<Vec<CoverPattern>>) -> Self {
        let rows = self
            .rows
            .iter()
            .filter_map(|row| {
                let mut expanded = expand_first(row.first()?)?;
                expanded.extend_from_slice(&row[1..]);
                Some(expanded)
            })
            .collect();
        Self::of_rows(rows)
    }
}
