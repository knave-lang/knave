use super::constructors::{Constructor, Tag};
use crate::core::{BindingArena, BindingId, BindingKind, Literal};
use crate::types::{TypedPattern, TypedPatternKind};

#[derive(Clone)]
pub(super) enum CoverPattern {
    Wild,
    Variant {
        owner: BindingId,
        index: usize,
        arguments: Vec<CoverPattern>,
    },
    Struct {
        owner: BindingId,
        arguments: Vec<CoverPattern>,
    },
    Literal(Literal),
}

impl CoverPattern {
    pub(super) fn of(pattern: &TypedPattern, bindings: &BindingArena) -> Self {
        match &pattern.kind {
            TypedPatternKind::Wildcard | TypedPatternKind::Bind(_) => CoverPattern::Wild,
            TypedPatternKind::Variant(binding, arguments) => {
                let BindingKind::Variant(_, info) = bindings.get(*binding).kind() else {
                    unreachable!("a typed variant pattern names a variant binding");
                };
                CoverPattern::Variant {
                    owner: info.owner,
                    index: info.index,
                    arguments: Self::all_of(arguments, bindings),
                }
            }
            TypedPatternKind::Struct(binding, arguments) => CoverPattern::Struct {
                owner: *binding,
                arguments: Self::all_of(arguments, bindings),
            },
            TypedPatternKind::Literal(literal, _) => CoverPattern::Literal(literal.clone()),
        }
    }

    fn all_of(patterns: &[TypedPattern], bindings: &BindingArena) -> Vec<CoverPattern> {
        patterns
            .iter()
            .map(|pattern| Self::of(pattern, bindings))
            .collect()
    }

    pub(super) fn arguments(&self) -> &[CoverPattern] {
        match self {
            CoverPattern::Variant { arguments, .. } | CoverPattern::Struct { arguments, .. } => {
                arguments
            }
            CoverPattern::Wild | CoverPattern::Literal(_) => &[],
        }
    }

    fn arguments_for(&self, constructor: &Constructor) -> Option<Vec<CoverPattern>> {
        match self {
            CoverPattern::Wild => Some(vec![CoverPattern::Wild; constructor.arguments.len()]),
            CoverPattern::Variant {
                owner,
                index,
                arguments,
            } if constructor.tag
                == (Tag::Variant {
                    owner: *owner,
                    index: *index,
                }) =>
            {
                Some(arguments.clone())
            }
            CoverPattern::Struct { owner, arguments } if constructor.tag == Tag::Struct(*owner) => {
                Some(arguments.clone())
            }
            CoverPattern::Literal(Literal::Bool(value)) if constructor.tag == Tag::Bool(*value) => {
                Some(Vec::new())
            }
            _ => None,
        }
    }

    fn matches_literal(&self, literal: &Literal) -> bool {
        match (self, literal) {
            (CoverPattern::Wild, _) => true,
            (CoverPattern::Literal(Literal::Number(left)), Literal::Number(right)) => {
                left.value() == right.value()
            }
            (CoverPattern::Literal(Literal::String(left)), Literal::String(right)) => left == right,
            (CoverPattern::Literal(Literal::Bool(left)), Literal::Bool(right)) => left == right,
            _ => false,
        }
    }
}

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
        self.keep_rows(|first| first.arguments_for(constructor))
    }

    pub(super) fn specialize_literal(&self, literal: &Literal) -> Self {
        self.keep_rows(|first| first.matches_literal(literal).then(Vec::new))
    }

    pub(super) fn default_rows(&self) -> Self {
        self.keep_rows(|first| matches!(first, CoverPattern::Wild).then(Vec::new))
    }

    pub(super) fn split_wild_rows(&self) -> (Self, Self) {
        let (wild, concrete): (Vec<_>, Vec<_>) = self
            .rows
            .iter()
            .partition(|row| matches!(row.first(), Some(CoverPattern::Wild)));
        let default = wild.into_iter().map(|row| row[1..].to_vec()).collect();
        let concrete = concrete.into_iter().cloned().collect();
        (Self::of_rows(default), Self::of_rows(concrete))
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
