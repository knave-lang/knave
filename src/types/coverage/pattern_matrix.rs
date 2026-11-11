use super::constructor::Constructor;
use super::cover_pattern::CoverPattern;

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
        self.rows
            .iter()
            .any(|row| row.iter().all(CoverPattern::is_wild))
    }

    pub(super) fn specialize(&self, constructor: &Constructor) -> Self {
        self.keep_rows(|first| constructor.arguments_of(first))
    }

    pub(super) fn specialize_literal(&self, head: &CoverPattern) -> Self {
        self.keep_rows(|first| first.covers(head).then(Vec::new))
    }

    pub(super) fn default_rows(&self) -> Self {
        self.keep_rows(|first| first.is_wild().then(Vec::new))
    }

    pub(super) fn heads_cover(&self, constructors: &[Constructor]) -> bool {
        constructors.iter().all(|constructor| {
            self.rows
                .iter()
                .any(|row| row.first().and_then(CoverPattern::head_tag) == Some(constructor.tag()))
        })
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

#[cfg(test)]
mod tests {
    use super::PatternMatrix;
    use crate::core::{BindingId, Literal, PrimitiveType};
    use crate::types::Type;
    use crate::types::coverage::constructor::Constructor;
    use crate::types::coverage::constructor_tag::ConstructorTag;
    use crate::types::coverage::cover_pattern::CoverPattern;

    struct MatrixFixture {
        owner: BindingId,
    }

    impl MatrixFixture {
        fn new() -> Self {
            Self {
                owner: BindingId::from_index(0),
            }
        }

        fn rows(patterns: Vec<CoverPattern>) -> PatternMatrix {
            patterns.into_iter().collect()
        }

        fn variant(&self, index: usize) -> CoverPattern {
            CoverPattern::constructed(
                ConstructorTag::Variant {
                    owner: self.owner,
                    index,
                },
                vec![CoverPattern::from(Literal::Bool(true))],
            )
        }

        fn first_variant(&self) -> Constructor {
            Constructor::new(
                ConstructorTag::Variant {
                    owner: self.owner,
                    index: 0,
                },
                vec![Type::Primitive(PrimitiveType::Bool)],
            )
        }

        fn text(text: &str) -> CoverPattern {
            CoverPattern::from(Literal::String(text.into()))
        }
    }

    #[test]
    fn a_matrix_with_no_rows_is_empty() {
        assert!(PatternMatrix::default().is_empty());
    }

    #[test]
    fn a_pushed_row_makes_it_nonempty() {
        let mut matrix = PatternMatrix::default();
        matrix.push(CoverPattern::wild());
        assert!(!matrix.is_empty());
    }

    #[test]
    fn a_row_of_wildcards_is_a_wild_row() {
        let matrix = MatrixFixture::rows(vec![MatrixFixture::text("a"), CoverPattern::wild()]);
        assert!(matrix.has_wild_row());
    }

    #[test]
    fn a_row_with_a_wildcard_beside_a_value_is_not_a_wild_row() {
        let fixture = MatrixFixture::new();
        let mixed = CoverPattern::constructed(
            ConstructorTag::Variant {
                owner: fixture.owner,
                index: 0,
            },
            vec![
                CoverPattern::wild(),
                CoverPattern::from(Literal::Bool(true)),
            ],
        );
        let pair = Constructor::new(
            ConstructorTag::Variant {
                owner: fixture.owner,
                index: 0,
            },
            vec![
                Type::Primitive(PrimitiveType::Bool),
                Type::Primitive(PrimitiveType::Bool),
            ],
        );
        let specialized = MatrixFixture::rows(vec![mixed]).specialize(&pair);
        assert!(!specialized.has_wild_row());
    }

    #[test]
    fn rows_that_each_test_a_value_have_no_wild_row() {
        let matrix = MatrixFixture::rows(vec![MatrixFixture::text("a"), MatrixFixture::text("b")]);
        assert!(!matrix.has_wild_row());
    }

    #[test]
    fn specializing_keeps_a_row_of_the_same_constructor_with_its_arguments() {
        let fixture = MatrixFixture::new();
        let specialized =
            MatrixFixture::rows(vec![fixture.variant(0)]).specialize(&fixture.first_variant());
        assert!(!specialized.is_empty());
        assert!(!specialized.has_wild_row());
    }

    #[test]
    fn specializing_drops_a_row_of_another_constructor() {
        let fixture = MatrixFixture::new();
        let specialized =
            MatrixFixture::rows(vec![fixture.variant(1)]).specialize(&fixture.first_variant());
        assert!(specialized.is_empty());
    }

    #[test]
    fn specializing_expands_a_wildcard_row_into_wildcards() {
        let fixture = MatrixFixture::new();
        let specialized =
            MatrixFixture::rows(vec![CoverPattern::wild()]).specialize(&fixture.first_variant());
        assert!(specialized.has_wild_row());
    }

    #[test]
    fn specializing_by_a_literal_keeps_the_same_literal_and_wildcards() {
        let literal = CoverPattern::from(Literal::String("a".into()));
        let kept = MatrixFixture::rows(vec![MatrixFixture::text("a")]).specialize_literal(&literal);
        let wild = MatrixFixture::rows(vec![CoverPattern::wild()]).specialize_literal(&literal);
        assert!(!kept.is_empty());
        assert!(!wild.is_empty());
    }

    #[test]
    fn specializing_by_a_literal_drops_another_literal() {
        let literal = CoverPattern::from(Literal::String("a".into()));
        let other =
            MatrixFixture::rows(vec![MatrixFixture::text("b")]).specialize_literal(&literal);
        assert!(other.is_empty());
    }

    #[test]
    fn the_default_rows_are_the_rows_that_start_with_a_wildcard() {
        let fixture = MatrixFixture::new();
        assert!(
            !MatrixFixture::rows(vec![CoverPattern::wild()])
                .default_rows()
                .is_empty()
        );
        assert!(
            MatrixFixture::rows(vec![fixture.variant(0)])
                .default_rows()
                .is_empty()
        );
    }

    #[test]
    fn heads_cover_the_constructors_that_some_row_starts_with() {
        let fixture = MatrixFixture::new();
        let matrix = MatrixFixture::rows(vec![fixture.variant(0)]);
        assert!(matrix.heads_cover(&[fixture.first_variant()]));
    }

    #[test]
    fn heads_do_not_cover_a_constructor_that_no_row_starts_with() {
        let fixture = MatrixFixture::new();
        let matrix = MatrixFixture::rows(vec![fixture.variant(1), CoverPattern::wild()]);
        assert!(!matrix.heads_cover(&[fixture.first_variant()]));
    }

    #[test]
    fn the_default_rows_keep_the_columns_after_the_first() {
        let fixture = MatrixFixture::new();
        let mixed = CoverPattern::constructed(
            ConstructorTag::Variant {
                owner: fixture.owner,
                index: 0,
            },
            vec![
                CoverPattern::wild(),
                CoverPattern::from(Literal::Bool(true)),
            ],
        );
        let pair = Constructor::new(
            ConstructorTag::Variant {
                owner: fixture.owner,
                index: 0,
            },
            vec![
                Type::Primitive(PrimitiveType::Bool),
                Type::Primitive(PrimitiveType::Bool),
            ],
        );
        let default = MatrixFixture::rows(vec![mixed])
            .specialize(&pair)
            .default_rows();
        assert!(!default.is_empty());
        assert!(!default.has_wild_row());
    }
}
