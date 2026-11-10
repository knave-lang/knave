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

    pub(super) fn matches_literal(&self, literal: &Literal) -> bool {
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
